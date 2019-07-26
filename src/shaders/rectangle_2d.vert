#version 330 core

layout (location = 0) in vec2 Position;

uniform mat4 model;
uniform mat4 projection;
uniform vec2 size;

void main() {
    vec2 pos = vec2(Position.x * size.x, Position.y * size.y);

    gl_Position = projection * model * vec4(pos.xy, 0.0, 1.0);
}