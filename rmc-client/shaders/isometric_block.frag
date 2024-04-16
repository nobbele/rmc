#version 330 core

in vec2 vert_Uv;

out vec4 frag_Color;

uniform sampler2DArray uniform_Texture;
uniform uint uniform_TextureLayer;

void main() {
    float z = float(uniform_TextureLayer);
    vec4 texel = texture(uniform_Texture, vec3(vert_Uv, z));
    if (texel.w == 0.0) {
        discard;
    }
    frag_Color = texel;
}
