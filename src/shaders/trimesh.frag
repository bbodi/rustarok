#version 330 core

out vec4 out_color;
uniform float alpha;
uniform vec3 color;

void main() {
    out_color = vec4(color, 1);
}