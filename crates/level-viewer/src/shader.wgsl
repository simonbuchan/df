[[group(0), binding(1)]]
var r_color: texture_2d<f32>;
[[group(0), binding(2)]]
var r_sampler: sampler;

let size: vec2<f32> = vec2<f32>(256.0, 256.0);

[[stage(vertex)]]
fn vs_main(
    [[location(0)]] position: vec4<f32>,
) -> [[builtin(position)]] vec4<f32> {
    return position;
}

[[stage(fragment)]]
fn fs_main(
    [[builtin(position)]] position: vec4<f32>,
) -> [[location(0)]] vec4<f32> {
    return textureSampleLevel(r_color, r_sampler, position.xy / size, 0.0);
    // return textureLoad(r_color, vec2<i32>(position.xy / 4.0) % textureDimensions(r_color), 0);
}
