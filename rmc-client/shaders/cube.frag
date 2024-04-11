#version 330 core

in vec3 vert_Position;
in vec2 vert_Uv;
in float vert_Highlighted;
flat in uint vert_Texture;
flat in uint vert_Light;

out vec4 frag_Color;

uniform sampler2DArray uniform_Texture;

void main() {
    float z = float(vert_Texture);
    vec3 texel = vec3(texture(uniform_Texture, vec3(vert_Uv, z)));
    vec3 highlightColor = vert_Highlighted > 0.5 ? vec3(0.5, 0.5, 0.5) : vec3(0.0, 0.0, 0.0);

    // frag_Color = vec4(texel + highlightColor, 1.0);
    float lightStrength = float(vert_Light) / 15.0;
    frag_Color = vec4(lightStrength * texel + highlightColor, 1.0);
}
