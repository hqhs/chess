use discipline::wgpu::{
  self, CommandEncoder, Device, Queue, TextureFormat, TextureView,
};
use egui::Context;
use egui_wgpu::{Renderer, ScreenDescriptor};
use egui_winit::{EventResponse, State};
use winit::{event::WindowEvent, window::Window};

pub struct EguiRenderer {
  pub context: Context,
  state: State,
  renderer: Renderer,
}

impl EguiRenderer {
  pub fn new(
    device: &Device,
    output_color_format: TextureFormat,
    output_depth_format: Option<TextureFormat>,
    msaa_samples: u32,
    window: &Window,
  ) -> EguiRenderer {
    let egui_context = Context::default();
    let max_texture_side = None;
    let egui_state = egui_winit::State::new(
      egui_context.clone(),
      egui::ViewportId::default(),
      &window,
      Some(window.scale_factor() as f32),
      max_texture_side,
    );
    let egui_renderer = Renderer::new(
      device,
      output_color_format,
      output_depth_format,
      msaa_samples,
    );

    EguiRenderer {
      context: egui_context,
      state: egui_state,
      renderer: egui_renderer,
    }
  }

  pub fn handle_input(
    &mut self,
    window: &Window,
    event: &WindowEvent,
  ) -> EventResponse {
    self.state.on_window_event(window, event)
  }

  pub fn draw(
    &mut self,
    device: &Device,
    queue: &Queue,
    encoder: &mut CommandEncoder,
    window: &Window,
    window_surface_view: &TextureView,
    screen_descriptor: ScreenDescriptor,
    run_ui: impl FnOnce(&Context),
  ) {
    // self.state.set_pixels_per_point(window.scale_factor() as f32);
    let raw_input = self.state.take_egui_input(window);
    let full_output = self.context.run(raw_input, |ui| {
      run_ui(&self.context);
    });

    self.state.handle_platform_output(window, full_output.platform_output);

    // BUG: typing '+' increases scale factor, which produces
    // panic in the code below
    let scale_factor = window.scale_factor() as f32;
    let tris = self.context.tessellate(full_output.shapes, scale_factor);
    for (id, image_delta) in &full_output.textures_delta.set {
      self.renderer.update_texture(device, queue, *id, image_delta);
    }
    self.renderer.update_buffers(
      device,
      queue,
      encoder,
      &tris,
      &screen_descriptor,
    );
    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
      color_attachments: &[Some(wgpu::RenderPassColorAttachment {
        view: window_surface_view,
        resolve_target: None,
        ops: wgpu::Operations {
          load: wgpu::LoadOp::Load,
          store: wgpu::StoreOp::Store,
        },
      })],
      occlusion_query_set: None,
      timestamp_writes: None,
      depth_stencil_attachment: None,
      label: Some("egui main render pass"),
    });
    self.renderer.render(&mut rpass, &tris, &screen_descriptor);
    drop(rpass);
    for x in &full_output.textures_delta.free {
      self.renderer.free_texture(x)
    }
  }
}
