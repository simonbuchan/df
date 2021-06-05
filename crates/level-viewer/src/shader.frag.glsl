#version 450
#extension GL_EXT_nonuniform_qualifier : require

layout(location = 0) in vec2 in_uv;
layout(location = 1) flat in uint in_tex;
layout(location = 2) flat in float in_light;

layout(location = 0) out vec4 out_color;

layout(set = 0, binding = 1) uniform texture2D u_textures[];
layout(set = 0, binding = 2) uniform sampler u_sampler;

void main() {
    out_color = texture(sampler2D(u_textures[in_tex], u_sampler), in_uv, 0.0) * vec4(vec3(in_light), 1.0);
}
