#version 330 core

out vec4 out_color;

in vec2 tex_coord;

uniform vec4 color;
uniform sampler2D model_texture;

void main() {
    vec4 texture = texture2D(model_texture, tex_coord);
    if (texture.a == 0.0 || color.a == 0.0) {
        discard;
    } else {
        out_color = texture * color;
    }

}