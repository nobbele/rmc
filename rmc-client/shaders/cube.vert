#version 330 core

layout(location = 0) in vec3 in_Position;
layout(location = 1) in vec2 in_Uv;
layout(location = 2) in uint in_Face;
layout(location = 3) in vec3 instance_Position;
layout(location = 4) in uint instance_Texture;
layout(location = 5) in uvec4 instance_Light1;
layout(location = 6) in uvec2 instance_Light2;

uniform mat4 uniform_Mvp;
uniform vec3 uniform_Highlighted;

out vec3 vert_Position;
out vec2 vert_Uv;
out float vert_Highlighted;
flat out uint vert_Texture;
flat out uint vert_Light;

void main() {
    uint light[6] = uint[6](
            instance_Light1.x,
            instance_Light1.y,
            instance_Light1.z,
            instance_Light1.w,
            instance_Light2.x,
            instance_Light2.y
        );

    vert_Position = in_Position;
    vert_Uv = in_Uv;
    vert_Light = light[in_Face];
    vert_Texture = instance_Texture;
    vert_Highlighted = instance_Position == uniform_Highlighted ? 1.0 : 0.0;

    gl_Position = uniform_Mvp * vec4(in_Position + instance_Position, 1.0);
}
