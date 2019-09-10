package rustarok.render

import org.khronos.webgl.*
import rustarok.*

class Sprite3dRenderer(gl: WebGL2RenderingContext) {

    private val sprite_gl_program = load_sprite_shader(gl)
    private val single_digit_width = 10.0f
    private val texture_width = single_digit_width * 10.0f
    private val single_digit_u_coord = single_digit_width / texture_width
    private val texture_u_coords = Array(10) { i ->
        single_digit_u_coord * 1.0f * i
    }

    fun render_sprites(gl: WebGL2RenderingContext,
                       sprite_render_commands: List<RenderCommand.Sprite3D>,
                       centered_sprite_vertex_buffer: WebGLBuffer) {
        gl.useProgram(sprite_gl_program.program)
        gl.activeTexture(WebGLRenderingContext.TEXTURE0)
        gl.uniform1i(sprite_gl_program.texture, 0)
        gl.uniformMatrix4fv(sprite_gl_program.projection_mat, false, PROJECTION_MATRIX)

        gl.bindBuffer(WebGLRenderingContext.ARRAY_BUFFER, centered_sprite_vertex_buffer)
        gl.enableVertexAttribArray(sprite_gl_program.a_pos)
        gl.enableVertexAttribArray(sprite_gl_program.a_uv)
        gl.vertexAttribPointer(sprite_gl_program.a_pos, 2, WebGLRenderingContext.FLOAT, false, 4 * 4, 0)
        gl.vertexAttribPointer(sprite_gl_program.a_uv, 2, WebGLRenderingContext.FLOAT, false, 4 * 4, 2 * 4)

        gl.uniformMatrix4fv(sprite_gl_program.view_mat, false, VIEW_MATRIX)
        for (command in sprite_render_commands) {
            gl.uniform4fv(sprite_gl_program.color, command.color)
            // TODO
            //gl.uniformMatrix4fv(sprite_gl_program.model_mat, false, command.matrix)
            gl.uniform2fv(sprite_gl_program.offset, command.offset)
            val texture = get_or_load_server_texture(command.server_texture_id, WebGLRenderingContext.NEAREST)
            gl.bindTexture(WebGLRenderingContext.TEXTURE_2D, texture.texture)
            val w = if (command.is_vertically_flipped) -texture.w else texture.w
            gl.uniform2fv(sprite_gl_program.size,
                          arrayOf(w * ONE_SPRITE_PIXEL_SIZE_IN_3D * command.size,
                                  texture.h * ONE_SPRITE_PIXEL_SIZE_IN_3D * command.size))

            gl.drawArrays(WebGLRenderingContext.TRIANGLE_STRIP, 0, 4)
        }
    }

    fun render_numbers(gl: WebGL2RenderingContext, commands: List<RenderCommand.Number3D>) {
        gl.disable(WebGLRenderingContext.DEPTH_TEST)
        gl.bindTexture(WebGLRenderingContext.TEXTURE_2D,
                       get_or_load_server_texture(path_to_server_gl_indices["assets\\damage.bmp"]!!.gl_textures[0].server_gl_index,
                                                  WebGLRenderingContext.NEAREST).texture
        )
        for (command in commands) {
            gl.uniform4fv(sprite_gl_program.color, command.color)
            // TODO
//            gl.uniformMatrix4fv(sprite_gl_program.model_mat, false, command.matrix)
            gl.uniform2fv(sprite_gl_program.offset, Float32Array(2));
            gl.uniform2fv(sprite_gl_program.size, arrayOf(command.size, command.size))

            val (buffer, vertex_count) = this.create_number_vertex_array(gl, command.value)

            gl.bindBuffer(WebGLRenderingContext.ARRAY_BUFFER, buffer)
            gl.enableVertexAttribArray(sprite_gl_program.a_pos)
            gl.enableVertexAttribArray(sprite_gl_program.a_uv)
            gl.vertexAttribPointer(sprite_gl_program.a_pos, 2, WebGLRenderingContext.FLOAT, false, 4 * 4, 0)
            gl.vertexAttribPointer(sprite_gl_program.a_uv, 2, WebGLRenderingContext.FLOAT, false, 4 * 4, 2 * 4)

            gl.drawArrays(WebGLRenderingContext.TRIANGLES, 0, vertex_count)

            gl.deleteBuffer(buffer)
        }
        gl.enable(WebGLRenderingContext.DEPTH_TEST)
    }

    private fun create_number_vertex_array(gl: WebGL2RenderingContext, value: Int): Pair<WebGLBuffer, Int> {
        val digits = value.toString()
        var width = 0.0f
        val vertices = Float32Array(digits.length * 6 * 4)
        var offset = 0
        for (digit in digits) {
            val digit = digit.toInt() - 48
            vertices.set(arrayOf(
                    width - 0.5f, 0.5f, this.texture_u_coords[digit], 0.0f,
                    width + 0.5f, 0.5f, this.texture_u_coords[digit] + this.single_digit_u_coord, 0.0f,
                    width - 0.5f, -0.5f, this.texture_u_coords[digit], 1.0f,
                    width + 0.5f, 0.5f, this.texture_u_coords[digit] + this.single_digit_u_coord, 0.0f,
                    width - 0.5f, -0.5f, this.texture_u_coords[digit], 1.0f,
                    width + 0.5f, -0.5f, this.texture_u_coords[digit] + this.single_digit_u_coord, 1.0f
            ), offset)

            offset += 6 * 4
            width += 1.0f
        }
        val buffer = gl.createBuffer()!!
        gl.bindBuffer(WebGLRenderingContext.ARRAY_BUFFER, buffer)
        gl.bufferData(WebGLRenderingContext.ARRAY_BUFFER,
                      vertices,
                      WebGLRenderingContext.STATIC_DRAW)
        return buffer to digits.length * 6
    }


    private fun load_sprite_shader(gl: WebGL2RenderingContext): SpriteShader {
        val vs = gl.createShader(WebGLRenderingContext.VERTEX_SHADER).apply {
            gl.shaderSource(this, """#version 300 es
layout (location = 0) in vec2 Position;
layout (location = 1) in vec2 aTexCoord;

uniform mat4 view;
uniform mat4 model;
uniform mat4 projection;
uniform vec2 size;
uniform vec2 offset;

out vec2 tex_coord;

void main() {
    vec4 pos = vec4(Position.x * size.x, Position.y * size.y, 0.0, 1.0);
    pos.x += offset.x;
    pos.y -= offset.y;
    mat4 model_view = view * model;
    model_view[0][0] = 1.0;
    model_view[0][1] = 0.0;
    model_view[0][2] = 0.0;

//    if (spherical == 1) {
        // Second colunm.
        model_view[1][0] = 0.0;
        model_view[1][1] = 1.0;
        model_view[1][2] = 0.0;
//    }

    // Thrid colunm.
    model_view[2][0] = 0.0;
    model_view[2][1] = 0.0;
    model_view[2][2] = 1.0;

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

in vec2 tex_coord;

uniform vec4 color;
uniform sampler2D model_texture;

out vec4 out_color;

void main() {
    vec4 texture = texture(model_texture, tex_coord);
    if (texture.a == 0.0 || color.a == 0.0) {
        discard;
    } else {
        out_color = texture * color;
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
        return SpriteShader(program = program!!,
                            projection_mat = gl.getUniformLocation(program, "projection")!!,
                            view_mat = gl.getUniformLocation(program, "view")!!,
                            texture = gl.getUniformLocation(program, "model_texture")!!,
                            size = gl.getUniformLocation(program, "size")!!,
                            model_mat = gl.getUniformLocation(program, "model")!!,
                            color = gl.getUniformLocation(program, "color")!!,
                            a_pos = gl.getAttribLocation(program, "Position"),
                            a_uv = gl.getAttribLocation(program, "aTexCoord"),
                            offset = gl.getUniformLocation(program, "offset")!!)
    }

    private data class SpriteShader(val program: WebGLProgram,
                                    val projection_mat: WebGLUniformLocation,
                                    val view_mat: WebGLUniformLocation,
                                    val texture: WebGLUniformLocation,
                                    val size: WebGLUniformLocation,
                                    val model_mat: WebGLUniformLocation,
                                    val color: WebGLUniformLocation,
                                    val offset: WebGLUniformLocation,
                                    val a_pos: Int,
                                    val a_uv: Int)

}