#version 330 core

layout (location = 0) in vec2 Position;
layout (location = 1) in vec2 aTexCoord;

uniform mat4 view;
uniform mat4 model;
uniform mat4 projection;
uniform vec2 size;
uniform vec2 offset;

out vec2 tex_coord;

void main() {
    vec4 pos = vec4(Position.x * size.x, Position.y * size.y, 0.0, 1.0);
    pos.x += offset.x;
    pos.y -= offset.y;
    mat4 model_view = view * model;
    model_view[0][0] = 1.0;
    model_view[0][1] = 0.0;
    model_view[0][2] = 0.0;

//    if (spherical == 1) {
        // Second colunm.
        model_view[1][0] = 0.0;
        model_view[1][1] = 1.0;
        model_view[1][2] = 0.0;
//    }

    // Thrid colunm.
    model_view[2][0] = 0.0;
    model_view[2][1] = 0.0;
    model_view[2][2] = 1.0;

    gl_Position = projection * model_view * pos;
    tex_coord = aTexCoord;
}