#version 330 core

layout (location = 0) in vec3 Position;
layout (location = 1) in vec4 a_color;

uniform mat4 view;
uniform mat4 model;
uniform mat4 projection;

out vec4 color;

void main() {
    vec4 pos = vec4(Position.x, Position.y, Position.z, 1.0);
    color =  a_color;
    gl_Position = projection * view * model * pos;
}