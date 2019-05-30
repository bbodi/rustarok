#version 330 core

out vec4 Color;

in vec2 tex_coord;

uniform sampler2D gnd_texture_atlas;
uniform sampler2D tile_color_texture;

varying vec2 vTileColorCoord;
//varying float vLightWeighting;

void main()
{
    float lightWeight = 1.0;
    vec4 texture = texture2D(gnd_texture_atlas, tex_coord);
    if (vTileColorCoord.st != vec2(0.0, 0.0)) {
	    texture    *= texture2D(tile_color_texture, vTileColorCoord.st);
		// lightWeight = vLightWeighting;
	}
    Color = texture;
}