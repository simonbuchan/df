#version 450
#extension GL_GOOGLE_include_directive : require
#include "shader.uniforms.glsl"

layout(location = 0) in vec3 in_pos;
layout(location = 1) in vec2 in_uv;
layout(location = 2) in uint in_tex;
layout(location = 3) in uint in_light;

layout(location = 0) out vec2 out_uv;
layout(location = 1) flat out uint out_tex;
layout(location = 2) flat out float out_light;

void main() {
    gl_Position = l.view * vec4(in_pos, 1);
    out_uv = in_uv;
    out_tex = in_tex;
    out_light = pow(clamp(float(in_light) / 30.0, 0, 1), 2.2);
}
