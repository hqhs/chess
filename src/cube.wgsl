struct VertexOutput {
  @location(0) tex_coord : vec2<f32>, @builtin(position) position : vec4<f32>,
};

struct UniformInput {
  camera_transform : mat4x4<f32>, scale_matrix : mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> uniform_input : UniformInput;

@vertex fn vs_main(@location(0) position
                   : vec4<f32>, @location(1) tex_coord
                   : vec2<f32>, )
    ->VertexOutput {
  var result : VertexOutput;
  // var scaled_position = position * uniform_input.scale;
  result.tex_coord = tex_coord;
  // result.position = uniform_input.camera_transform *
  // uniform_input.scale_matrix * position;
  result.position = uniform_input.camera_transform * position;
  return result;
}

@group(0) @binding(1) var r_color : texture_2d<u32>;

@fragment fn fs_main(vertex : VertexOutput)->@location(0) vec4<f32> {
  let tex = textureLoad(r_color, vec2<i32>(vertex.tex_coord * 256.0), 0);
  let v = f32(tex.x) / 255.0;
  return vec4<f32>(1.0 - (v * 5.0), 1.0 - (v * 15.0), 1.0 - (v * 50.0), 1.0);
  // return vec4<f32>(0.3, 0.2, 0.1, 1.0);
}

// @fragment
// fn fs_wire(vertex: VertexOutput) -> @location(0) vec4<f32> {
//     return vec4<f32>(0.0, 0.5, 0.0, 0.5);
// }
