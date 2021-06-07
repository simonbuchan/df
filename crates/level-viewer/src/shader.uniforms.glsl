layout(set = 0, binding = 0) uniform Locals {
    mat4 view;
    vec2 viewport_size;
    vec2 sky; // angular size, offset
} l;
layout(set = 0, binding = 1) uniform texture2D u_textures[];
layout(set = 0, binding = 2) uniform sampler u_sampler;
