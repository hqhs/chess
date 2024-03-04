use std::{borrow::Cow, mem};

use bytemuck::{Pod, Zeroable};
use discipline::{
  glam::{self, Mat4, Vec3},
  wgpu::{self, util::DeviceExt},
};

use crate::depth::depth_stencil_for_pipeline;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
  _pos: [f32; 4],
  _tex_coord: [f32; 2],
}

fn vertex(
  pos: [i8; 3],
  tc: [i8; 2],
) -> Vertex {
  Vertex {
    _pos: [pos[0] as f32, pos[1] as f32, pos[2] as f32, 1.0],
    _tex_coord: [tc[0] as f32, tc[1] as f32],
  }
}

fn create_vertices() -> (Vec<Vertex>, Vec<u16>) {
  let vertex_data = [
    // top (0, 0, 1)
    vertex([-1, -1, 1], [0, 0]),
    vertex([1, -1, 1], [1, 0]),
    vertex([1, 1, 1], [1, 1]),
    vertex([-1, 1, 1], [0, 1]),
    // bottom (0, 0, -1)
    vertex([-1, 1, -1], [1, 0]),
    vertex([1, 1, -1], [0, 0]),
    vertex([1, -1, -1], [0, 1]),
    vertex([-1, -1, -1], [1, 1]),
    // right (1, 0, 0)
    vertex([1, -1, -1], [0, 0]),
    vertex([1, 1, -1], [1, 0]),
    vertex([1, 1, 1], [1, 1]),
    vertex([1, -1, 1], [0, 1]),
    // left (-1, 0, 0)
    vertex([-1, -1, 1], [1, 0]),
    vertex([-1, 1, 1], [0, 0]),
    vertex([-1, 1, -1], [0, 1]),
    vertex([-1, -1, -1], [1, 1]),
    // front (0, 1, 0)
    vertex([1, 1, -1], [1, 0]),
    vertex([-1, 1, -1], [0, 0]),
    vertex([-1, 1, 1], [0, 1]),
    vertex([1, 1, 1], [1, 1]),
    // back (0, -1, 0)
    vertex([1, -1, 1], [0, 0]),
    vertex([-1, -1, 1], [1, 0]),
    vertex([-1, -1, -1], [1, 1]),
    vertex([1, -1, -1], [0, 1]),
  ];

  let index_data: &[u16] = &[
    0, 1, 2, 2, 3, 0, // top
    4, 5, 6, 6, 7, 4, // bottom
    8, 9, 10, 10, 11, 8, // right
    12, 13, 14, 14, 15, 12, // left
    16, 17, 18, 18, 19, 16, // front
    20, 21, 22, 22, 23, 20, // back
  ];

  (vertex_data.to_vec(), index_data.to_vec())
}

fn create_texels(size: usize) -> Vec<u8> {
  (0..size * size)
    .map(|id| {
      // get high five for recognizing this ;)
      let cx = 3.0 * (id % size) as f32 / (size - 1) as f32 - 2.0;
      let cy = 2.0 * (id / size) as f32 / (size - 1) as f32 - 1.0;
      let (mut x, mut y, mut count) = (cx, cy, 0);
      while count < 0xFF && x * x + y * y < 4.0 {
        let old_x = x;
        x = x * x - y * y + cx;
        y = 2.0 * old_x * y + cy;
        count += 1;
      }
      count
    })
    .collect()
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct ShaderUniformInput {
  camera_view: Mat4,
  scale: Mat4,
  // NOTE: according to wgsl memory layout
  // https://sotrh.github.io/learn-wgpu/showcase/alignment/#alignment-of-uniform-and-storage-buffers
  // memory layout should be aligned to 16
}

pub struct Cube {
  vertex_buf: wgpu::Buffer,
  index_buf: wgpu::Buffer,
  index_count: usize,
  bind_group: wgpu::BindGroup,
  uniform_buf: wgpu::Buffer,
  pipeline: wgpu::RenderPipeline,
}

const SCALE: f32 = 0.10;

impl Cube {
  pub fn update_camera(
    &mut self,
    queue: &wgpu::Queue,
    camera_view: Mat4,
  ) {
    let _padding = Vec3::ZERO;
    let scale = Vec3::ONE * SCALE;
    let scale = Mat4::from_scale(scale);
    let uniform_input = ShaderUniformInput { camera_view, scale };
    queue.write_buffer(
      &self.uniform_buf,
      0,
      &bytemuck::bytes_of(&uniform_input),
    );
  }

  pub fn new(
    format: wgpu::TextureFormat,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    camera_view: Mat4,
  ) -> Self {
    // Create the vertex and index buffers
    let vertex_size = mem::size_of::<Vertex>();
    let (vertex_data, index_data) = create_vertices();

    let vertex_buf =
      device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Vertex Buffer"),
        contents: bytemuck::cast_slice(&vertex_data),
        usage: wgpu::BufferUsages::VERTEX,
      });

    let index_buf =
      device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Index Buffer"),
        contents: bytemuck::cast_slice(&index_data),
        usage: wgpu::BufferUsages::INDEX,
      });

    // Create pipeline layout
    let bind_group_layout =
      device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
          wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
              ty: wgpu::BufferBindingType::Uniform,
              has_dynamic_offset: false,
              min_binding_size: None, // wgpu::BufferSize::new(128), // FIXME: how to properly calculate size?..
            },
            count: None,
          },
          wgpu::BindGroupLayoutEntry {
            binding: 1,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
              multisampled: false,
              sample_type: wgpu::TextureSampleType::Uint,
              view_dimension: wgpu::TextureViewDimension::D2,
            },
            count: None,
          },
        ],
      });
    let pipeline_layout =
      device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
      });

    // Create the texture
    let size = 256u32;
    let texels = create_texels(size as usize);
    let texture_extent =
      wgpu::Extent3d { width: size, height: size, depth_or_array_layers: 1 };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
      label: None,
      size: texture_extent,
      mip_level_count: 1,
      sample_count: 1,
      dimension: wgpu::TextureDimension::D2,
      format: wgpu::TextureFormat::R8Uint,
      usage: wgpu::TextureUsages::TEXTURE_BINDING
        | wgpu::TextureUsages::COPY_DST,
      view_formats: &[],
    });
    let texture_view =
      texture.create_view(&wgpu::TextureViewDescriptor::default());
    queue.write_texture(
      texture.as_image_copy(),
      &texels,
      wgpu::ImageDataLayout {
        offset: 0,
        bytes_per_row: Some(size),
        rows_per_image: None,
      },
      texture_extent,
    );

    // Create other resources
    let _padding = Vec3::ZERO;
    let scale = Vec3::ONE * SCALE;
    let scale = Mat4::from_scale(scale);
    let uniform_input = ShaderUniformInput { camera_view, scale };
    let uniform_buf =
      device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Uniform Buffer"),
        contents: &bytemuck::bytes_of(&uniform_input),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
      });

    // Create bind group
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
      layout: &bind_group_layout,
      entries: &[
        wgpu::BindGroupEntry {
          binding: 0,
          resource: uniform_buf.as_entire_binding(),
        },
        wgpu::BindGroupEntry {
          binding: 1,
          resource: wgpu::BindingResource::TextureView(&texture_view),
        },
      ],
      label: None,
    });

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
      label: None,
      source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!(
        "cube.wgsl"
      ))),
    });

    let attributes = &wgpu::vertex_attr_array![
        0 => Float32x4,
        1 => Float32x2,
    ];

    let vertex_buffers = [wgpu::VertexBufferLayout {
      array_stride: vertex_size as wgpu::BufferAddress,
      step_mode: wgpu::VertexStepMode::Vertex,
      attributes,
    }];

    // let mut color_target_state: wgpu::ColorTargetState = format.into();
    // color_target_state.blend = Some(wgpu::BlendState::ALPHA_BLENDING);

    // TODO: pass from the arguments maybe?
    let depth_stencil = depth_stencil_for_pipeline();
    let pipeline =
      device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
          module: &shader,
          entry_point: "vs_main",
          buffers: &vertex_buffers,
        },
        fragment: Some(wgpu::FragmentState {
          module: &shader,
          entry_point: "fs_main",
          targets: &[Some(format.into())],
        }),
        primitive: wgpu::PrimitiveState {
          cull_mode: Some(wgpu::Face::Back),
          ..Default::default()
        },
        depth_stencil,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
      });

    // Done
    Self {
      vertex_buf,
      index_buf,
      index_count: index_data.len(),
      bind_group,
      uniform_buf,
      pipeline,
    }
  }

  pub fn render<'rpass>(
    &'rpass mut self,
    rpass: &mut wgpu::RenderPass<'rpass>,
  ) {
    rpass.push_debug_group("Cube rendering");
    rpass.set_pipeline(&self.pipeline);
    rpass.set_bind_group(0, &self.bind_group, &[]);
    rpass.set_index_buffer(self.index_buf.slice(..), wgpu::IndexFormat::Uint16);
    rpass.set_vertex_buffer(0, self.vertex_buf.slice(..));
    rpass.draw_indexed(0..self.index_count as u32, 0, 0..1);

    rpass.pop_debug_group();
  }
}
