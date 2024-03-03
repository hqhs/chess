use std::sync::Arc;

use ui::EguiRenderer;
use winit::{
    event::{Event, KeyEvent, WindowEvent},
    event_loop::EventLoopWindowTarget,
    keyboard::PhysicalKey,
    window::Window,
};

use discipline::{
    camera::{self, Camera as _},
    glam::{self, Mat4, Quat, Vec3},
    setup,
    wgpu::{self, util::DeviceExt},
};

mod cube;
mod depth;
mod grid;
mod ui;

use cube::Cube;
use depth::Depth;
use grid::Grid;

struct Game {
    iad: discipline::InstanceAdapterDevice,
    egui_renderer: ui::EguiRenderer,

    background_color: [f32; 4],
    camera: Camera,
    cube: Cube,
    debug_grid: Grid,
    depth: Depth,
}

#[derive(Debug)]
struct Camera {
    view: Mat4,
    flying: camera::Flying,
    // conrtols
    speed: f32,

    // settings
    aspect_ratio: f32,
    projection: Mat4,
    perspective: camera::Projection,
}

impl Camera {
    fn new(aspect_ratio: f32) -> Self {
        let perspective = camera::Projection::InfinitePerspective {
            vfov: 45.0,
            near: 0.1,
        };
        let projection = perspective.compute_matrix(aspect_ratio);

        let yaw = -0.001;
        let pitch = -1.3;
        let roll = -0.3;
        let translation = -Vec3::new(0.0, 0.0, 6.0);
        let scale = Vec3::ONE;

        let flying = camera::Flying {
            yaw,
            pitch,
            roll,
            translation,
            scale,
        };

        // let view = Self::view_from_eye_and_point();
        // let view = Self::isometric_view();
        let speed = 0.1;
        let view = projection * flying.view();
        Self {
            view,
            flying,
            speed,
            aspect_ratio,
            projection,
            perspective,
        }
    }

    fn update_projection(&mut self, aspect_ratio: f32) {
        self.aspect_ratio = aspect_ratio;
        self.projection = self.perspective.compute_matrix(self.aspect_ratio);
        self.update_view();
    }

    fn update_view(&mut self) {
        self.view = self.projection * self.flying.view();
    }

    fn grid_input(&self, scale: f32) -> grid::UniformInput {
        grid::UniformInput {
            projection: self.projection,
            view: self.flying.view(),
            scale: Mat4::from_scale(Vec3::ONE * 80.0),
        }
    }

    fn view_from_eye_and_point() -> Mat4 {
        camera::Pointed {
            eye: Vec3::new(1.5, -5.0, 3.0),
            looking_at: Vec3::ZERO,
            top: Vec3::Z,
        }
        .view()
    }

    fn isometric_view() -> Mat4 {
        camera::Isometric {
            looking_at: Vec3::ZERO,
            distance: 5.0,
        }
        .view()
    }
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

    log::info!("preferred surface texture format: {:#?}", preferred_format);

    let size_vec = glam::UVec2::new(size.width, size.height);
    setup::configure_surface(
        &surface,
        &iad.device,
        preferred_format,
        size_vec,
        wgpu::PresentMode::Fifo,
    );

    // let ui = UI::new(window.clone(), &iad.device, preferred_format);
    let egui_renderer = EguiRenderer::new(&iad.device, preferred_format, None, 1, &window);
    let background_color = [0.1, 0.2, 0.3, 1.0];
    let aspect_ratio = size_vec.x as f32 / size_vec.y as f32;
    let camera = Camera::new(aspect_ratio);
    let depth = depth::Depth::new(&iad.device, size_vec, "Depth texture label");
    let cube = Cube::new(preferred_format, &iad.device, &iad.queue, camera.view);
    let grid_input = camera.grid_input(80.0);
    let debug_grid = Grid::new(preferred_format, &iad.device, &iad.queue, &grid_input);
    let mut game = Game {
        iad,
        background_color,
        egui_renderer,
        camera,
        cube,
        debug_grid,
        depth,
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
            let size = glam::UVec2::new(new_size.width, new_size.height);
            setup::configure_surface(
                surface,
                &game.iad.device,
                preferred_format,
                size,
                wgpu::PresentMode::Fifo,
            );
            game.depth = Depth::new(&game.iad.device, size, "Depth texture label");
            let aspect_ratio = new_size.width as f32 / new_size.height as f32;
            game.camera.update_projection(aspect_ratio);

            let grid_input = game.camera.grid_input(80.0);
            game.cube.update_camera(&game.iad.queue, game.camera.view);
            game.debug_grid.write_uniform(&game.iad.queue, &grid_input);
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
        Event::WindowEvent {
            event: WindowEvent::KeyboardInput { event, .. },
            ..
        } => {
            let result = maybe_move_camera(&mut game.camera, event);
            if matches!(result, EventResult::Ignored) {
                return Ok(());
            }

            game.camera.update_view();

            let grid_input = game.camera.grid_input(80.0);
            game.cube.update_camera(&game.iad.queue, game.camera.view);
            game.debug_grid.write_uniform(&game.iad.queue, &grid_input);

            window.request_redraw();
        }
        _ => {}
    };
    Ok(())
}

enum EventResult {
    Ignored,
    Redraw,
}

fn maybe_move_camera(camera: &mut Camera, key: KeyEvent) -> EventResult {
    let KeyEvent {
        state,
        physical_key,
        ..
    } = key;

    if matches!(state, winit::event::ElementState::Released) {
        return EventResult::Ignored;
    }

    use winit::keyboard::{KeyCode, PhysicalKey};

    let code = match physical_key {
        PhysicalKey::Code(code) => code,
        PhysicalKey::Unidentified(_) => {
            return EventResult::Ignored;
        }
    };
    match code {
        KeyCode::ArrowUp => {
            camera.flying.translation.y += camera.speed;
        }
        KeyCode::ArrowDown => {
            camera.flying.translation.y -= camera.speed;
        }
        KeyCode::ArrowLeft => {
            camera.flying.translation.x += camera.speed;
        }
        KeyCode::ArrowRight => {
            camera.flying.translation.x -= camera.speed;
        }
        KeyCode::Space => {
            camera.flying.translation.z += camera.speed;
        }
        KeyCode::ShiftRight => {
            camera.flying.translation.z -= camera.speed;
        }
        _ => {
            return EventResult::Ignored;
        }
    };
    EventResult::Redraw
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

    let depth_ops = Some(wgpu::Operations {
        load: wgpu::LoadOp::Clear(1.0),
        store: wgpu::StoreOp::Store,
    });

    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: None,
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(bg),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: &game.depth.view,
            depth_ops,
            stencil_ops: None,
        }),
        timestamp_writes: None,
        occlusion_query_set: None,
    });

    // NOTE: grid should be rendered last
    // TODO: explain why
    game.cube.render(&mut rpass);
    game.debug_grid.render(&mut rpass);

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
            .show(cx, |mut ui| {
                // let available_width = ui.available_width();
                if ui.button("Reset camera").clicked() {
                    log::info!("resetting camera");
                    let aspect_ratio = 1.0;
                    game.camera = Camera::new(aspect_ratio);

                    let grid_input = game.camera.grid_input(80.0);
                    game.cube.update_camera(&game.iad.queue, game.camera.view);
                    game.debug_grid.write_uniform(&game.iad.queue, &grid_input);
                }

                ui.horizontal(|ui| {
                    ui.label("Background color: ");
                    if ui
                        .color_edit_button_rgba_unmultiplied(&mut game.background_color)
                        .changed()
                    {}
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
        window,
        view,
        screen_descriptor,
        egui_lambda,
    );
}

fn ced(label: Option<&'static str>) -> wgpu::CommandEncoderDescriptor {
    wgpu::CommandEncoderDescriptor { label }
}
