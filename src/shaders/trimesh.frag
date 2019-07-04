#version 330 core

out vec4 out_color;
uniform float alpha;
uniform vec4 color;

void main() {
    out_color = color;
}