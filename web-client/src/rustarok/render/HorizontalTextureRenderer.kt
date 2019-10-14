package rustarok.render

import org.khronos.webgl.WebGLBuffer
import org.khronos.webgl.WebGLProgram
import org.khronos.webgl.WebGLRenderingContext
import org.khronos.webgl.WebGLUniformLocation
import rustarok.*

class HorizontalTextureRenderer(gl: WebGL2RenderingContext) {

    private val shader = load_shader(gl)

    fun render(gl: WebGL2RenderingContext,
               commands: ArrayList<RenderCommand.HorizontalTexture3D>,
               centered_sprite_vertex_buffer: WebGLBuffer) {
        gl.useProgram(shader.program)
        gl.uniformMatrix4fv(shader.projection_mat, false, PROJECTION_MATRIX)
        gl.uniformMatrix4fv(shader.view_mat, false, VIEW_MATRIX)
        gl.activeTexture(WebGLRenderingContext.TEXTURE0)
        gl.uniform1i(shader.texture, 0)

        gl.bindBuffer(WebGLRenderingContext.ARRAY_BUFFER, centered_sprite_vertex_buffer)
        enableVertexAttribs(gl, shader)
        for (command in commands) {
            val matrix = Matrix()
            matrix.set_translation(command.x, 0.2f, command.z)
            matrix.rotate_around_y_mut(command.rotation_rad)
            gl.uniformMatrix4fv(shader.model_mat, false, matrix.buffer)

            val texture =
                    get_or_load_server_texture(command.server_texture_id,
                            WebGLRenderingContext.NEAREST)
            val (w, h) = when (command.size) {
                is TextureSize.Fixed -> {
                    command.size.fixed to command.size.fixed
                }
                is TextureSize.Scaled -> {
                    texture.w * command.size.scaled * ONE_SPRITE_PIXEL_SIZE_IN_3D to
                            texture.h * command.size.scaled * ONE_SPRITE_PIXEL_SIZE_IN_3D
                }
            }
            gl.uniform2f(shader.size, w, h)
            gl.uniform4fv(shader.color, command.color)

            gl.bindTexture(WebGLRenderingContext.TEXTURE_2D, texture.texture)
            gl.drawArrays(WebGLRenderingContext.TRIANGLE_STRIP, 0, 4)
        }
    }

    private fun enableVertexAttribs(gl: WebGL2RenderingContext,
                                    shader: HorizTextureShader
    ) {
        gl.enableVertexAttribArray(shader.a_pos)
        gl.vertexAttribPointer(shader.a_pos,
                2,
                WebGLRenderingContext.FLOAT,
                false,
                4 * 4,
                0)
        gl.enableVertexAttribArray(shader.a_uv)
        gl.vertexAttribPointer(shader.a_uv,
                2,
                WebGLRenderingContext.FLOAT,
                false,
                4 * 4,
                2 * 4)
    }


    private fun load_shader(gl: WebGL2RenderingContext): HorizTextureShader {
        val vs = gl.createShader(WebGLRenderingContext.VERTEX_SHADER).apply {
            gl.shaderSource(this, """#version 300 es

layout (location = 0) in vec2 Position;
layout (location = 1) in vec2 aTexCoord;


uniform mat4 view;
uniform mat4 model;
uniform mat4 projection;
uniform vec2 size;

out vec2 tex_coord;

void main() {
    vec4 pos = vec4(Position.x * size.x, 0.0, Position.y * size.y, 1.0);
    mat4 model_view = view * model;

    gl_Position = projection * model_view * pos;
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
    out_color = tex * color;
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
        return HorizTextureShader(program = program!!,
                projection_mat = gl.getUniformLocation(program, "projection")!!,
                view_mat = gl.getUniformLocation(program, "view")!!,
                color = gl.getUniformLocation(program, "color")!!,
                model_mat = gl.getUniformLocation(program, "model")!!,
                size = gl.getUniformLocation(program, "size")!!,
                a_pos = gl.getAttribLocation(program, "Position"),
                texture = gl.getUniformLocation(program, "model_texture")!!,
                a_uv = gl.getAttribLocation(program, "aTexCoord"))
    }

    private data class HorizTextureShader(val program: WebGLProgram,
                                          val projection_mat: WebGLUniformLocation,
                                          val model_mat: WebGLUniformLocation,
                                          val view_mat: WebGLUniformLocation,
                                          val color: WebGLUniformLocation,
                                          val size: WebGLUniformLocation,
                                          val a_pos: Int,
                                          val texture: WebGLUniformLocation,
                                          val a_uv: Int)
}