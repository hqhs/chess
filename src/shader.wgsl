struct VertexOutput {
  @builtin(position) position : vec4<f32>,
  // @location(0) vert_pos : vec3<f32>,
}
@group(0)
@binding(0)
var<uniform> transform: mat4x4<f32>;

@vertex fn vs_main(@location(0) position: vec4<f32>) -> VertexOutput {
  var out : VertexOutput;
  out.position = position * transform;

  return out;
}

// Fragment shader

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(0.3, 0.2, 0.1, 1.0);
}
