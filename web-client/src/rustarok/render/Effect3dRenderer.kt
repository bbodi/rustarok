package rustarok.render

import org.khronos.webgl.*
import rustarok.*
import kotlin.math.max

class Effect3dRenderer(gl: WebGL2RenderingContext) {

    private val effect_gl_program = load_effect_shader(gl)
    private val dynamic_buffer = create_dynamic_buffer(gl)
    private val float_buffer = Float32Array(arrayOf(
            0f, 0f, 0f, 0f,
            0f, 0f, 1f, 0f,
            0f, 0f, 0f, 1f,
            0f, 0f, 1f, 1f
    ))

    fun render_effects(gl: WebGL2RenderingContext,
                       sprite_render_commands: List<RenderCommand.Effect3D>,
                       effects: Array<StrFile>) {
        if (sprite_render_commands.isEmpty()) {
            return
        }
        gl.disable(WebGLRenderingContext.DEPTH_TEST)

        gl.useProgram(effect_gl_program.program)
        gl.activeTexture(WebGLRenderingContext.TEXTURE0)
        gl.uniform1i(effect_gl_program.texture, 0)
        gl.uniformMatrix4fv(effect_gl_program.projection_mat, false, PROJECTION_MATRIX)
        gl.uniformMatrix4fv(effect_gl_program.view_mat, false, VIEW_MATRIX)

        gl.bindBuffer(WebGLRenderingContext.ARRAY_BUFFER, dynamic_buffer)
        gl.enableVertexAttribArray(effect_gl_program.a_pos)
        gl.enableVertexAttribArray(effect_gl_program.a_uv)
        gl.vertexAttribPointer(effect_gl_program.a_pos,
                               2,
                               WebGLRenderingContext.FLOAT,
                               false,
                               4 * 4,
                               0)
        gl.vertexAttribPointer(effect_gl_program.a_uv,
                               2,
                               WebGLRenderingContext.FLOAT,
                               false,
                               4 * 4,
                               2 * 4)


        for (command in sprite_render_commands) {
            val str_file = effects[command.effect_id]
            for (layer in str_file.layers) {
                val data = calc(gl, dynamic_buffer, layer, command.key_index) ?: continue
                gl.blendFunc(data.src_alpha, data.dst_alpha);

                gl.uniform4fv(effect_gl_program.color, data.color)

                val matrix = Matrix()
                matrix.set_translation(command.x, 0f, command.y)
                matrix.rotate_around_z_mut(-data.angle)
                gl.uniformMatrix4fv(effect_gl_program.model_mat, false, matrix.buffer)

                gl.uniform2fv(effect_gl_program.offset, data.offset)
                val server_texture_index = str_file.server_texture_indices[data.texture_index]
                val texture = get_or_load_server_texture(server_texture_index,
                                                         WebGLRenderingContext.NEAREST)
                gl.bindTexture(WebGLRenderingContext.TEXTURE_2D, texture.texture)

                gl.drawArrays(WebGLRenderingContext.TRIANGLE_STRIP, 0, 4)
            }
        }

        gl.blendFunc(WebGLRenderingContext.SRC_ALPHA, WebGLRenderingContext.ONE_MINUS_SRC_ALPHA);
        gl.enable(WebGLRenderingContext.DEPTH_TEST)
    }

    private class EffectDrawing(val color: Float32Array,
                                val offset: Array<Float>,
                                val src_alpha: Int,
                                val dst_alpha: Int,
                                val texture_index: Int,
                                val angle: Float)

    private fun calc(gl: WebGL2RenderingContext,
                     dynamic_buffer: WebGLBuffer,
                     layer_frames: StrLayer,
                     key_index: Int): EffectDrawing? {
        var from_id: Int? = null
        var to_id: Int? = null
        var last_source_id = 0
        var last_frame_id = 0
        for ((i, key_frame) in layer_frames.key_frames.withIndex()) {
            if (key_frame.frame <= key_index) {
                when (key_frame.typ) {
                    0 -> from_id = i
                    else -> to_id = i
                }
            }
            last_frame_id = max(last_frame_id, key_frame.frame)
            if (key_frame.typ == 0) {
                last_source_id = max(last_source_id, key_frame.frame)
            }
        }
        if (from_id == null || to_id == null || last_frame_id < key_index) {
            return null
        }
        if (from_id >= layer_frames.key_frames.size || to_id >= layer_frames.key_frames.size) {
            return null
        }
        val from_frame = layer_frames.key_frames[from_id]
        val to_frame = layer_frames.key_frames[to_id]


        var color = from_frame.color
        var pos = arrayOf(from_frame.pos_x, from_frame.pos_y)
        var xy = from_frame.xy
        var angle = from_frame.angle
        if (to_id != from_id + 1 || to_frame.frame != from_frame.frame) {
            if (last_source_id <= from_frame.frame) {
                return null
            }
            color = from_frame.color
            pos = arrayOf(from_frame.pos_x, from_frame.pos_y)
            xy = from_frame.xy
            angle = from_frame.angle
        } else {
            val delta = key_index - from_frame.frame;

            // morphing
            color = Float32Array(arrayOf(
                    (from_frame.color[0] + to_frame.color[0] * delta),
                    (from_frame.color[1] + to_frame.color[1] * delta),
                    (from_frame.color[2] + to_frame.color[2] * delta),
                    (from_frame.color[3] + to_frame.color[3] * delta)
            ))

            xy = arrayOf(
                    from_frame.xy[0] + to_frame.xy[0] * delta,
                    from_frame.xy[1] + to_frame.xy[1] * delta,
                    from_frame.xy[2] + to_frame.xy[2] * delta,
                    from_frame.xy[3] + to_frame.xy[3] * delta,
                    from_frame.xy[4] + to_frame.xy[4] * delta,
                    from_frame.xy[5] + to_frame.xy[5] * delta,
                    from_frame.xy[6] + to_frame.xy[6] * delta,
                    from_frame.xy[7] + to_frame.xy[7] * delta
            )
            angle = from_frame.angle + to_frame.angle * delta;
            pos = arrayOf(
                    from_frame.pos_x + to_frame.pos_x * delta,
                    from_frame.pos_y + to_frame.pos_y * delta
            )
        }
        val offset = arrayOf(pos[0] - 320f, pos[1] - 320f)


        // dynamic_buffer is already binded
        float_buffer[0] = xy[0]
        float_buffer[1] = xy[4]
        float_buffer[4] = xy[1]
        float_buffer[5] = xy[5]
        float_buffer[8] = xy[3]
        float_buffer[9] = xy[7]
        float_buffer[12] = xy[2]
        float_buffer[13] = xy[6]
        gl.bufferData(WebGLRenderingContext.ARRAY_BUFFER,
                      float_buffer,
                      WebGLRenderingContext.DYNAMIC_DRAW)
        return EffectDrawing(color,
                             offset,
                             from_frame.src_alpha,
                             from_frame.dst_alpha,
                             from_frame.texture_index,
                             angle)

    }

    private fun create_dynamic_buffer(gl: WebGL2RenderingContext): WebGLBuffer {
        val buffer = gl.createBuffer()!!
        gl.bindBuffer(WebGLRenderingContext.ARRAY_BUFFER, buffer)
        gl.bufferData(WebGLRenderingContext.ARRAY_BUFFER,
                      Float32Array(arrayOf(
                              0f, 0f, 0f, 0f,
                              0f, 0f, 0f, 0f,
                              0f, 0f, 0f, 0f,
                              0f, 0f, 0f, 0f
                      )),
                      WebGLRenderingContext.DYNAMIC_DRAW)
        return buffer
    }

    private fun load_effect_shader(gl: WebGL2RenderingContext): EffectShader {
        val vs = gl.createShader(WebGLRenderingContext.VERTEX_SHADER).apply {
            gl.shaderSource(this, """#version 300 es

layout (location = 0) in vec2 vertex_pos;
layout (location = 1) in vec2 aTexCoord;

uniform mat4 view;
uniform mat4 model;
uniform mat4 projection;
uniform vec2 offset;

const float ONE_SPRITE_PIXEL_SIZE_IN_3D = 1.0 / 35.0;

out vec2 tex_coord;

void main() {
    vec4 pos = vec4(vertex_pos.x * ONE_SPRITE_PIXEL_SIZE_IN_3D,
                    -vertex_pos.y * ONE_SPRITE_PIXEL_SIZE_IN_3D,
                    0.0, 1.0);
    pos.x += offset.x * ONE_SPRITE_PIXEL_SIZE_IN_3D;
    pos.y -= offset.y * ONE_SPRITE_PIXEL_SIZE_IN_3D;
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

out vec4 out_color;

in vec2 tex_coord;

uniform sampler2D model_texture;
uniform float alpha;
uniform vec4 color;


void main() {
    vec4 tex = texture(model_texture, tex_coord);
    if (tex.a == 0.0) {
        discard;
    } else {
        out_color = tex * color;
        out_color.a = color.a;
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
        return EffectShader(
                program = program!!,
                projection_mat = gl.getUniformLocation(program, "projection")!!,
                view_mat = gl.getUniformLocation(program, "view")!!,
                model_mat = gl.getUniformLocation(program, "model")!!,
                offset = gl.getUniformLocation(program, "offset")!!,
                texture = gl.getUniformLocation(program, "model_texture")!!,
                color = gl.getUniformLocation(program, "color")!!,
                a_pos = gl.getAttribLocation(program, "vertex_pos"),
                a_uv = gl.getAttribLocation(program, "aTexCoord")
        )
    }

    private data class EffectShader(val program: WebGLProgram,
                                    val projection_mat: WebGLUniformLocation,
                                    val view_mat: WebGLUniformLocation,
                                    val texture: WebGLUniformLocation,
                                    val model_mat: WebGLUniformLocation,
                                    val color: WebGLUniformLocation,
                                    val offset: WebGLUniformLocation,
                                    val a_pos: Int,
                                    val a_uv: Int)

}