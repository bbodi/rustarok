#version 330 core

out vec4 Color;

in vec2 tex_coord;

uniform sampler2D model_texture;

uniform bool use_lighting;

uniform vec3 light_ambient;
uniform vec3 light_diffuse;
uniform float light_opacity;

in float vLightWeighting;
uniform float alpha;


void main() {
    vec4 texture = texture2D(model_texture, tex_coord);

    if (use_lighting) {
        vec3 Ambient    = light_ambient * light_opacity;
        vec3 Diffuse    = light_diffuse * vLightWeighting;
        vec4 LightColor = vec4((Ambient + Diffuse), 1.0);
        Color = texture * clamp(LightColor, 0.0, 1.0);
    } else {
        Color = texture;
    }
    Color.a *= alpha;

}