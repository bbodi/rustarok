#version 330 core

out vec4 out_color;


in vec2 tex_coord;

uniform vec3 color;
uniform sampler2D model_texture;
uniform float alpha;


void main() {
    vec4 texture = texture2D(model_texture, tex_coord);
    if (texture.a == 0.0) {
        discard;
    } else {
        out_color = texture * vec4(color, alpha);
    }

}