package rustarok.render

import org.khronos.webgl.*
import rustarok.*

fun render_model(gl: WebGL2RenderingContext, command: RenderCommand.Model3D,
                 ground_command: RenderCommand.Ground3D, model_gl_program: ModelShader,
                 models: Array<ModelData>) {
    gl.useProgram(model_gl_program.program)
    gl.uniformMatrix4fv(model_gl_program.projection_mat, false, PROJECTION_MATRIX)
    gl.uniformMatrix4fv(model_gl_program.view, false, VIEW_MATRIX)
    gl.uniformMatrix3fv(model_gl_program.normal_matrix, false, NORMAL_MATRIX)

    gl.uniform3fv(model_gl_program.light_dir, ground_command.light_dir)
    gl.uniform3fv(model_gl_program.light_ambient, ground_command.light_ambient)
    gl.uniform3fv(model_gl_program.light_diffuse, ground_command.light_diffuse)
    gl.uniform1f(model_gl_program.light_opacity, ground_command.light_opacity)

    gl.activeTexture(WebGLRenderingContext.TEXTURE0)
    gl.uniform1i(model_gl_program.model_texture, 0)

    val model = models[command.model_index]
    for (node in model.nodes) {
        gl.uniformMatrix4fv(model_gl_program.model, false, command.matrix)
        gl.uniform1f(model_gl_program.alpha, command.alpha)

        gl.bindBuffer(WebGLRenderingContext.ARRAY_BUFFER, node.buffer)
        gl.enableVertexAttribArray(model_gl_program.a_pos)
        gl.enableVertexAttribArray(model_gl_program.a_uv)
        gl.enableVertexAttribArray(model_gl_program.a_vertex_normal)

        gl.vertexAttribPointer(model_gl_program.a_pos, 3, WebGLRenderingContext.FLOAT, false, 8 * 4, 0)
        gl.vertexAttribPointer(model_gl_program.a_vertex_normal, 3, WebGLRenderingContext.FLOAT, false, 8 * 4, 3 * 4)
        gl.vertexAttribPointer(model_gl_program.a_uv, 2, WebGLRenderingContext.FLOAT, false, 8 * 4, 6 * 4)

        gl.bindTexture(WebGLRenderingContext.TEXTURE_2D, get_or_load_server_texture(node.server_texture_id, WebGLRenderingContext.NEAREST))
        gl.drawArrays(WebGLRenderingContext.TRIANGLES, 0, node.vertex_count)
    }
}


fun load_model_shader(gl: WebGL2RenderingContext): ModelShader {
    val vs = gl.createShader(WebGLRenderingContext.VERTEX_SHADER).apply {
        gl.shaderSource(this, """#version 300 es

layout (location = 0) in vec3 Position;
layout (location = 1) in vec3 aVertexNormal;
layout (location = 2) in vec2 aTexCoord;

uniform mat4 view;
uniform mat4 model;
uniform mat4 projection;
uniform mat3 normal_matrix;

uniform vec3 light_dir;

out vec2 tex_coord;
out float vLightWeighting;

void main() {
    mat4 model_view = view * model;
    gl_Position = projection * model_view * vec4(Position, 1.0);
    tex_coord = aTexCoord;

    vec4 lDirection  = model_view * vec4( light_dir, 0.0);
    vec3 dirVector   = normalize(lDirection.xyz);
    float dotProduct = dot( normal_matrix * aVertexNormal, dirVector );
    vLightWeighting  = max( dotProduct, 0.5 );
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

uniform sampler2D model_texture;

uniform vec3 light_ambient;
uniform vec3 light_diffuse;
uniform float light_opacity;

in float vLightWeighting;
uniform float alpha;

void main() {
    vec4 tex = texture(model_texture, tex_coord);
    if (tex.a == 0.0) {
        discard;
    }
    vec3 Ambient    = light_ambient * light_opacity;
    vec3 Diffuse    = light_diffuse * vLightWeighting;
    vec4 LightColor = vec4((Ambient + Diffuse), 1.0);
    Color = tex * clamp(LightColor, 0.0, 1.0);
    Color.a *= alpha;

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
    return ModelShader(program = program!!,
                       projection_mat = gl.getUniformLocation(program, "projection")!!,
                       normal_matrix = gl.getUniformLocation(program, "normal_matrix")!!,
                       light_dir = gl.getUniformLocation(program, "light_dir")!!,
                       model = gl.getUniformLocation(program, "model")!!,
                       view = gl.getUniformLocation(program, "view")!!,
                       a_pos = gl.getAttribLocation(program, "Position"),
                       a_vertex_normal = gl.getAttribLocation(program, "aVertexNormal"),
                       a_uv = gl.getAttribLocation(program, "aTexCoord"),
                       alpha = gl.getUniformLocation(program, "alpha")!!,
                       model_texture = gl.getUniformLocation(program, "model_texture")!!,
                       light_ambient = gl.getUniformLocation(program, "light_ambient")!!,
                       light_diffuse = gl.getUniformLocation(program, "light_diffuse")!!,
                       light_opacity = gl.getUniformLocation(program, "light_opacity")!!)
}

data class ModelShader(val program: WebGLProgram,
                       val projection_mat: WebGLUniformLocation,
                       val model: WebGLUniformLocation,
                       val view: WebGLUniformLocation,
                       val a_pos: Int,
                       val a_uv: Int,
                       val alpha: WebGLUniformLocation,
                       val normal_matrix: WebGLUniformLocation,
                       val light_dir: WebGLUniformLocation,
                       val a_vertex_normal: Int,
                       val model_texture: WebGLUniformLocation,
                       val light_ambient: WebGLUniformLocation,
                       val light_diffuse: WebGLUniformLocation,
                       val light_opacity: WebGLUniformLocation)