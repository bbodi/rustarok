#version 330 core

layout (location = 0) in vec2 Position;
layout (location = 1) in vec2 aTexCoord;

uniform mat4 view;
uniform mat4 model;
uniform mat4 projection;
uniform vec2 size;

out vec2 tex_coord;

void main() {
    vec4 pos = vec4(Position.x * size.x, 0.0, Position.y * size.y, 1.0);
    mat4 model_view = view * model;

    gl_Position = projection * model_view * pos;
    tex_coord = aTexCoord;
}