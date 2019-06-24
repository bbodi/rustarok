#version 330 core

layout (location = 0) in vec3 Position;

uniform mat4 view;
uniform mat4 model;
uniform mat4 projection;

void main() {
    mat4 model_view = view * model;
    gl_Position = projection * model_view * vec4(Position, 1.0);
}