#version 330 core

layout (location = 0) in vec2 vertex_pos;
layout (location = 1) in vec2 aTexCoord;

uniform mat4 view;
uniform mat4 model;
uniform mat4 projection;
uniform vec2 offset;

const float ONE_SPRITE_PIXEL_SIZE_IN_3D = 1.0 / 35.0;


out vec2 tex_coord;

void main() {
    vec4 pos = vec4(vertex_pos.x * ONE_SPRITE_PIXEL_SIZE_IN_3D,
                    vertex_pos.y * ONE_SPRITE_PIXEL_SIZE_IN_3D,
                    0.0, 1.0);
    pos.x += offset.x * ONE_SPRITE_PIXEL_SIZE_IN_3D;
    pos.y -= offset.y * ONE_SPRITE_PIXEL_SIZE_IN_3D - 10.0;
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