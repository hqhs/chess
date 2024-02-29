use std::sync::Arc;

use ui::EguiRenderer;
use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoopWindowTarget,
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

struct UI {
    renderer: egui_wgpu::Renderer,
    screen_descriptor: egui_wgpu::ScreenDescriptor,
    egui_cx: egui::Context,
    platform: egui_winit::State,
    clipped_meshes: Vec<egui::ClippedPrimitive>,
}

impl UI {
    fn new(
        window: Arc<winit::window::Window>,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
    ) -> Self {
        let msaa_samples = 1u32;
        let output_depth_format: Option<wgpu::TextureFormat> = None;
        let renderer = egui_wgpu::Renderer::new(device, format, output_depth_format, msaa_samples);
        let size = window.inner_size();
        let pixels_per_point = window.scale_factor() as f32;
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [size.width, size.height],
            pixels_per_point,
        };
        let egui_cx = egui::Context::default();
        let max_texture_side = None;
        let platform = egui_winit::State::new(
            egui_cx.clone(),
            egui::ViewportId::default(),
            &window,
            Some(window.scale_factor() as f32),
            max_texture_side,
        );
        let clipped_meshes = vec![];
        Self {
            renderer,
            screen_descriptor,
            egui_cx,
            platform,
            clipped_meshes,
        }
    }
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

fn process_event(
    event: Event<()>,
    event_loop_window_target: &winit::event_loop::EventLoopWindowTarget<()>,
    game: &mut Game,
    surface: &wgpu::Surface,
    window: Arc<winit::window::Window>,
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
            let frame = match surface.get_current_texture() {
                Ok(frame) => frame,
                Err(e) => {
                    log::error!("failed to get surface texture to draw next frame: {}", e);
                    return Ok(());
                }
            };
            redraw(window, game, &frame);
            frame.present();
        }
        _ => {}
    };
    Ok(())
}

fn redraw(window: Arc<winit::window::Window>, game: &mut Game, frame: &wgpu::SurfaceTexture) {
    let texture_descriptor = wgpu::TextureViewDescriptor::default();
    let view = frame.texture.create_view(&texture_descriptor);
    let iad = &game.iad;
    let mut encoder = iad.device.create_command_encoder(&ced(None));
    // next thing: make color configurable with egui
    let background_color = wgpu::Color {
        r: 0.1,
        g: 0.2,
        b: 0.3,
        a: 1.0,
    };

    let size = window.inner_size();
    let pixels_per_point = window.scale_factor() as f32;
    let screen_descriptor = egui_wgpu::ScreenDescriptor {
        size_in_pixels: [size.width, size.height],
        pixels_per_point,
    };
    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: None,
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(background_color),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
    });

    drop(rpass);

    game.egui_renderer.draw(
        &game.iad.device,
        &game.iad.queue,
        &mut encoder,
        &window,
        &view,
        screen_descriptor,
        |ui| {
            egui::Window::new("Settings")
                .resizable(true)
                .vscroll(true)
                .default_open(true)
                .show(&ui, |mut ui| {
                    ui.label("Window!");
                    ui.label("Window!");
                    ui.label("Window!");
                    ui.label("Window!");
                    if ui.button("Click me!").clicked() {
                        log::info!("button clicked");
                    }

                    // proto_scene.egui(ui);
                });
        },
    );

    iad.queue.submit(Some(encoder.finish()));
}

fn ced(label: Option<&'static str>) -> wgpu::CommandEncoderDescriptor {
    wgpu::CommandEncoderDescriptor { label }
}
