#version 330 core

out vec4 Color;

in vec2 tex_coord;

uniform sampler2D our_texture;

void main()
{
    Color = texture(our_texture, tex_coord);
}