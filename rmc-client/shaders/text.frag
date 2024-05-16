#version 330 core

uniform sampler2D uniform_Texture;

in vec2 vert_TexPos;
in vec4 vert_Color;

out vec4 frag_Color;

void main() {
    float alpha = texture(uniform_Texture, vert_TexPos).r;
    if (alpha <= 0.0) {
        discard;
    }

    frag_Color = vert_Color * vec4(1.0, 1.0, 1.0, alpha);
}