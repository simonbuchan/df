#version 450

layout(location = 0) in vec3 in_pos;
layout(location = 1) in vec2 in_uv;
layout(location = 2) in uint in_tex;
layout(location = 3) in uint in_light;

layout(location = 0) out vec2 out_uv;
layout(location = 1) flat out uint out_tex;
layout(location = 2) flat out float out_light;

layout(set = 0, binding = 0) uniform Locals {
    mat4 view;
} l;

void main() {
    gl_Position = vec4(in_pos, 1) * l.view;
    out_uv = in_uv;
    out_tex = in_tex;
    out_light = clamp(float(in_light) / 50.0, 0, 1);
}
