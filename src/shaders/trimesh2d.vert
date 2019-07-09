#version 330 core

layout (location = 0) in vec2 Position;

uniform mat4 view;
uniform mat4 model;
uniform mat4 projection;

void main() {
    vec4 pos = vec4(Position.x, Position.y, 0.0, 1.0);
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
}