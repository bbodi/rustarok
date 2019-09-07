package rustarok.render

import org.khronos.webgl.*
import rustarok.*

class Trimesh2dRenderer(gl: WebGL2RenderingContext) {

    private val trimesh_2d_shader = load_shader(gl)
    private val circle_buffers = create_partial_circle_vertex_buffers(gl)

    fun render_rectangles(gl: WebGL2RenderingContext,
                          commands: List<RenderCommand.Rectangle2D>,
                          sprite_vertex_buffer: WebGLBuffer) {
        gl.useProgram(trimesh_2d_shader.program)
        gl.uniformMatrix4fv(trimesh_2d_shader.projection_mat, false, ORTHO_MATRIX)

        gl.bindBuffer(WebGLRenderingContext.ARRAY_BUFFER, sprite_vertex_buffer)
        gl.enableVertexAttribArray(trimesh_2d_shader.a_pos)
        gl.vertexAttribPointer(trimesh_2d_shader.a_pos, 2, WebGLRenderingContext.FLOAT, false, 4 * 4, 0)

        for (command in commands) {
            gl.uniformMatrix4fv(trimesh_2d_shader.model_mat, false, command.matrix)

            gl.uniform2f(trimesh_2d_shader.size, command.w.toFloat(), command.h.toFloat())
            gl.uniform4fv(trimesh_2d_shader.color, command.color)
            gl.uniform1f(trimesh_2d_shader.z, 0.01f * command.layer)

            gl.drawArrays(WebGLRenderingContext.TRIANGLE_STRIP, 0, 4)
        }
    }


    fun render_partial_circles(gl: WebGL2RenderingContext, commands: List<RenderCommand.PartialCircle2D>) {
        gl.useProgram(trimesh_2d_shader.program)
        gl.uniformMatrix4fv(trimesh_2d_shader.projection_mat, false, ORTHO_MATRIX)
        for (command in commands) {
            gl.uniformMatrix4fv(trimesh_2d_shader.model_mat, false, command.matrix)

            gl.uniform2f(trimesh_2d_shader.size, 1f, 1f)
            gl.uniform4fv(trimesh_2d_shader.color, command.color)
            gl.uniform1f(trimesh_2d_shader.z, 0.01f * command.layer)

            gl.bindBuffer(WebGLRenderingContext.ARRAY_BUFFER, circle_buffers[command.index])
            gl.enableVertexAttribArray(trimesh_2d_shader.a_pos)
            gl.vertexAttribPointer(trimesh_2d_shader.a_pos, 2, WebGLRenderingContext.FLOAT, false, 2 * 4, 0)
            gl.drawArrays(WebGLRenderingContext.LINE_STRIP, 0, command.index + 1)
        }
    }


    private fun create_partial_circle_vertex_buffers(gl: WebGL2RenderingContext): Array<WebGLBuffer> {
        return (1..100).map {
            create_partial_circle_vertex_buffer(gl, it)
        }.toTypedArray()
    }

    private fun create_partial_circle_vertex_buffer(gl: WebGL2RenderingContext,
                                                    percentage: Int): WebGLBuffer {
        val two_pi = kotlin.math.PI.toFloat() * 2.0f;
        val dtheta = two_pi / 100;
        val pts = Array(percentage * 2) { 0f }
        val radius = 12f

        var curr_theta = 0.0
        var i = 0
        while (i < (percentage) * 2) {
            pts[i] = kotlin.math.cos(curr_theta).toFloat() * radius
            pts[i + 1] = kotlin.math.sin(curr_theta).toFloat() * radius
            i += 2
            curr_theta += dtheta
        }

        val buffer = gl.createBuffer()!!
        gl.bindBuffer(WebGLRenderingContext.ARRAY_BUFFER, buffer)
        gl.bufferData(WebGLRenderingContext.ARRAY_BUFFER,
                      Float32Array(pts),
                      WebGLRenderingContext.STATIC_DRAW)
        return buffer
    }

    private fun load_shader(gl: WebGL2RenderingContext): Trimesh2dShader {
        val vs = gl.createShader(WebGLRenderingContext.VERTEX_SHADER).apply {
            gl.shaderSource(this, """#version 300 es

layout (location = 0) in vec2 Position;

uniform mat4 model;
uniform mat4 projection;
uniform vec2 size;
uniform float z;

void main() {
    vec4 pos = vec4(Position.x * size.x, Position.y * size.y, z, 1.0);
    gl_Position = projection * model * pos;
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
uniform vec4 color;

void main() {
    out_color = color;
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
        return Trimesh2dShader(program = program!!,
                               projection_mat = gl.getUniformLocation(program, "projection")!!,
                               model_mat = gl.getUniformLocation(program, "model")!!,
                               size = gl.getUniformLocation(program, "size")!!,
                               color = gl.getUniformLocation(program, "color")!!,
                               z = gl.getUniformLocation(program, "z")!!,
                               a_pos = gl.getAttribLocation(program, "Position"))
    }

    private data class Trimesh2dShader(val program: WebGLProgram,
                                       val projection_mat: WebGLUniformLocation,
                                       val model_mat: WebGLUniformLocation,
                                       val size: WebGLUniformLocation,
                                       val color: WebGLUniformLocation,
                                       val z: WebGLUniformLocation,
                                       val a_pos: Int)
}