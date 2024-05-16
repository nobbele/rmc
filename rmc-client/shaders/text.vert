#version 330 core

const mat3 INVERT_Y_AXIS = mat3(
        vec3(1.0, 0.0, 0.0),
        vec3(0.0, -1.0, 0.0),
        vec3(0.0, 0.0, 1.0)
    );

uniform mat3 uniform_Transform;

in vec2 in_LeftTop;
in vec2 in_RightBottom;
in vec2 in_TexLeftTop;
in vec2 in_TexRightBottom;
in vec4 in_Color;

out vec2 vert_TexPos;
out vec4 vert_Color;

void main() {
    vec2 pos = vec2(0.0);
    float left = in_LeftTop.x;
    float right = in_RightBottom.x;
    float top = in_LeftTop.y;
    float bottom = in_RightBottom.y;

    switch (gl_VertexID) {
        case 0:
        pos = vec2(left, top);
        vert_TexPos = in_TexLeftTop;
        break;
        case 1:
        pos = vec2(right, top);
        vert_TexPos = vec2(in_TexRightBottom.x, in_TexLeftTop.y);
        break;
        case 2:
        pos = vec2(left, bottom);
        vert_TexPos = vec2(in_TexLeftTop.x, in_TexRightBottom.y);
        break;
        case 3:
        pos = vec2(right, bottom);
        vert_TexPos = in_TexRightBottom;
        break;
    }

    vert_Color = in_Color;
    vec2 view_pos = vec2(uniform_Transform * vec3(pos, 1.0));
    gl_Position = vec4(vec2(view_pos.x, 1.0 - view_pos.y) * vec2(2.0) - vec2(1.0), 0.0, 1.0);
}
