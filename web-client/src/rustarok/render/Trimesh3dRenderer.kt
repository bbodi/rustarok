package rustarok.render

import org.khronos.webgl.*
import rustarok.*

class Trimesh3dRenderer(gl: WebGL2RenderingContext) {

    private val CIRCLE_VERTEX_COUNT = 32
    private val trimesh_3d_shader = load_shader(gl)
    private val circle_buffer = create_circle_vertex_buffer(gl, 1f, CIRCLE_VERTEX_COUNT)
    private val centered_rectangle_vao = create_centered_rectangle_buffer(gl)
    private val sanctuary_vao = create_sanctuary_vao(gl)

    fun render_all_trimeshes(gl: WebGL2RenderingContext,
                             circle_commands: ArrayList<RenderCommand.Circle3D>,
                             rectangle_commands: ArrayList<RenderCommand.Rectangle3D>,
                             trimesh_commands: ArrayList<RenderCommand.Trimesh3D>) {
        gl.useProgram(trimesh_3d_shader.program)
        gl.uniformMatrix4fv(trimesh_3d_shader.projection_mat, false, PROJECTION_MATRIX)
        gl.uniformMatrix4fv(trimesh_3d_shader.view_mat, false, VIEW_MATRIX)

        gl.bindBuffer(WebGLRenderingContext.ARRAY_BUFFER, circle_buffer)
        enableVertexAttribs(gl, trimesh_3d_shader)
        render_circles(gl, circle_commands)

        gl.bindBuffer(WebGLRenderingContext.ARRAY_BUFFER, centered_rectangle_vao)
        enableVertexAttribs(gl, trimesh_3d_shader)
        render_rectangles(gl, rectangle_commands)

        gl.bindBuffer(WebGLRenderingContext.ARRAY_BUFFER, sanctuary_vao)
        enableVertexAttribs(gl, trimesh_3d_shader)
        render_trimeshes(gl, trimesh_commands)
    }

    private fun enableVertexAttribs(gl: WebGL2RenderingContext,
                                    shader: Trimesh3dShader
    ) {
        gl.enableVertexAttribArray(shader.a_pos)
        gl.vertexAttribPointer(shader.a_pos,
                               3,
                               WebGLRenderingContext.FLOAT,
                               false,
                               7 * 4,
                               0)
        gl.enableVertexAttribArray(shader.a_color)
        gl.vertexAttribPointer(shader.a_color,
                               4,
                               WebGLRenderingContext.FLOAT,
                               false,
                               7 * 4,
                               3 * 4)
    }

    fun render_trimeshes(gl: WebGL2RenderingContext, commands: ArrayList<RenderCommand.Trimesh3D>) {
        for (command in commands) {
            val matrix = Matrix()
            matrix.set_translation(command.x, command.y, command.z)
            gl.uniformMatrix4fv(trimesh_3d_shader.model_mat, false, matrix.buffer)

            gl.uniform3f(trimesh_3d_shader.size, 1f, 1f, 1f)
            gl.uniform4fv(trimesh_3d_shader.color, arrayOf(1f, 1f, 1f, 1f))

            gl.drawArrays(WebGLRenderingContext.TRIANGLES, 0, 630)
        }
    }


    fun render_circles(gl: WebGL2RenderingContext, commands: ArrayList<RenderCommand.Circle3D>) {
        for (command in commands) {
            val matrix = Matrix()
            matrix.set_translation(command.x, command.y, command.z)
            gl.uniformMatrix4fv(trimesh_3d_shader.model_mat, false, matrix.buffer)

            gl.uniform3f(trimesh_3d_shader.size, command.radius * 2f, 1f, command.radius * 2f)
            gl.uniform4fv(trimesh_3d_shader.color, command.color)

            gl.drawArrays(WebGLRenderingContext.LINE_LOOP, 0, CIRCLE_VERTEX_COUNT)
        }
    }

    fun render_rectangles(gl: WebGL2RenderingContext,
                          commands: ArrayList<RenderCommand.Rectangle3D>) {
        for (command in commands) {
            val matrix = Matrix()
            matrix.set_translation(command.x, command.y, command.z)
            matrix.rotate_around_y_mut(command.rotation_rad)
            gl.uniformMatrix4fv(trimesh_3d_shader.model_mat, false, matrix.buffer)

            gl.uniform3f(trimesh_3d_shader.size, command.w, 1f, command.h)
            gl.uniform4fv(trimesh_3d_shader.color, command.color)

            gl.drawArrays(WebGLRenderingContext.LINE_LOOP, 0, 4)
        }
    }

    private fun create_sanctuary_vao(gl: WebGL2RenderingContext): WebGLBuffer {
        val single_cube = arrayOf(
                // Front
                arrayOf(-0.5f, 0.5f, 0.5f, 0.0f),
                arrayOf(-0.5f, -0.5f, 0.5f, 0.7f),
                arrayOf(0.5f, 0.5f, 0.5f, 0.0f),
                arrayOf(0.5f, 0.5f, 0.5f, 0.0f),
                arrayOf(-0.5f, -0.5f, 0.5f, 0.7f),
                arrayOf(0.5f, -0.5f, 0.5f, 0.7f),
                // Right
                arrayOf(0.5f, 0.5f, 0.5f, 0.0f),
                arrayOf(0.5f, -0.5f, 0.5f, 0.7f),
                arrayOf(0.5f, 0.5f, -0.5f, 0.0f),
                arrayOf(0.5f, 0.5f, -0.5f, 0.0f),
                arrayOf(0.5f, -0.5f, 0.5f, 0.7f),
                arrayOf(0.5f, -0.5f, -0.5f, 0.7f),
                // Back
                arrayOf(0.5f, 0.5f, -0.5f, 0.0f),
                arrayOf(0.5f, -0.5f, -0.5f, 0.7f),
                arrayOf(-0.5f, 0.5f, -0.5f, 0.0f),
                arrayOf(-0.5f, 0.5f, -0.5f, 0.0f),
                arrayOf(0.5f, -0.5f, -0.5f, 0.7f),
                arrayOf(-0.5f, -0.5f, -0.5f, 0.7f),
                // Let
                arrayOf(-0.5f, 0.5f, -0.5f, 0.0f),
                arrayOf(-0.5f, -0.5f, -0.5f, 0.7f),
                arrayOf(-0.5f, 0.5f, 0.5f, 0.0f),
                arrayOf(-0.5f, 0.5f, 0.5f, 0.0f),
                arrayOf(-0.5f, -0.5f, -0.5f, 0.7f),
                arrayOf(-0.5f, -0.5f, 0.5f, 0.7f),
                // Bottom
                arrayOf(-0.5f, -0.5f, 0.5f, 0.3f),
                arrayOf(-0.5f, -0.5f, -0.5f, 0.3f),
                arrayOf(0.5f, -0.5f, 0.5f, 0.3f),
                arrayOf(0.5f, -0.5f, 0.5f, 0.3f),
                arrayOf(-0.5f, -0.5f, -0.5f, 0.3f),
                arrayOf(0.5f, -0.5f, -0.5f, 0.3f)
        );
        fun translate(vec: Array<Array<Float>>, x: Double, z: Double): Array<Array<Float>> {
            val x = x.toFloat()
            val z = z.toFloat()
            return vec.map { v ->
                arrayOf(v[0] + x, v[1], v[2] + z, v[3])
            }.toTypedArray()
        }

        val cubes = arrayOf(
                translate(single_cube, -1.0, -2.0),
                translate(single_cube, 0.0, -2.0),
                translate(single_cube, 1.0, -2.0),
                //
                translate(single_cube, -2.0, -1.0),
                translate(single_cube, -1.0, -1.0),
                translate(single_cube, 0.0, -1.0),
                translate(single_cube, 1.0, -1.0),
                translate(single_cube, 2.0, -1.0),
                //
                translate(single_cube, -2.0, 0.0),
                translate(single_cube, -1.0, 0.0),
                translate(single_cube, 0.0, 0.0),
                translate(single_cube, 1.0, 0.0),
                translate(single_cube, 2.0, 0.0),
                //
                translate(single_cube, -2.0, 1.0),
                translate(single_cube, -1.0, 1.0),
                translate(single_cube, 0.0, 1.0),
                translate(single_cube, 1.0, 1.0),
                translate(single_cube, 2.0, 1.0),
                //
                translate(single_cube, -1.0, 2.0),
                translate(single_cube, 0.0, 2.0),
                translate(single_cube, 1.0, 2.0)
        ).flatten().toTypedArray()
        val colored_cubes: Array<Float> = cubes.map { v ->
            arrayOf(
                    v[0] * 1.0f,
                    (v[1] + 0.5f) * 2.0f,
                    v[2] * 1.0f,
                    0.86f,
                    0.99f,
                    0.86f,
                    v[3]
            )
        }.toTypedArray().flatten().toTypedArray()

        val buffer = gl.createBuffer()!!
        gl.bindBuffer(WebGLRenderingContext.ARRAY_BUFFER, buffer)
        gl.bufferData(WebGLRenderingContext.ARRAY_BUFFER,
                      Float32Array(colored_cubes),
                      WebGLRenderingContext.STATIC_DRAW)
        return buffer
    }


    private fun create_centered_rectangle_buffer(gl: WebGL2RenderingContext): WebGLBuffer {
        val buffer = gl.createBuffer()!!
        gl.bindBuffer(WebGLRenderingContext.ARRAY_BUFFER, buffer)
        gl.bufferData(WebGLRenderingContext.ARRAY_BUFFER,
                      Float32Array(arrayOf(
                              -0.5f, 0.0f, -0.5f, 1f, 1f, 1f, 1f,
                              -0.5f, 0.0f, 0.5f, 1f, 1f, 1f, 1f,
                              0.5f, 0.0f, 0.5f, 1f, 1f, 1f, 1f,
                              0.5f, 0.0f, -0.5f, 1f, 1f, 1f, 1f
                      )),
                      WebGLRenderingContext.STATIC_DRAW)
        return buffer
    }


    private fun create_circle_vertex_buffer(gl: WebGL2RenderingContext,
                                            diameter: Float, nsubdivs: Int): WebGLBuffer {
        val two_pi = kotlin.math.PI.toFloat() * 2.0f;
        val dtheta = two_pi / nsubdivs;
        val floats_per_vertex = 7;
        val pts = Array(nsubdivs * floats_per_vertex) { 0f }
        val radius = diameter / 2f

        var curr_theta = 0.0
        var i = 0
        while (i < nsubdivs * floats_per_vertex) {
            pts[i] = kotlin.math.cos(curr_theta).toFloat() * radius
            pts[i + 1] = 0f
            pts[i + 2] = kotlin.math.sin(curr_theta).toFloat() * radius
            pts[i + 3] = 1f
            pts[i + 4] = 1f
            pts[i + 5] = 1f
            pts[i + 6] = 1f
            i += floats_per_vertex
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
layout (location = 1) in vec4 a_color;

out vec4 color;

uniform mat4 view;
uniform mat4 model;
uniform mat4 projection;
uniform vec3 size;
uniform vec4 global_color;

void main() {
    vec4 pos = vec4(Position.x * size.x, Position.y * size.y, Position.z * size.z, 1.0);
    color =  global_color * a_color;
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

in vec4 color;

out vec4 out_color;

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
                               color = gl.getUniformLocation(program, "global_color")!!,
                               a_pos = gl.getAttribLocation(program, "Position"),
                               a_color = gl.getAttribLocation(program, "a_color"))
    }

    private data class Trimesh3dShader(val program: WebGLProgram,
                                       val projection_mat: WebGLUniformLocation,
                                       val model_mat: WebGLUniformLocation,
                                       val view_mat: WebGLUniformLocation,
                                       val size: WebGLUniformLocation,
                                       val color: WebGLUniformLocation,
                                       val a_pos: Int,
                                       val a_color: Int)
}