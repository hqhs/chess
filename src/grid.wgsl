struct VertexOutput {
  @location(0) tex_coord : vec2<f32>,
  @builtin(position) position : vec4<f32>,
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

fn compute_linear_depth(pos: vec4<f32>) -> f32 {
    var near = 0.1;
    var far = 1.0;
    var clip_space_pos = pos;
    var clip_space_depth = (clip_space_pos.z / clip_space_pos.w) * 2.0 - 1.0; // put back between -1 and 1
    var linearDepth = (2.0 * near * far) / (far + near - clip_space_depth * (far - near)); // get linear value between 0.01 and 100
    return linearDepth / far; // normalize
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
  // https://gamedev.stackexchange.com/questions/93055/getting-the-real-fragment-depth-in-glsl
  color.a /= vertex.position.z / vertex.position.w;
  return color;
}
