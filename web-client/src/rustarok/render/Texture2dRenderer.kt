package rustarok.render

import org.khronos.webgl.*
import rustarok.*

class Texture2dRenderer(gl: WebGL2RenderingContext) {

    private val tex2d_gl_program = load_texture2d_shader(gl)

    fun render_texture_2d(gl: WebGL2RenderingContext, commands: ArrayList<RenderCommand.Texture2D>,
                          sprite_vertex_buffer: WebGLBuffer) {
        gl.useProgram(tex2d_gl_program.program)
        gl.uniformMatrix4fv(tex2d_gl_program.projection_mat, false, ORTHO_MATRIX)

        gl.activeTexture(WebGLRenderingContext.TEXTURE0)
        gl.uniform1i(tex2d_gl_program.texture, 0)

        gl.bindBuffer(WebGLRenderingContext.ARRAY_BUFFER, sprite_vertex_buffer)
        gl.enableVertexAttribArray(tex2d_gl_program.a_pos)
        gl.enableVertexAttribArray(tex2d_gl_program.a_uv)
        gl.vertexAttribPointer(tex2d_gl_program.a_pos, 2, WebGLRenderingContext.FLOAT, false, 4 * 4, 0)
        gl.vertexAttribPointer(tex2d_gl_program.a_uv, 2, WebGLRenderingContext.FLOAT, false, 4 * 4, 2 * 4)

        for (command in commands) {
            gl.uniformMatrix4fv(tex2d_gl_program.model, false, command.matrix)
            gl.uniform1f(tex2d_gl_program.z, 0.01f * command.layer)
            gl.uniform2i(tex2d_gl_program.offset, command.offset[0], command.offset[1])

            val texture = get_or_load_server_texture(command.server_texture_id, WebGLRenderingContext.NEAREST)
            gl.uniform2f(tex2d_gl_program.size, texture.w * command.size, texture.h * command.size)
            gl.uniform4fv(tex2d_gl_program.color, command.color)

            gl.bindTexture(WebGLRenderingContext.TEXTURE_2D, texture.texture)

            gl.drawArrays(WebGLRenderingContext.TRIANGLE_STRIP, 0, 4)
        }
    }


    private fun load_texture2d_shader(gl: WebGL2RenderingContext): Texture2dShader {
        val vs = gl.createShader(WebGLRenderingContext.VERTEX_SHADER).apply {
            gl.shaderSource(this, """#version 300 es

layout (location = 0) in vec2 Position;
layout (location = 1) in vec2 aTexCoord;

uniform mat4 model;
uniform mat4 projection;
uniform vec2 size;
uniform ivec2 offset;
uniform float z;

out vec2 tex_coord;

void main() {
    vec2 pos = vec2(Position.x * size.x, Position.y * size.y);
    pos.x += float(offset.x);
    pos.y += float(offset.y);

    gl_Position = projection * model * vec4(pos.xy, z, 1.0);
    tex_coord = aTexCoord;
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

out vec4 out_color;

in vec2 tex_coord;

uniform vec4 color;
uniform sampler2D model_texture;


void main() {
    vec4 tex = texture(model_texture, tex_coord);
    if (tex.a == 0.0) {
        discard;
    } else {
        out_color = tex * color;
    }

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
        return Texture2dShader(program = program!!,
                               projection_mat = gl.getUniformLocation(program, "projection")!!,
                               model = gl.getUniformLocation(program, "model")!!,
                               z = gl.getUniformLocation(program, "z")!!,
                               offset = gl.getUniformLocation(program, "offset")!!,
                               size = gl.getUniformLocation(program, "size")!!,
                               color = gl.getUniformLocation(program, "color")!!,
                               a_pos = gl.getAttribLocation(program, "Position"),
                               a_uv = gl.getAttribLocation(program, "aTexCoord"),
                               texture = gl.getUniformLocation(program, "model_texture")!!)
    }

    private data class Texture2dShader(val program: WebGLProgram,
                                       val projection_mat: WebGLUniformLocation,
                                       val texture: WebGLUniformLocation,
                                       val model: WebGLUniformLocation,
                                       val z: WebGLUniformLocation,
                                       val offset: WebGLUniformLocation,
                                       val size: WebGLUniformLocation,
                                       val color: WebGLUniformLocation,
                                       val a_pos: Int,
                                       val a_uv: Int)
}