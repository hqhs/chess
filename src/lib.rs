use std::sync::Arc;

use winit::event::{Event, WindowEvent};

use discipline::{glam, setup, wgpu};

pub fn log_iad_info(iad: &discipline::InstanceAdapterDevice) {
    // let info = iad.adapter.get_info();
    // let limits = iad.adapter.limits();
    // let features = iad.adapter.features();
    // log::debug!("Chosen adapter: {:#?}", adapter.i)
}

struct Game {
    iad: discipline::InstanceAdapterDevice,
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

    let game = Game { iad };

    run_event_loop(event_loop, game, &surface)?;

    Ok(())
}

fn run_event_loop(
    event_loop: winit::event_loop::EventLoop<()>,
    mut game: Game,
    surface: &wgpu::Surface,
) -> anyhow::Result<()> {
    event_loop.run(move |event, event_loop_window_target| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => event_loop_window_target.exit(),
        // next: resizing
        Event::WindowEvent {
            event: WindowEvent::RedrawRequested,
            ..
        } => {
            match redraw(&mut game, surface) {
                Ok(_) => {
                    // record timing maybe?
                }
                Err(e) => {
                    log::error!("failed to redraw: {}", e);
                }
            };
        }
        _ => {}
    })?;

    Ok(())
}

fn redraw(game: &mut Game, surface: &wgpu::Surface) -> anyhow::Result<()> {
    let frame = surface.get_current_texture()?;
    let texture_descriptor = wgpu::TextureViewDescriptor::default();
    let view = frame.texture.create_view(&texture_descriptor);
    let iad = &game.iad;
    // next thing: clear screen with some color
    let mut encoder = iad
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    let background_color = wgpu::Color {
        r: 0.1,
        g: 0.2,
        b: 0.3,
        a: 1.0,
    };
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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

    drop(pass);

    // output.present
    iad.queue.submit(Some(encoder.finish()));
    frame.present();
    Ok(())
}
