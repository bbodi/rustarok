#version 330 core

layout (location = 0) in vec3 Position;

uniform mat4 view;
uniform mat4 model;
uniform mat4 projection;
uniform vec2 size;

void main() {
    vec4 pos = vec4(Position.x * size.x, Position.y, Position.z * size.y, 1.0);
    gl_Position = projection * view * model * pos;
}