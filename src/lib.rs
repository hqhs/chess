use std::sync::Arc;

use ui::EguiRenderer;
use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoopWindowTarget,
    window::Window,
};

use discipline::{glam, setup, wgpu};

mod ui;

pub fn log_iad_info(iad: &discipline::InstanceAdapterDevice) {
    // let info = iad.adapter.get_info();
    // let limits = iad.adapter.limits();
    // let features = iad.adapter.features();
    // log::debug!("Chosen adapter: {:#?}", adapter.i)
}

struct Game {
    iad: discipline::InstanceAdapterDevice,
    // ui: UI,
    egui_renderer: ui::EguiRenderer,

    background_color: [f32; 4],
}

pub async fn run() -> anyhow::Result<()> {
    let size = winit::dpi::LogicalSize::new(800.0, 600.0);
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
        glam::UVec2::new(size.width, size.height),
        wgpu::PresentMode::Fifo,
    );

    // let ui = UI::new(window.clone(), &iad.device, preferred_format);
    let egui_renderer = EguiRenderer::new(&iad.device, preferred_format, None, 1, &window);
    let background_color = [0.1, 0.2, 0.3, 1.0];
    let mut game = Game {
        iad,
        background_color,
        egui_renderer,
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
                glam::UVec2::new(new_size.width, new_size.height),
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
        egui::Window::new("Settings")
            .resizable(true)
            .vscroll(true)
            .default_open(true)
            .show(&cx, |mut ui| {
                ui.label("Window!");
                if ui.button("Click me!").clicked() {
                    log::info!("button clicked");
                }

                ui.label("Background color: ");
                if ui
                    .color_edit_button_rgba_unmultiplied(&mut game.background_color)
                    .changed()
                {
                    log::info!("background color changed");
                }

                // proto_scene.egui(ui);
            });
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

fn populate_ui_elements(cx: &egui::Context, game: &mut Game) {}

fn ced(label: Option<&'static str>) -> wgpu::CommandEncoderDescriptor {
    wgpu::CommandEncoderDescriptor { label }
}
