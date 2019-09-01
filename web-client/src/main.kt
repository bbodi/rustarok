import kotlinx.coroutines.*
import org.khronos.webgl.*
import org.w3c.dom.*
import org.w3c.files.Blob
import org.w3c.files.FileReader
import kotlin.browser.document
import kotlin.browser.window
import kotlin.js.Date
import kotlin.js.Promise

class TextureData(private val native: dynamic) {
    val server_gl_index: Int
        get() = native[0]

    val width: Int
        get() = native[1]

    val height: Int
        get() = native[2]
}

interface DatabaseTextureEntry {
    val gl_textures: Array<Any>
    val hash: String
}

abstract external class WebGL2RenderingContext : WebGLRenderingContext

val server_to_client_gl_indices = hashMapOf<Int, WebGLTexture>()
val path_to_server_gl_indices = hashMapOf<String, DatabaseTextureEntry>()
val server_texture_index_to_path = hashMapOf<Int, Triple<TextureData, String, Int>>() // path, i
var canvas = document.getElementById("main_canvas") as HTMLCanvasElement
var gl = canvas.getContext("webgl2") as WebGL2RenderingContext
var VIDEO_WIDTH = 0
var VIDEO_HEIGHT = 0
var PROJECTION_MATRIX: Float32Array = 0.asDynamic()
var VIEW_MATRIX: Float32Array = 0.asDynamic()
var state = 0

sealed class RenderCommand {
    data class Sprite3D(val server_texture_id: Int,
                        val matrix: Float32Array,
                        val color: Float32Array,
                        val offset: Float32Array,
                        val w: Float,
                        val h: Float) : RenderCommand()
}

val sprite_render_commands = arrayListOf<RenderCommand.Sprite3D>()

val sprite_gl_program = load_sprite_shader(gl)
val sprite_buffer = create_sprite_buffer(gl)

var socket: WebSocket = 0.asDynamic()

fun main() {
    val loc = window.location
    var new_uri = when {
        loc.protocol === "https:" -> "wss:"
        else -> "ws:"
    }
    new_uri += "//" + loc.hostname.ifEmpty { "localhost" }
    new_uri += ":6969"

    socket = WebSocket(new_uri)
    socket.binaryType = BinaryType.ARRAYBUFFER
    socket.onopen = { _ ->

    }


    socket.onmessage = { event ->
        when (state) {
            0 -> {
                state = 1
                val blob = Blob(arrayOf(Uint8Array(event.data as ArrayBuffer)))
                FileReader().apply {
                    this.onload = {
                        val result = JSON.parse<dynamic>(this.result)
                        VIDEO_WIDTH = result.screen_width
                        VIDEO_HEIGHT = result.screen_height
                        canvas.width = VIDEO_WIDTH
                        canvas.height = VIDEO_HEIGHT
                        PROJECTION_MATRIX = Float32Array(result.projection_mat as Array<Float>)

                        gl.viewport(0, 0, VIDEO_WIDTH, VIDEO_HEIGHT)
                        gl.clearColor(0.3f, 0.3f, 0.5f, 1.0f)
                        gl.enable(WebGLRenderingContext.DEPTH_TEST)
                        gl.depthFunc(WebGLRenderingContext.LEQUAL)
                        gl.enable(WebGLRenderingContext.BLEND)
                        gl.blendFunc(WebGLRenderingContext.SRC_ALPHA, WebGLRenderingContext.ONE_MINUS_SRC_ALPHA)
                        gl.lineWidth(2.0f)

                        val texture_db = result.asset_database.texture_db.entries
                        val keys: Array<String> = js("Object").keys(texture_db)
                        val map = hashMapOf<String, DatabaseTextureEntry>()
                        for (key in keys) {
                            val databaseTextureEntry: DatabaseTextureEntry = texture_db[key]
                            map[key] = databaseTextureEntry
                            if (key == "[100, 97, 116, 97, 92, 115, 112, 114, 105, 116, 101, 92, 195, 128, 195, 142, 194, 176, 194, 163, 195, 129, 194, 183, 92, 194, 184, 195, 182, 195, 133, 195, 171, 92, 194, 191, 194, 169, 92, 195, 133, 194, 169, 194, 183, 195, 167, 194, 188, 194, 188, 195, 128, 195, 140, 194, 180, 195, 181, 95, 194, 191, 194, 169]") {
                                js("debugger")
                            }
                            path_to_server_gl_indices[key] = databaseTextureEntry
                            for ((i, glTexture) in databaseTextureEntry.gl_textures.withIndex()) {
                                val glTexture = TextureData(glTexture)
                                server_texture_index_to_path[glTexture.server_gl_index] = Triple(glTexture, key, i)
                            }
                        }
                        GlobalScope.launch {
                            val mismatched_textures = IndexedDb.collect_mismatched_textures(map)
                            console.log(mismatched_textures)
                            socket.send(JSON.stringify(object {
                                val mismatched_textures = mismatched_textures
                            }))
                        }
                    }
                }.readAsText(blob)
            }
            1 -> {
                console.info("Received missing textures")
                GlobalScope.launch {
                    val reader = BufferReader(event.data as ArrayBuffer)
                    if (reader.view.byteLength >= 4 && reader.view.getUint32(reader.offset).asDynamic() == js("0xB16B00B5")) {
                        console.info("DONE")
                        reader.next_f32()
                        state = 2
                        start_frame(socket)
                    } else {
                        while (reader.has_next()) {
                            val path = reader.next_string_with_length()
                            val hash = reader.next_string_with_length()
                            val count = reader.next_u16()
                            if (path == "[100, 97, 116, 97, 92, 115, 112, 114, 105, 116, 101, 92, 195, 128, 195, 142, 194, 176, 194, 163, 195, 129, 194, 183, 92, 194, 184, 195, 182, 195, 133, 195, 171, 92, 194, 191, 194, 169, 92, 195, 133, 194, 169, 194, 183, 195, 167, 194, 188, 194, 188, 195, 128, 195, 140, 194, 180, 195, 181, 95, 194, 191, 194, 169]") {
                                js("debugger")
                            }
                            console.info("Download $path with $count textures")
                            IndexedDb.store_texture_info(path, hash, count)
                            for (i in 0 until count) {
                                val w = reader.next_u16()
                                val h = reader.next_u16()
                                val raw_data = reader.read(w * h * 4)
                                IndexedDb.store_texture(path, i, w, h, raw_data)
                            }
                        }
                    }
                }
            }
            2 -> {
                val reader = BufferReader(event.data as ArrayBuffer)
                VIEW_MATRIX = reader.next_matrix()

                while (reader.has_next()) {
                    sprite_render_commands.add(RenderCommand.Sprite3D(w = reader.next_f32(),
                                                                      h = reader.next_f32(),
                                                                      color = reader.next_v4(),
                                                                      offset = reader.next_v2(),
                                                                      matrix = reader.next_matrix(),
                                                                      server_texture_id = reader.next_u16()))
                    val _dummy = reader.next_u16()
                }
            }
            else -> {

            }
        }
    }
}

fun start_frame(socket: WebSocket) {
    var last_tick = 0.0
    var tickrate = 1000 / 20
    var tick = { s: Double ->

    }
    Input.register_event_handlers(canvas, document)

    tick = { s: Double ->
        render(gl, sprite_render_commands)
        sprite_render_commands.clear()

        val now = Date.now()
        val elapsed = now - last_tick

        if (elapsed > tickrate) {
            last_tick = now
            Input.send_input_data(socket)
        }
        window.requestAnimationFrame(tick)
    }
    window.requestAnimationFrame(tick)
}

fun render(gl: WebGL2RenderingContext, sprite_render_commands: ArrayList<RenderCommand.Sprite3D>) {
    gl.useProgram(sprite_gl_program.program)
    gl.activeTexture(WebGLRenderingContext.TEXTURE0)
    gl.uniform1i(sprite_gl_program.model_texture, 0)
    gl.uniformMatrix4fv(sprite_gl_program.projection_mat, false, PROJECTION_MATRIX)

    gl.bindBuffer(WebGLRenderingContext.ARRAY_BUFFER, sprite_buffer)
    gl.enableVertexAttribArray(sprite_gl_program.a_pos)
    gl.enableVertexAttribArray(sprite_gl_program.a_uv)
    gl.vertexAttribPointer(sprite_gl_program.a_pos, 2, WebGLRenderingContext.FLOAT, false, 4 * 4, 0)
    gl.vertexAttribPointer(sprite_gl_program.a_uv, 2, WebGLRenderingContext.FLOAT, false, 4 * 4, 2 * 4)

    for (render_command in sprite_render_commands) {
        gl.uniformMatrix4fv(sprite_gl_program.view_mat, false, VIEW_MATRIX)
        gl.uniform4fv(sprite_gl_program.color, render_command.color)
        gl.uniformMatrix4fv(sprite_gl_program.model, false, render_command.matrix)
        gl.uniform2fv(sprite_gl_program.offset, render_command.offset)
        gl.uniform2fv(sprite_gl_program.size, arrayOf(render_command.w, render_command.h))

        gl.bindTexture(WebGLRenderingContext.TEXTURE_2D, get_or_load_texture(render_command.server_texture_id))
        gl.drawArrays(WebGLRenderingContext.TRIANGLE_STRIP, 0, 4)
    }
}

private suspend fun load_texture(glTexture: TextureData, path: String, i: Int): WebGLTexture {
    val raw_data =
            IndexedDb.get_texture(path, i) ?: Uint8Array(glTexture.width * glTexture.height * 4).apply {
                this.asDynamic().fill(150)
            }
    val texture_obj = gl.createTexture()!!
    gl.bindTexture(WebGLRenderingContext.TEXTURE_2D, texture_obj)
    gl.texParameteri(WebGLRenderingContext.TEXTURE_2D,
                     WebGLRenderingContext.TEXTURE_MIN_FILTER,
                     WebGLRenderingContext.LINEAR)

    val canvas = document.createElement("canvas") as HTMLCanvasElement
    canvas.width = glTexture.width
    canvas.height = glTexture.height
    val ctx = canvas.getContext("2d") as CanvasRenderingContext2D
    val imageData =
            ImageData(Uint8ClampedArray(raw_data.buffer, raw_data.byteOffset, raw_data.byteLength),
                      glTexture.width,
                      glTexture.height)
    ctx.putImageData(imageData,
                     0.0, 0.0, 0.0, 0.0, canvas.width.toDouble(), canvas.height.toDouble());

    // TODO: this is too slow
    val img = ctx.getImageData(0.0, 0.0, canvas.width.toDouble(), canvas.height.toDouble())
    gl.texImage2D(WebGLRenderingContext.TEXTURE_2D,
                  level = 0,
                  internalformat = WebGLRenderingContext.RGBA,
                  width = glTexture.width,
                  height = glTexture.height,
                  border = 0,
                  format = WebGLRenderingContext.RGBA,
                  type = WebGLRenderingContext.UNSIGNED_BYTE,
                  pixels = img.data)
    return texture_obj
}

val dummy_texture = gl.createTexture()!!
private fun get_or_load_texture(server_texture_id: Int): WebGLTexture {
    val texture_id = server_to_client_gl_indices[server_texture_id]
    if (texture_id == null) {
        console.log("Loading texture $server_texture_id")
        // put dummy value into it so it won't trigger loading again
        server_to_client_gl_indices[server_texture_id] = dummy_texture
        GlobalScope.launch {
            val maybe = server_texture_index_to_path[server_texture_id]
            if (maybe == null) {
                console.error("No path data for $server_texture_id")
            } else {
                val (glTexture, path, i) = maybe
                val new_texture_id = load_texture(glTexture, path, i)
                console.log("Texture was loaded: $server_texture_id")
                server_to_client_gl_indices[server_texture_id] = new_texture_id
            }
        }
        return dummy_texture
    } else {
        return texture_id
    }
}

fun toPowerOfTwo(num: Int): Int {
    val pow = js("Math.pow")
    val ceil = js("Math.ceil")
    val log = js("Math.log")
    val r: Double = pow(2.0, ceil(log(num.toDouble()) / log(2.0)))
    return r.toInt()
}

class BufferReader(val buffer: ArrayBuffer) {
    var offset = 0
    val view = DataView(buffer)
    fun next_u16(): Int {
        val ret = view.getUint16(offset, true)
        offset += 2
        return ret as Int
    }

    fun next_f32(): Float {
        val ret = view.getFloat32(offset, true)
        offset += 4
        return ret
    }

    fun next_string_with_length(): String {
        val str_len = view.getUint16(offset, true) as Int
        val path = Uint8Array(buffer, offset + 2, str_len)
        offset += str_len + 2
        return js("String.fromCharCode.apply(null, path)")
    }

    fun read(len: Int): Uint8Array {
        val ret = Uint8Array(buffer, offset, len)
        offset += len
        return ret
    }

    fun has_next(): Boolean {
        return offset < buffer.byteLength
    }

    fun next_matrix(): Float32Array {
        val ret = Float32Array(buffer, offset, 16)
        offset += 4 * 4 * 4
        return ret
    }

    fun next_v4(): Float32Array {
        val ret = Float32Array(buffer, offset, 4)
        offset += 4 * 4
        return ret
    }

    fun next_v2(): Float32Array {
        val ret = Float32Array(buffer, offset, 2)
        offset += 4 * 2
        return ret
    }
}

suspend fun Job.await() {
    Promise<Nothing> { resolve, reject ->
        var handler: dynamic = null
        handler = {
            if (this.isCompleted) {
                resolve(0.asDynamic())
            } else {
                window.setTimeout(handler, 100)
            }
        }
        window.setTimeout(handler, 100)
    }.await()
}

fun create_sprite_buffer(gl: WebGL2RenderingContext): WebGLBuffer {
    val buffer = gl.createBuffer()!!
    gl.bindBuffer(WebGLRenderingContext.ARRAY_BUFFER, buffer)
    gl.bufferData(WebGLRenderingContext.ARRAY_BUFFER,
                  Float32Array(arrayOf(-0.5f, +0.5f, 0.0f, 0.0f,
                                       +0.5f, +0.5f, 1.0f, 0.0f,
                                       -0.5f, -0.5f, 0.0f, 1.0f,
                                       +0.5f, -0.5f, 1.0f, 1.0f)),
                  WebGLRenderingContext.STATIC_DRAW)
    return buffer
}

fun load_sprite_shader(gl: WebGL2RenderingContext): SpriteShader {
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
                        model_texture = gl.getUniformLocation(program, "model_texture")!!,
                        size = gl.getUniformLocation(program, "size")!!,
                        model = gl.getUniformLocation(program, "model")!!,
                        color = gl.getUniformLocation(program, "color")!!,
                        a_pos = gl.getAttribLocation(program, "Position"),
                        a_uv = gl.getAttribLocation(program, "aTexCoord"),
                        offset = gl.getUniformLocation(program, "offset")!!)
}

data class SpriteShader(val program: WebGLProgram,
                        val projection_mat: WebGLUniformLocation,
                        val view_mat: WebGLUniformLocation,
                        val model_texture: WebGLUniformLocation,
                        val size: WebGLUniformLocation,
                        val model: WebGLUniformLocation,
                        val color: WebGLUniformLocation,
                        val offset: WebGLUniformLocation,
                        val a_pos: Int,
                        val a_uv: Int)