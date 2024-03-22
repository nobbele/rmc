#version 330 core

layout(location = 0) in vec2 in_Position;
layout(location = 1) in vec2 in_Uv;

uniform mat3 uniform_Mat;

out vec2 vert_Uv;

void main() {
    vert_Uv = in_Uv;

    gl_Position = vec4(vec2(uniform_Mat * vec3(in_Position, 1.0)), 0.0, 1.0);
}
