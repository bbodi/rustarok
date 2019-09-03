package rustarok.render

import org.khronos.webgl.*
import rustarok.*

fun render_ground(gl: WebGL2RenderingContext, command: RenderCommand.Ground3D, ground_gl_program: GroundShader,
                  ground_vertex_buffer: WebGLBuffer, ground_vertex_count: Int) {
    gl.useProgram(ground_gl_program.program)
    gl.uniformMatrix4fv(ground_gl_program.projection_mat, false, PROJECTION_MATRIX)
    gl.uniformMatrix4fv(ground_gl_program.model, false, VIEW_MATRIX)
    gl.uniformMatrix3fv(ground_gl_program.normal_matrix, false, NORMAL_MATRIX)

    gl.uniform3fv(ground_gl_program.light_dir, command.light_dir)
    gl.uniform3fv(ground_gl_program.light_ambient, command.light_ambient)
    gl.uniform3fv(ground_gl_program.light_diffuse, command.light_diffuse)
    gl.uniform1f(ground_gl_program.light_opacity, command.light_opacity)

    gl.bindBuffer(WebGLRenderingContext.ARRAY_BUFFER, ground_vertex_buffer)

    gl.enableVertexAttribArray(ground_gl_program.a_pos)
    gl.enableVertexAttribArray(ground_gl_program.a_uv)
    gl.enableVertexAttribArray(ground_gl_program.a_vertex_normal)
    gl.enableVertexAttribArray(ground_gl_program.a_lightmap_coord)
    gl.enableVertexAttribArray(ground_gl_program.a_tile_color_coord)

    gl.vertexAttribPointer(ground_gl_program.a_pos, 3, WebGLRenderingContext.FLOAT, false, 12 * 4, 0)
    gl.vertexAttribPointer(ground_gl_program.a_vertex_normal, 3, WebGLRenderingContext.FLOAT, false, 12 * 4, 3 * 4)
    gl.vertexAttribPointer(ground_gl_program.a_uv, 2, WebGLRenderingContext.FLOAT, false, 12 * 4, 6 * 4)
    gl.vertexAttribPointer(ground_gl_program.a_lightmap_coord, 2, WebGLRenderingContext.FLOAT, false, 12 * 4, 8 * 4)
    gl.vertexAttribPointer(ground_gl_program.a_tile_color_coord, 2, WebGLRenderingContext.FLOAT, false, 12 * 4, 10 * 4)


    gl.activeTexture(WebGLRenderingContext.TEXTURE0)
    gl.bindTexture(WebGLRenderingContext.TEXTURE_2D, get_or_load_server_texture(command.server_texture_atlas_id, WebGLRenderingContext.NEAREST))
    gl.uniform1i(ground_gl_program.gnd_texture_atlas, 0)

    gl.activeTexture(WebGLRenderingContext.TEXTURE1)
    gl.bindTexture(WebGLRenderingContext.TEXTURE_2D, get_or_load_server_texture(command.server_tile_color_texture_id, WebGLRenderingContext.LINEAR))
    gl.uniform1i(ground_gl_program.tile_color_texture, 1)

    gl.activeTexture(WebGLRenderingContext.TEXTURE2)
    gl.bindTexture(WebGLRenderingContext.TEXTURE_2D, get_or_load_server_texture(command.server_lightmap_texture_id, WebGLRenderingContext.LINEAR))
    gl.uniform1i(ground_gl_program.lightmap_texture, 2)


    gl.drawArrays(WebGLRenderingContext.TRIANGLES, 0, ground_vertex_count)

    gl.disableVertexAttribArray(ground_gl_program.a_pos)
    gl.disableVertexAttribArray(ground_gl_program.a_uv)
    gl.disableVertexAttribArray(ground_gl_program.a_vertex_normal)
    gl.disableVertexAttribArray(ground_gl_program.a_lightmap_coord)
    gl.disableVertexAttribArray(ground_gl_program.a_tile_color_coord)
}

fun create_vertex_buffer(gl: WebGL2RenderingContext, raw: Uint8Array): WebGLBuffer {
    val buffer = gl.createBuffer()!!
    gl.bindBuffer(WebGLRenderingContext.ARRAY_BUFFER, buffer)
    try {
        gl.bufferData(WebGLRenderingContext.ARRAY_BUFFER,
                      Float32Array(raw.buffer.slice(raw.byteOffset, raw.byteOffset + raw.byteLength)),
                      WebGLRenderingContext.STATIC_DRAW)
    } catch (e: Throwable) {
        js("debugger")
    }
    return buffer
}

fun load_ground_shader(gl: WebGL2RenderingContext): GroundShader {
    val vs = gl.createShader(WebGLRenderingContext.VERTEX_SHADER).apply {
        gl.shaderSource(this, """#version 300 es

layout (location = 0) in vec3 Position;
layout (location = 1) in vec3 aVertexNormal;
layout (location = 2) in vec2 aTexCoord;
layout (location = 3) in vec2 aLightmapCoord;
layout (location = 4) in vec2 aTileColorCoord;

uniform mat4 model_matrix;
uniform mat4 projection;
uniform mat3 normal_matrix;

uniform vec3 light_dir;


out vec2 tex_coord;
out vec2 vLightmapCoord;
out vec2 vTileColorCoord;
out float vLightWeighting;

void main() {
    gl_Position = projection * model_matrix * vec4(Position, 1.0);

    tex_coord = aTexCoord;
    vLightmapCoord = aLightmapCoord;
    vTileColorCoord = aTileColorCoord;

    vec4 lDirection  = model_matrix * vec4( light_dir, 0.0);
    vec3 dirVector   = normalize(lDirection.xyz);
    float dotProduct = dot( normal_matrix * aVertexNormal, dirVector );
    vLightWeighting  = max( dotProduct, 0.1 );
}""")
        gl.compileShader(this)

        if (gl.getShaderParameter(this, WebGLRenderingContext.COMPILE_STATUS) != null) {
            val error = gl.getShaderInfoLog(this)
            if (!error.isNullOrEmpty()) {
                gl.deleteShader(this)

                throw IllegalArgumentException(error)
            }
        }

    }

    val fs = gl.createShader(WebGLRenderingContext.FRAGMENT_SHADER).apply {
        gl.shaderSource(this, """#version 300 es
precision mediump float;

out vec4 Color;

in vec2 tex_coord;

uniform sampler2D gnd_texture_atlas;
uniform sampler2D tile_color_texture;
uniform sampler2D lightmap_texture;

uniform vec3 light_ambient;
uniform vec3 light_diffuse;
uniform float light_opacity;

in vec2 vLightmapCoord;
in vec2 vTileColorCoord;
in float vLightWeighting;

void main() {
    vec4 tex = texture(gnd_texture_atlas, tex_coord.st);
    if (vTileColorCoord.st != vec2(0.0, 0.0)) {
        tex    *= texture(tile_color_texture, vTileColorCoord.st);
    }

    vec3 Ambient    = light_ambient * light_opacity;
    vec3 Diffuse    = light_diffuse * vLightWeighting;
    vec4 lightmap   = texture(lightmap_texture, vLightmapCoord.st);
    vec4 LightColor = vec4((Ambient + Diffuse) * lightmap.a, 1.0);
    vec4 ColorMap   = vec4(lightmap.rgb, 0.0);
    Color = tex * clamp(LightColor, 0.0, 1.0) + ColorMap;
}""")
        gl.compileShader(this)

        if (gl.getShaderParameter(this, WebGLRenderingContext.COMPILE_STATUS) != null) {
            val error = gl.getShaderInfoLog(this)
            if (!error.isNullOrEmpty()) {
                gl.deleteShader(this)

                throw IllegalArgumentException(error)
            }
        }
    }

    val program = gl.createProgram()
    gl.attachShader(program, vs)
    gl.attachShader(program, fs)
    gl.linkProgram(program)

    if (gl.getProgramParameter(program, WebGLRenderingContext.LINK_STATUS) != null) {
        val error = gl.getProgramInfoLog(program)
        if (!error.isNullOrEmpty()) {
            gl.deleteProgram(program)
            gl.deleteShader(vs)
            gl.deleteShader(fs)

            throw IllegalArgumentException(error)
        }
    }
    return GroundShader(program = program!!,
                        projection_mat = gl.getUniformLocation(program, "projection")!!,
                        normal_matrix = gl.getUniformLocation(program, "normal_matrix")!!,
                        light_dir = gl.getUniformLocation(program, "light_dir")!!,
                        model = gl.getUniformLocation(program, "model_matrix")!!,
                        a_pos = gl.getAttribLocation(program, "Position"),
                        a_vertex_normal = gl.getAttribLocation(program, "aVertexNormal"),
                        a_lightmap_coord = gl.getAttribLocation(program, "aLightmapCoord"),
                        a_tile_color_coord = gl.getAttribLocation(program, "aTileColorCoord"),
                        a_uv = gl.getAttribLocation(program, "aTexCoord"),
                        gnd_texture_atlas = gl.getUniformLocation(program, "gnd_texture_atlas")!!,
                        tile_color_texture = gl.getUniformLocation(program, "tile_color_texture")!!,
                        lightmap_texture = gl.getUniformLocation(program, "lightmap_texture")!!,
                        light_ambient = gl.getUniformLocation(program, "light_ambient")!!,
                        light_diffuse = gl.getUniformLocation(program, "light_diffuse")!!,
                        light_opacity = gl.getUniformLocation(program, "light_opacity")!!)
}

data class GroundShader(val program: WebGLProgram,
                        val projection_mat: WebGLUniformLocation,
                        val model: WebGLUniformLocation,
                        val a_pos: Int,
                        val a_uv: Int,
                        val normal_matrix: WebGLUniformLocation,
                        val light_dir: WebGLUniformLocation,
                        val a_vertex_normal: Int,
                        val a_lightmap_coord: Int,
                        val a_tile_color_coord: Int,
                        val gnd_texture_atlas: WebGLUniformLocation,
                        val tile_color_texture: WebGLUniformLocation,
                        val lightmap_texture: WebGLUniformLocation,
                        val light_ambient: WebGLUniformLocation,
                        val light_diffuse: WebGLUniformLocation,
                        val light_opacity: WebGLUniformLocation)