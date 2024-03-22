#version 330 core

in vec3 vert_Position;
in vec2 vert_Uv;
flat in int vert_Texture;

in float vert_Highlighted;

out vec4 frag_Color;

uniform sampler2DArray uniform_Texture;

void main() {
    float z = float(vert_Texture);
    vec3 texel = vec3(texture(uniform_Texture, vec3(vert_Uv, z)));
    vec3 highlightColor = vert_Highlighted > 0.5 ? vec3(0.5, 0.5, 0.5) : vec3(0.0, 0.0, 0.0);
    frag_Color = vec4(texel + highlightColor, 1.0);
}
