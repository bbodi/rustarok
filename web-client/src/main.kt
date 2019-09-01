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
    val gl_textures: Array<TextureData>
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
var VIEW_MATRIX: Float32Array = Float32Array(16)
var state = 0

sealed class RenderCommand {
    data class Sprite3D(val server_texture_id: Int,
                        val matrix: Float32Array,
                        val color: Float32Array,
                        val offset: Float32Array,
                        val w: Float,
                        val h: Float) : RenderCommand()

    data class Number3D(val value: Int,
                        val matrix: Float32Array,
                        val color: Float32Array,
                        val offset: Float32Array,
                        val size: Float) : RenderCommand()
}

val render_commands = RenderCommands()

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
                            for (i in databaseTextureEntry.gl_textures.indices) {
                                databaseTextureEntry.gl_textures[i] = TextureData(databaseTextureEntry.gl_textures[i])
                            }
                            map[key] = databaseTextureEntry
                            if (key == "[100, 97, 116, 97, 92, 115, 112, 114, 105, 116, 101, 92, 195, 128, 195, 142, 194, 176, 194, 163, 195, 129, 194, 183, 92, 194, 184, 195, 182, 195, 133, 195, 171, 92, 194, 191, 194, 169, 92, 195, 133, 194, 169, 194, 183, 195, 167, 194, 188, 194, 188, 195, 128, 195, 140, 194, 180, 195, 181, 95, 194, 191, 194, 169]") {
                                js("debugger")
                            }
                            path_to_server_gl_indices[key] = databaseTextureEntry
                            for ((i, glTexture) in databaseTextureEntry.gl_textures.withIndex()) {
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
                }
                        .readAsText(blob)
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
                    for (i in 0 until reader.next_u32()) {
                        render_commands.sprite_render_commands.add(RenderCommand.Sprite3D(w = reader.next_f32(),
                                                                                          h = reader.next_f32(),
                                                                                          color = reader.next_v4(),
                                                                                          offset = reader.next_v2(),
                                                                                          matrix = reader.next_matrix(),
                                                                                          server_texture_id = reader.next_u32()))
                    }

                    for (i in 0 until reader.next_u32()) {
                        render_commands.number_render_commands.add(RenderCommand.Number3D(size = reader.next_f32(),
                                                                                          color = reader.next_v4(),
                                                                                          offset = reader.next_v2(),
                                                                                          matrix = reader.next_matrix(),
                                                                                          value = reader.next_u32()))
                    }
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
        render_commands.render(gl)
        render_commands.clear()

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


private suspend fun load_texture(glTexture: TextureData, path: String, i: Int): WebGLTexture? {
    val raw_data = IndexedDb.get_texture(path, i) ?: return null
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
fun get_or_load_texture(server_texture_id: Int): WebGLTexture {
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
                if (new_texture_id == null) {
                    console.error("Texture was not found: $server_texture_id")
                } else {
                    console.log("Texture was loaded: $server_texture_id")
                    server_to_client_gl_indices[server_texture_id] = new_texture_id
                }
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

    fun next_u32(): Int {
        val ret = view.getUint32(offset, true)
        offset += 4
        return ret
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