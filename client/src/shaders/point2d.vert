#version 330 core

layout (location = 0) in vec3 Position;
layout (location = 1) in vec4 Color;

uniform mat4 projection;

out vec4 out_color;

void main() {
    vec4 pos = vec4(Position, 1.0);
    gl_Position = projection * pos;
    out_color = Color;
}