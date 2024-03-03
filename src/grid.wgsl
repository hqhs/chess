struct VertexOutput {
  @location(0) tex_coord : vec2<f32>, @builtin(position) position : vec4<f32>,
};

struct UniformInput {
  projection: mat4x4<f32>,
  view: mat4x4<f32>,
  scale : mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> uniform_input : UniformInput;

@vertex fn vs_main(@location(0) position
                   : vec4<f32>, @location(1) tex_coord
                   : vec2<f32>, )
    ->VertexOutput {
  var result : VertexOutput;
  let camera_transform = uniform_input.projection * uniform_input.view;

  result.position = camera_transform * uniform_input.scale * position;
  result.tex_coord = tex_coord;
  return result;
}

@fragment fn fs_main(vertex : VertexOutput)->@location(0) vec4<f32> {
  var scale : f32 = 640.0;
  var coord : vec2<f32> = vertex.tex_coord * scale;
  var derivative : vec2<f32> = fwidth(coord);
  var grid = abs(fract(coord - 0.5) - 0.5) / derivative;
  var line : f32 = min(grid.x, grid.y);
  var minimumy = min(derivative.y, 1.0);
  var minimumx = min(derivative.x, 1.0);
  var color = vec4(0.4, 0.4, 0.4, 1.0 - min(line, 1.0));
  if (vertex.position.x > -0.1 * minimumx &&
      vertex.position.x < 0.1 * minimumx) {
    color.z = 1.0;
  }
  if (vertex.position.y > -0.1 * minimumy &&
      vertex.position.y < 0.1 * minimumy) {
    color.r = 1.0;
  }

  // return vec4(0.2, 0.2, 0.2, 0.5);
  return color;
}
