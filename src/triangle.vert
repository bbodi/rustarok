#version 330 core

layout (location = 0) in vec3 Position;
layout (location = 1) in vec2 aTexCoord;
layout (location = 2) in vec2 aTileColorCoord;

uniform mat4 model;
uniform mat4 view;
uniform mat4 projection;

out vec2 tex_coord;
varying vec2 vTileColorCoord;

void main()
{
    gl_Position = projection * view * model * vec4(Position, 1.0);
    tex_coord = aTexCoord;
    vTileColorCoord = aTileColorCoord;
}