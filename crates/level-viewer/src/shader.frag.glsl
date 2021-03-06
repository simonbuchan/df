#version 450
#extension GL_GOOGLE_include_directive : require
#extension GL_EXT_nonuniform_qualifier : require

#include "shader.uniforms.glsl"

layout(location = 0) in vec2 in_uv;
layout(location = 1) flat in uint in_tex;
layout(location = 2) flat in float in_light;

layout(location = 0) out vec4 out_color;

void main() {
    uint tex_index = in_tex & 0xffffu;
    uint tex_flags = in_tex >> 16;

    if ((tex_flags & 0x1u) == 0u) { // Sky?
        out_color = texture(sampler2D(u_textures[tex_index], u_sampler), in_uv, 0.0) * vec4(vec3(in_light), 1.0);
    } else {
        vec2 uv = (gl_FragCoord.xy - l.viewport_size / 2) / l.viewport_size.x * l.sky.x + vec2(l.sky.y, 0.5);
        out_color = texture(sampler2D(u_textures[tex_index], u_sampler), uv, 0.0);
    }
}
