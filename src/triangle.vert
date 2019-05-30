#version 330 core

layout (location = 0) in vec3 Position;
layout (location = 1) in vec3 aVertexNormal;
layout (location = 2) in vec2 aTexCoord;
layout (location = 3) in vec2 aLightmapCoord;
layout (location = 4) in vec2 aTileColorCoord;

uniform mat4 model_view;
uniform mat4 projection;
uniform mat3 normal_matrix;

uniform vec3 light_dir;


out vec2 tex_coord;
out vec2 vLightmapCoord;
out vec2 vTileColorCoord;
out float vLightWeighting;

void main()
{
    gl_Position = projection * model_view * vec4(Position, 1.0);

    tex_coord = aTexCoord;
    vLightmapCoord = aLightmapCoord;
    vTileColorCoord = aTileColorCoord;

    vec4 lDirection  = model_view * vec4( light_dir, 0.0);
    vec3 dirVector   = normalize(lDirection.xyz);
    float dotProduct = dot( normal_matrix * aVertexNormal, dirVector );
    vLightWeighting  = max( dotProduct, 0.1 );
}