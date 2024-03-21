#version 330 core

in vec3 vert_Position;
in vec2 vert_Uv;
in float vert_Highlighted;

out vec4 frag_Color;

uniform sampler2D uniform_Texture;

void main() {
    vec3 texel = vec3(texture(uniform_Texture, vert_Uv));
    frag_Color = vec4(vert_Highlighted > 0.5 ? vec3(1.0, 1.0, 1.0) : texel, 1.0);
}
