use winit::event::{Event, WindowEvent};

pub async fn run() -> anyhow::Result<()> {
    env_logger::init();
    let size = winit::dpi::LogicalSize::new(800.0, 600.0);
    let event_loop = winit::event_loop::EventLoop::new()?;
    let window = winit::window::WindowBuilder::new()
        .with_resizable(true)
        .with_inner_size(size)
        .build(&event_loop)?;

    event_loop.run(move |event, event_loop_window_target| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => event_loop_window_target.exit(),
        _ => {}
    })?;

    Ok(())
}
