#version 330 core

out vec4 Color;

in vec2 tex_coord;

uniform sampler2D gnd_texture_atlas;
uniform sampler2D tile_color_texture;
uniform sampler2D lightmap_texture;

uniform bool use_tile_color;
uniform bool use_lightmap;
uniform bool use_lighting;

uniform vec3 light_ambient;
uniform vec3 light_diffuse;
uniform float light_opacity;

in vec2 vLightmapCoord;
in vec2 vTileColorCoord;
in float vLightWeighting;

void main() {
    vec4 texture = texture2D(gnd_texture_atlas, tex_coord);
    if (use_tile_color && vTileColorCoord.st != vec2(0.0, 0.0)) {
        texture    *= texture2D(tile_color_texture, vTileColorCoord.st);
    }

    if (use_lightmap) {
        vec3 Ambient    = light_ambient * light_opacity;
        vec3 Diffuse    = light_diffuse * vLightWeighting;
        vec4 lightmap   = texture2D(lightmap_texture, vLightmapCoord.st);
        vec4 LightColor = vec4((Ambient + Diffuse) * lightmap.a, 1.0);
        vec4 ColorMap   = vec4(lightmap.rgb, 0.0);
        Color = texture * clamp(LightColor, 0.0, 1.0) + ColorMap;
    } else if (use_lighting) {
        vec3 Ambient    = light_ambient * light_opacity;
        vec3 Diffuse    = light_diffuse * vLightWeighting;
        vec4 LightColor = vec4((Ambient + Diffuse), 1.0);
        Color = texture * clamp(LightColor, 0.0, 1.0);
    } else {
        Color = texture;
    }
    Color = vec4(Color.rgb, 1.0);
}