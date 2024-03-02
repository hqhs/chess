/// Infinite debug grid,
/// Algorithm descriptionhttps://asliceofrendering.com/scene%20helper/2020/01/05/InfiniteGrid/
use std::{borrow::Cow, f32::consts, mem};

use bytemuck::{Pod, Zeroable};
use discipline::{
    glam::{self, Mat4},
    wgpu::{self, util::DeviceExt},
};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    _pos: [f32; 4],
}

fn vertex(pos: [f32; 3]) -> Vertex {
    Vertex {
        _pos: [pos[0], pos[1], pos[2], 1.0],
    }
}

fn create_vertices() -> Vec<Vertex> {
    let vertex_data = [
        vertex([1.0, 1.0, 0.0]),   // right
        vertex([-1.0, -1.0, 0.0]), // left
        vertex([1.0, -1.0, 0.0]),  // middle below
        //
        vertex([1.0, 1.0, 0.0]),   // right
        vertex([-1.0, 1.0, 0.0]),  // middle up
        vertex([-1.0, -1.0, 0.0]), // left
    ];
    vertex_data.to_vec()
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct ShaderUniformInput {
    camera_view: Mat4,
}

pub struct Grid {
    vertex_buf: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    uniform_buf: wgpu::Buffer,
    pipeline: wgpu::RenderPipeline,
    num_vertices: u32,
}

impl Grid {
    pub fn update_camera(&mut self, queue: &wgpu::Queue, camera_view: Mat4) {
        let uniform_input = ShaderUniformInput { camera_view };
        queue.write_buffer(&self.uniform_buf, 0, &bytemuck::bytes_of(&uniform_input));
    }

    pub fn new(
        format: wgpu::TextureFormat,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        camera_view: Mat4,
    ) -> Self {
        let vertex_data = create_vertices();
        let num_vertices = vertex_data.len() as u32;

        let vertex_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex buffer"),
            contents: bytemuck::cast_slice(&vertex_data),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let vertex_size = mem::size_of::<Vertex>();
        let attributes = &wgpu::vertex_attr_array![
            0 => Float32x4,
        ];
        let vertex_buffers = [wgpu::VertexBufferLayout {
            array_stride: vertex_size as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes,
        }];

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let uniform_input = ShaderUniformInput { camera_view };
        let uniform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Debug grid uniform buffer"),
            contents: &bytemuck::bytes_of(&uniform_input),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            }],
            label: None,
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Debug grid shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("grid.wgsl"))),
        });

        let mut color_target_state: wgpu::ColorTargetState = format.into();
        color_target_state.blend = Some(wgpu::BlendState::ALPHA_BLENDING);

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Debug grid render pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &vertex_buffers,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                // TODO: next thing; opacity targets should be set here
                targets: &[Some(color_target_state)],
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        Self {
            vertex_buf,
            bind_group,
            uniform_buf,
            pipeline,
            num_vertices,
        }
    }

    pub fn render<'rpass>(&'rpass mut self, rpass: &mut wgpu::RenderPass<'rpass>) {
        rpass.push_debug_group("Debug grid rendering");
        rpass.set_pipeline(&self.pipeline);
        rpass.set_bind_group(0, &self.bind_group, &[]);
        rpass.set_vertex_buffer(0, self.vertex_buf.slice(..));

        let instances = 1;
        rpass.draw(0..self.num_vertices, 0..instances);

        rpass.pop_debug_group();
    }
}
