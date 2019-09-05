package rustarok.render

import org.khronos.webgl.*
import rustarok.PROJECTION_MATRIX
import rustarok.RenderCommand
import rustarok.VIEW_MATRIX
import rustarok.WebGL2RenderingContext

class Trimesh3dRenderer(gl: WebGL2RenderingContext) {

    private val CIRCLE_VERTEX_COUNT = 32
    private val trimesh_3d_shader = load_shader(gl)
    private val circle_buffer = create_circle_vertex_buffer(gl, 1f, CIRCLE_VERTEX_COUNT)
    private val centered_rectangle_vao = create_centered_rectangle_buffer(gl)


    fun render_circle(gl: WebGL2RenderingContext, command: RenderCommand.Circle3D) {
        gl.useProgram(trimesh_3d_shader.program)
        gl.uniformMatrix4fv(trimesh_3d_shader.projection_mat, false, PROJECTION_MATRIX)
        gl.uniformMatrix4fv(trimesh_3d_shader.view_mat, false, VIEW_MATRIX)


        gl.uniformMatrix4fv(trimesh_3d_shader.model_mat, false, command.matrix)

        gl.uniform2f(trimesh_3d_shader.size, command.radius * 2f, command.radius * 2f)
        gl.uniform4fv(trimesh_3d_shader.color, command.color)

        gl.bindBuffer(WebGLRenderingContext.ARRAY_BUFFER, circle_buffer)
        gl.enableVertexAttribArray(trimesh_3d_shader.a_pos)
        gl.vertexAttribPointer(trimesh_3d_shader.a_pos, 3, WebGLRenderingContext.FLOAT, false, 3 * 4, 0)
        gl.drawArrays(WebGLRenderingContext.LINE_LOOP, 0, CIRCLE_VERTEX_COUNT)
    }

    fun render_rectangle(gl: WebGL2RenderingContext, command: RenderCommand.Rectangle3D) {
        gl.useProgram(trimesh_3d_shader.program)
        gl.uniformMatrix4fv(trimesh_3d_shader.projection_mat, false, PROJECTION_MATRIX)
        gl.uniformMatrix4fv(trimesh_3d_shader.view_mat, false, VIEW_MATRIX)


        gl.uniformMatrix4fv(trimesh_3d_shader.model_mat, false, command.matrix)

        gl.uniform2f(trimesh_3d_shader.size, command.w, command.h)
        gl.uniform4fv(trimesh_3d_shader.color, command.color)

        gl.bindBuffer(WebGLRenderingContext.ARRAY_BUFFER, centered_rectangle_vao)
        gl.enableVertexAttribArray(trimesh_3d_shader.a_pos)
        gl.vertexAttribPointer(trimesh_3d_shader.a_pos, 3, WebGLRenderingContext.FLOAT, false, 3 * 4, 0)
        gl.drawArrays(WebGLRenderingContext.LINE_LOOP, 0, 4)
    }

    private fun create_centered_rectangle_buffer(gl: WebGL2RenderingContext): WebGLBuffer {
        val buffer = gl.createBuffer()!!
        gl.bindBuffer(WebGLRenderingContext.ARRAY_BUFFER, buffer)
        gl.bufferData(WebGLRenderingContext.ARRAY_BUFFER,
                      Float32Array(arrayOf(
                              -0.5f, 0.0f, -0.5f,
                              -0.5f, 0.0f, 0.5f,
                              0.5f, 0.0f, 0.5f,
                              0.5f, 0.0f, -0.5f
                      )),
                      WebGLRenderingContext.STATIC_DRAW)
        return buffer
    }


    private fun create_circle_vertex_buffer(gl: WebGL2RenderingContext,
                                    diameter: Float, nsubdivs: Int): WebGLBuffer {
        val two_pi = kotlin.math.PI.toFloat() * 2.0f;
        val dtheta = two_pi / nsubdivs;
        val pts = Array(nsubdivs * 3) { 0f }
        val radius = diameter / 2f

        var curr_theta = 0.0
        var i = 0
        while (i < nsubdivs * 3) {
            pts[i] = kotlin.math.cos(curr_theta).toFloat() * radius
            pts[i + 1] = 0f
            pts[i + 2] = kotlin.math.sin(curr_theta).toFloat() * radius
            i += 3
            curr_theta += dtheta
        }

        val buffer = gl.createBuffer()!!
        gl.bindBuffer(WebGLRenderingContext.ARRAY_BUFFER, buffer)
        gl.bufferData(WebGLRenderingContext.ARRAY_BUFFER,
                      Float32Array(pts),
                      WebGLRenderingContext.STATIC_DRAW)
        return buffer
    }

    private fun load_shader(gl: WebGL2RenderingContext): Trimesh3dShader {
        val vs = gl.createShader(WebGLRenderingContext.VERTEX_SHADER).apply {
            gl.shaderSource(this, """#version 300 es

layout (location = 0) in vec3 Position;

uniform mat4 view;
uniform mat4 model;
uniform mat4 projection;
uniform vec2 size;

void main() {
    vec4 pos = vec4(Position.x * size.x, Position.y, Position.z * size.y, 1.0);
    gl_Position = projection * view * model * pos;
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
        return Trimesh3dShader(program = program!!,
                               projection_mat = gl.getUniformLocation(program, "projection")!!,
                               view_mat = gl.getUniformLocation(program, "view")!!,
                               model_mat = gl.getUniformLocation(program, "model")!!,
                               size = gl.getUniformLocation(program, "size")!!,
                               color = gl.getUniformLocation(program, "color")!!,
                               a_pos = gl.getAttribLocation(program, "Position"))
    }

    private data class Trimesh3dShader(val program: WebGLProgram,
                               val projection_mat: WebGLUniformLocation,
                               val model_mat: WebGLUniformLocation,
                               val view_mat: WebGLUniformLocation,
                               val size: WebGLUniformLocation,
                               val color: WebGLUniformLocation,
                               val a_pos: Int)
}