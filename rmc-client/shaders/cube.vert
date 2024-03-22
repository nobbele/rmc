#version 330 core

layout(location = 0) in vec3 in_Position;
layout(location = 1) in vec2 in_Uv;
layout(location = 2) in vec3 instance_Position;
layout(location = 3) in int instance_Texture;

uniform mat4 uniform_Mvp;
uniform vec3 uniform_Highlighted;

out vec3 vert_Position;
out vec2 vert_Uv;
flat out int vert_Texture;

out float vert_Highlighted;

void main() {
    vert_Position = in_Position;
    vert_Uv = in_Uv;
    vert_Texture = instance_Texture;
    vert_Highlighted = instance_Position == uniform_Highlighted ? 1.0 : 0.0;

    gl_Position = uniform_Mvp * vec4(in_Position + instance_Position, 1.0);
}
