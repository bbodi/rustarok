#version 330 core

out vec4 Color;

in vec2 tex_coord;

uniform sampler2D model_texture;


void main() {
    vec4 texture = texture2D(model_texture, tex_coord);
    Color = texture;
}