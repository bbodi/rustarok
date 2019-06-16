#version 330 core

out vec4 Color;

in vec2 tex_coord;

uniform sampler2D model_texture;
uniform float alpha;


void main() {
    vec4 texture = texture2D(model_texture, tex_coord);
    if (texture.a == 0.0) {
        //Color = vec4(1, 0, 1, 1);
        discard;
    } else {
        Color = texture;
        //    Color = vec4(1, 0, 0, 1);
        Color.a *= alpha;
    }

}