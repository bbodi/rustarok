#version 330 core

layout (location = 0) in vec3 Position;
layout (location = 1) in vec4 a_color;

uniform mat4 view;
uniform mat4 model;
uniform mat4 projection;
uniform vec3 scale;
uniform vec4 global_color;

varying vec4 color;

void main() {
    vec4 pos = vec4(Position.x * scale.x, Position.y * scale.y, Position.z * scale.z, 1.0);
    color =  global_color * a_color;
    gl_Position = projection * view * model * pos;
}