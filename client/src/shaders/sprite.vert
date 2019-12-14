#version 330 core

layout (location = 0) in vec2 Position;
layout (location = 1) in vec2 aTexCoord;

uniform mat4 view;
uniform mat4 model;
uniform mat4 rot_mat;
uniform mat4 projection;
uniform vec2 size;
uniform vec2 offset;

out vec2 tex_coord;

void main() {
    vec4 pos = vec4(Position.x * size.x, Position.y * size.y, 0.0, 1.0);
    pos.x += offset.x;
    pos.y -= offset.y;
    mat4 model_view = view * model;

    // Spherical billboard
    model_view[0].xyz = vec3( 1.0, 0.0, 0.0 );
    model_view[1].xyz = vec3( 0.0, 1.0, 0.0 );
    model_view[2].xyz = vec3( 0.0, 0.0, 1.0 );

    gl_Position = projection * model_view * (rot_mat * pos);
    tex_coord = aTexCoord;
}