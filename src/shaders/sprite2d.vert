#version 330 core

layout (location = 0) in vec2 Position;
layout (location = 1) in vec2 aTexCoord;

uniform mat4 model;
uniform mat4 projection;
uniform vec2 size;
uniform ivec2 offset;
uniform float z;


out vec2 tex_coord;

void main() {
    vec2 pos = vec2(Position.x * size.x, Position.y * size.y);
    pos.x += float(offset.x);
    pos.y += float(offset.y);

    gl_Position = projection * model * vec4(pos.xy, z, 1.0);
    tex_coord = aTexCoord;
}