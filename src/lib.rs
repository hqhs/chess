use std::mem;
use std::sync::Arc;

use ui::EguiRenderer;
use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoopWindowTarget,
    window::Window,
};

use discipline::{
    nalgebra as na, setup,
    wgpu::{self, util::DeviceExt},
};

mod ui;

pub fn log_iad_info(iad: &discipline::InstanceAdapterDevice) {
    // let info = iad.adapter.get_info();
    // let limits = iad.adapter.limits();
    // let features = iad.adapter.features();
    // log::debug!("Chosen adapter: {:#?}", adapter.i)
}

struct Game {
    iad: discipline::InstanceAdapterDevice,
    egui_renderer: ui::EguiRenderer,

    background_color: [f32; 4],
    cube: Cube,
}

pub async fn run() -> anyhow::Result<()> {
    let size = winit::dpi::LogicalSize::new(800.0, 800.0);
    let event_loop = winit::event_loop::EventLoop::new()?;
    let window = winit::window::WindowBuilder::new()
        .with_resizable(true)
        .with_inner_size(size)
        .build(&event_loop)?;

    let window = Arc::new(window);
    let size = window.inner_size();

    let iad = setup::create_default_iad().await?;
    let surface = iad.instance.create_surface(window.clone())?;
    let caps = surface.get_capabilities(&iad.adapter);
    let preferred_format = caps.formats[0];

    setup::configure_surface(
        &surface,
        &iad.device,
        preferred_format,
        na::Vector2::new(size.width, size.height),
        wgpu::PresentMode::Fifo,
    );

    // let ui = UI::new(window.clone(), &iad.device, preferred_format);
    let egui_renderer = EguiRenderer::new(&iad.device, preferred_format, None, 1, &window);
    let background_color = [0.1, 0.2, 0.3, 1.0];
    let cube = Cube::new(preferred_format, &iad.device, &iad.queue);
    let mut game = Game {
        iad,
        background_color,
        egui_renderer,
        cube,
    };

    let event_lambda = move |event, event_loop_window_target: &EventLoopWindowTarget<()>| {
        process_event(
            event,
            event_loop_window_target,
            &mut game,
            &surface,
            window.clone(),
        );
    };

    event_loop.run(event_lambda)?;

    Ok(())
}

struct Frame {
    encoder: wgpu::CommandEncoder,
    surface_texture: wgpu::SurfaceTexture,
    view: wgpu::TextureView,
}

fn process_event(
    event: Event<()>,
    event_loop_window_target: &winit::event_loop::EventLoopWindowTarget<()>,
    game: &mut Game,
    surface: &wgpu::Surface,
    window: Arc<Window>,
) -> anyhow::Result<()> {
    if let Event::WindowEvent { ref event, .. } = event {
        let response = game.egui_renderer.handle_input(&window, event);
        if response.consumed {
            return Ok(());
        }
        if response.repaint {
            window.request_redraw();
        }
    }
    match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => event_loop_window_target.exit(),
        Event::WindowEvent {
            event: WindowEvent::Resized(new_size),
            ..
        } => {
            let caps = surface.get_capabilities(&game.iad.adapter);
            let preferred_format = caps.formats[0];
            setup::configure_surface(
                surface,
                &game.iad.device,
                preferred_format,
                na::Vector2::new(new_size.width, new_size.height),
                wgpu::PresentMode::Fifo,
            );
            // TODO: how to pass resize event to egui?
            window.request_redraw();
        }
        Event::WindowEvent {
            event: WindowEvent::ScaleFactorChanged { .. },
            ..
        } => {
            log::error!("Scale factor changed");
        }
        Event::WindowEvent {
            event: WindowEvent::RedrawRequested,
            ..
        } => {
            let surface_texture = match surface.get_current_texture() {
                Ok(frame) => frame,
                Err(e) => {
                    log::error!("failed to get surface texture to draw next frame: {}", e);
                    return Ok(());
                }
            };
            let texture_descriptor = wgpu::TextureViewDescriptor::default();
            let view = surface_texture.texture.create_view(&texture_descriptor);
            let encoder = game.iad.device.create_command_encoder(&ced(None));
            let mut frame = Frame {
                encoder,
                surface_texture,
                view,
            };

            redraw(game, &mut frame);
            redraw_ui(&window, game, &mut frame);

            let Frame {
                encoder,
                surface_texture,
                ..
            } = frame;
            game.iad.queue.submit(Some(encoder.finish()));
            surface_texture.present();
        }
        _ => {}
    };
    Ok(())
}

fn redraw(game: &mut Game, frame: &mut Frame) {
    let iad = &game.iad;
    let view = &mut frame.view;
    let encoder = &mut frame.encoder;
    // NOTE: probably would be a function
    let bg = game.background_color;
    let bg = wgpu::Color {
        r: bg[0].into(),
        g: bg[1].into(),
        b: bg[2].into(),
        a: bg[3].into(),
    };

    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: None,
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(bg),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
    });

    game.cube.render(&mut rpass);

    // next thing:
    // set camera parameters
    // show camera parameters with egui
    // allow camera movement
    // upload a cube vertex
    // render it

    drop(rpass);
}

fn redraw_ui(window: &Window, game: &mut Game, frame: &mut Frame) {
    let view = &frame.view;
    let size = window.inner_size();
    let pixels_per_point = window.scale_factor() as f32;
    let screen_descriptor = egui_wgpu::ScreenDescriptor {
        size_in_pixels: [size.width, size.height],
        pixels_per_point,
    };
    let egui_lambda = |cx: &egui::Context| {
        egui::Window::new("Debug")
            .resizable(true)
            .vscroll(true)
            .default_open(true)
            .show(&cx, |mut ui| {
                ui.label("Window!");
                if ui.button("Click me!").clicked() {
                    log::info!("button clicked");
                }

                ui.horizontal(|ui| {
                    ui.label("Background color: ");
                    if ui
                        .color_edit_button_rgba_unmultiplied(&mut game.background_color)
                        .changed()
                    {
                        log::info!("background color changed");
                    }
                })
            });

        // let menu_frame = egui::Frame::none()
        //     .fill(egui::Color32::DARK_GRAY)
        //     .inner_margin(egui::Margin::same(10.));

        // egui::Window::new("Menu")
        //     .frame(menu_frame)
        //     .anchor(egui::Align2::CENTER_CENTER, egui::Vec2 { x: 0., y: 0. })
        //     .resizable(false)
        //     .movable(false)
        //     .collapsible(false)
        //     // .fixed_pos(true)
        //     .show(&cx, |mut ui| {
        //         if ui.button("Settings").clicked() {
        //             log::info!("button clicked");
        //         }
        //         if ui.button("Quit to desktop").clicked() {
        //             log::info!("button clicked");
        //         }
        //     });
    };
    game.egui_renderer.draw(
        &game.iad.device,
        &game.iad.queue,
        &mut frame.encoder,
        &window,
        &view,
        screen_descriptor,
        egui_lambda,
    );
}

fn ced(label: Option<&'static str>) -> wgpu::CommandEncoderDescriptor {
    wgpu::CommandEncoderDescriptor { label }
}

fn vertex(pos: [f32; 3]) -> [f32; 4] {
    [pos[0], pos[1], pos[2], 1.0]
}

fn create_mesh() -> ([[f32; 4]; 24], [u32; 36]) {
    let vertex_positions = [
        // far side (0.0, 0.0, 1.0)
        vertex([-1.0, -1.0, 1.0]),
        vertex([1.0, -1.0, 1.0]),
        vertex([1.0, 1.0, 1.0]),
        vertex([-1.0, 1.0, 1.0]),
        // near side (0.0, 0.0, -1.0)
        vertex([-1.0, 1.0, -1.0]),
        vertex([1.0, 1.0, -1.0]),
        vertex([1.0, -1.0, -1.0]),
        vertex([-1.0, -1.0, -1.0]),
        // right side (1.0, 0.0, 0.0)
        vertex([1.0, -1.0, -1.0]),
        vertex([1.0, 1.0, -1.0]),
        vertex([1.0, 1.0, 1.0]),
        vertex([1.0, -1.0, 1.0]),
        // left side (-1.0, 0.0, 0.0)
        vertex([-1.0, -1.0, 1.0]),
        vertex([-1.0, 1.0, 1.0]),
        vertex([-1.0, 1.0, -1.0]),
        vertex([-1.0, -1.0, -1.0]),
        // top (0.0, 1.0, 0.0)
        vertex([1.0, 1.0, -1.0]),
        vertex([-1.0, 1.0, -1.0]),
        vertex([-1.0, 1.0, 1.0]),
        vertex([1.0, 1.0, 1.0]),
        // bottom (0.0, -1.0, 0.0)
        vertex([1.0, -1.0, 1.0]),
        vertex([-1.0, -1.0, 1.0]),
        vertex([-1.0, -1.0, -1.0]),
        vertex([1.0, -1.0, -1.0]),
    ];

    let index_data = [
        0, 1, 2, 2, 3, 0, // far
        4, 5, 6, 6, 7, 4, // bottom
        8, 9, 10, 10, 11, 8, // right
        12, 13, 14, 14, 15, 12, // left
        16, 17, 18, 18, 19, 16, // top
        20, 21, 22, 22, 23, 20, // bottom
    ];

    (vertex_positions, index_data)
}

struct Camera {
    // projection: CameraProjection,
    view: glam::Mat4, // na::Matrix4<f32>,
}

impl Camera {
    fn new() -> Self {
        let aspect_ration = 1.0;
        let projection =
            glam::Mat4::perspective_rh(std::f32::consts::FRAC_PI_4, aspect_ration, 1.0, 10.0);
        let view = glam::Mat4::look_at_rh(
            glam::Vec3::new(1.5f32, -5.0, 3.0),
            glam::Vec3::ZERO,
            glam::Vec3::Z,
        );
        let view = projection * view;

        log::info!("camera view: {:#?}", view);

        Self { view }
    }
}

/// Describes how the world should be projected into the camera.
#[derive(Debug, Copy, Clone)]
enum CameraProjection {
    Orthographic {
        /// Size assumes the location is at the center of the camera area.
        size: na::Vector3<f32>,
    },
    Perspective {
        /// Vertical field of view in degrees.
        vfov: f32,
        /// Near plane distance. All projection uses a infinite far plane.
        near: f32,
    },
    Raw(na::Matrix4<f32>),
}

struct Cube {
    render_pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,

    camera: Camera,
}

impl Cube {
    fn new(format: wgpu::TextureFormat, device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let (vertices, indexes) = create_mesh();
        let index_count: u32 = indexes.len().try_into().unwrap();
        let camera = Camera::new();

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Cube vertex buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Cube index buffer"),
            contents: bytemuck::cast_slice(&indexes),
            usage: wgpu::BufferUsages::INDEX,
        });

        let uniform_camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera.view]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // wgpu::BindGroupLayoutEntry {
                //     binding: 1,
                //     visibility: wgpu::ShaderStages::FRAGMENT,
                //     ty: wgpu::BindingType::Texture {
                //         multisampled: false,
                //         sample_type: wgpu::TextureSampleType::Uint,
                //         view_dimension: wgpu::TextureViewDimension::D2,
                //     },
                //     count: None,
                // },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_camera_buffer.as_entire_binding(),
            }],
            label: None,
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let vertex_size = mem::size_of::<na::Vector3<f32>>();
        let vertex_buffers = [wgpu::VertexBufferLayout {
            array_stride: vertex_size as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x4,
                offset: 0,
                shader_location: 0,
            }],
        }];

        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Cube render pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &vertex_buffers,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let cube = Cube {
            render_pipeline,
            index_buffer,
            vertex_buffer,
            bind_group,
            index_count,
            // TODO: shouldn't be here
            camera,
        };

        cube
    }

    fn render<'rpass>(&'rpass self, rpass: &mut wgpu::RenderPass<'rpass>) {
        rpass.set_pipeline(&self.render_pipeline);
        rpass.set_bind_group(0, &self.bind_group, &[]);
        rpass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        rpass.draw_indexed(0..self.index_count, 0, 0..1);
    }
}
