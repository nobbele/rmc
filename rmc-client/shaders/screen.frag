#version 330 core

in vec2 vert_Uv;

out vec4 frag_Color;

uniform sampler2D uniform_Texture;

void main() {
    vec4 texel = texture(uniform_Texture, vert_Uv);
    if (texel.w == 0.0) {
        discard;
    }
    frag_Color = texel;
}
