package rustarok

import kotlinx.coroutines.*
import org.khronos.webgl.*
import org.w3c.dom.*
import org.w3c.files.Blob
import org.w3c.files.FileReader
import rustarok.render.*
import kotlin.browser.document
import kotlin.browser.window
import kotlin.js.Date
import kotlin.js.Promise

data class ModelFace(val server_texture_id: Int,
                     val buffer: WebGLBuffer,
                     val vertex_count: Int)

class ModelData {
    val nodes = arrayListOf<ModelFace>()
}

data class ModelInstance(val index: Int, val matrix: Float32Array)

class BrowserTextureData(val texture: WebGLTexture, val w: Float, val h: Float)


class ServerTextureData(private val native: dynamic) {
    val server_gl_index: Int
        get() = native[0]

    val width: Int
        get() = native[1]

    val height: Int
        get() = native[2]
}

class StoredTexture(private val native: dynamic) {
    val raw: Uint8Array
        get() = native.raw

    val w: Int
        get() = native.w

    val h: Int
        get() = native.h
}

class StoredVertexArray(private val native: dynamic) {
    val raw: Uint8Array
        get() = native.raw

    val vertex_count: Int
        get() = native.vertex_count
}

class StoredModel(private val native: dynamic) {
    val raw: Uint8Array
        get() = native.raw

    val vertex_count: Int
        get() = native.vertex_count

    val texture_name: String
        get() = native.texture_name
}


interface DatabaseTextureEntry {
    val gl_textures: Array<ServerTextureData>
    val hash: String
}

abstract external class WebGL2RenderingContext : WebGLRenderingContext

val server_to_client_gl_indices = hashMapOf<Int, BrowserTextureData>()
val path_to_server_gl_indices = hashMapOf<String, DatabaseTextureEntry>()
val server_texture_index_to_path = hashMapOf<Int, Triple<ServerTextureData, String, Int>>() // path, i
var canvas = document.getElementById("main_canvas") as HTMLCanvasElement
private var gl: WebGL2RenderingContext = 0.asDynamic()

var VIDEO_WIDTH = 0
var VIDEO_HEIGHT = 0
var PROJECTION_MATRIX: Float32Array = 0.asDynamic()
var ORTHO_MATRIX: Float32Array = 0.asDynamic()
var ground_render_command: RenderCommand.Ground3D = 0.asDynamic()
var VIEW_MATRIX: Float32Array = Float32Array(4 * 4)
var NORMAL_MATRIX: Float32Array = Float32Array(3 * 3)
var map_name = ""

enum class ApppState {
    WaitingForWelcomeMsg,
    ReceivingMismatchingTextures,
    ReceivingGroundVertexBuffer,
    ReceivingModels,
    ReceivingModelInstances,
    ReceivingRenderCommands,
}

var state = ApppState.WaitingForWelcomeMsg

sealed class RenderCommand {
    data class Sprite3D(val server_texture_id: Int,
                        val matrix: Float32Array,
                        val color: Float32Array,
                        val size: Float,
                        val offset: Float32Array,
                        val is_vertically_flipped: Boolean) : RenderCommand()

    data class Texture2D(val server_texture_id: Int,
                         val matrix: Float32Array,
                         val color: Float32Array,
                         val offset: Array<Int>,
                         val layer: Int,
                         val size: Float) : RenderCommand()


    data class Number3D(val value: Int,
                        val matrix: Float32Array,
                        val color: Float32Array,
                        val size: Float) : RenderCommand()

    data class Rectangle3D(val matrix: Float32Array,
                           val color: Float32Array,
                           val w: Float,
                           val h: Float) : RenderCommand()

    data class Circle3D(val matrix: Float32Array,
                        val color: Float32Array,
                        val radius: Float) : RenderCommand()

    data class PartialCircle2D(val matrix: Float32Array,
                               val color: Float32Array,
                               val layer: Int,
                               val size: Float,
                               val index: Int) : RenderCommand()

    data class Model3D(val model_instance_index: Int,
                       val is_transparent: Boolean) : RenderCommand()

    data class Ground3D(val light_dir: Float32Array,
                        val light_ambient: Float32Array,
                        val light_diffuse: Float32Array,
                        val light_opacity: Float,
                        val server_texture_atlas_id: Int,
                        val server_lightmap_texture_id: Int,
                        val server_tile_color_texture_id: Int) : RenderCommand()
}

var socket: WebSocket = 0.asDynamic()
var dummy_texture: BrowserTextureData = 0.asDynamic()
val jobs = arrayListOf<suspend () -> Unit>()

fun main() {
    if (canvas.getContext("webgl2") == null) {
        window.alert("WebGL 2.0 is not enabled in your browser. Please follow the instructions: https://get.webgl.org/webgl2/enable.html")
        return
    }
    gl = canvas.getContext("webgl2") as WebGL2RenderingContext
    dummy_texture = BrowserTextureData(gl.createTexture()!!, 28f, 28f)

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

    GlobalScope.launch {
        while (true) {
            delay(100)
            console.log(jobs.size)
            while (jobs.iterator().hasNext()) {
                val job = jobs.removeAt(0)
                GlobalScope.launch {
                    job.invoke()
                }
            }

        }
    }

    var model_count = 0

    val renderer = Renderer(gl)

    socket.onmessage = { event ->
        when (state) {
            ApppState.WaitingForWelcomeMsg -> {
                state = ApppState.ReceivingMismatchingTextures
                val blob = Blob(arrayOf(Uint8Array(event.data as ArrayBuffer)))
                FileReader().apply {
                    this.onload = {
                        process_welcome_msg(JSON.parse(this.result))
                    }
                }
                        .readAsText(blob)
            }
            ApppState.ReceivingMismatchingTextures -> {
                console.info("Received missing textures")
                GlobalScope.launch {
                    val reader = BufferReader(event.data as ArrayBuffer)
                    if (reader.view.byteLength >= 4 && reader.view.getUint32(reader.offset).asDynamic() == js("0xB16B00B5")) {
                        console.info("DONE")
                        reader.next_f32()
                        console.log("All textures have been downloaded")

                        val mismatched_vertex_buffers = arrayListOf<String>()
                        val ground_vertex_array_data = IndexedDb.get_vertex_array("${map_name}_ground")
                        if (ground_vertex_array_data == null) {
                            console.info("${map_name}_ground is missing")
                            mismatched_vertex_buffers.add("3d_ground")
                        } else {
                            renderer.ground_renderer.set_vertex_buffer(gl,
                                                                       ground_vertex_array_data.raw,
                                                                       ground_vertex_array_data.vertex_count)
                        }

                        state = ApppState.ReceivingGroundVertexBuffer
                        socket.send(JSON.stringify(object {
                            val mismatched_vertex_buffers = mismatched_vertex_buffers
                        }))
                    } else {
                        while (reader.has_next()) {
                            val path = reader.next_string_with_length()
                            val hash = reader.next_string_with_length()
                            val count = reader.next_u16()
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
            ApppState.ReceivingGroundVertexBuffer -> {
                GlobalScope.launch {
                    val reader = BufferReader(event.data as ArrayBuffer)
                    if (reader.view.byteLength >= 4 && reader.view.getUint32(reader.offset).asDynamic() == js("0xB16B00B5")) {
                        console.info("ground DONE")
                        reader.next_f32()

                        val model_name_to_index = welcome_msg.asset_database.model_name_to_index
                        val model_names: Array<String> = js("Object").keys(model_name_to_index)
                        val map = hashMapOf<String, Int>()
                        for (model_name in model_names) {
                            map[model_name] = model_name_to_index[model_name]
                        }
                        welcome_msg.asset_database.model_name_to_index = map
                        val missing_models = map.filter { (model_name, _) ->
                            IndexedDb.get_model(model_name, 0) == null
                        }.map { it.key }
                        socket.send(JSON.stringify(object {
                            val missing_models = missing_models
                        }))
                        state = ApppState.ReceivingModels
                    } else {
                        while (reader.has_next()) {
                            when (reader.next_u8()) {
                                1 -> { // ground
                                    console.info("${map_name}_ground was downloaded")
                                    val vertex_count = reader.next_u32()
                                    val buffer_len = reader.next_u32()
                                    val raw_data = reader.read(buffer_len)
                                    IndexedDb.store_vertex_array("${map_name}_ground", vertex_count, raw_data)
                                    renderer.ground_renderer.set_vertex_buffer(gl, raw_data, vertex_count)
                                }
                            }
                        }
                    }
                }
            }
            ApppState.ReceivingModels -> {
                GlobalScope.launch {
                    val reader = BufferReader(event.data as ArrayBuffer)
                    if (reader.view.byteLength >= 4 && reader.view.getUint32(reader.offset).asDynamic() == js("0xB16B00B5")) {
                        console.info("Models DONE")
                        reader.next_f32()

                        // prepare models
                        val model_name_to_index: Map<String, Int> = welcome_msg.asset_database.model_name_to_index

                        val model_index_to_models = Array<ModelData>(model_name_to_index.size) { 0.asDynamic() }
                        model_name_to_index.forEach { (model_name, server_index) ->
                            val model = ModelData()
                            for (i in 0 until 1000) {
                                val data = IndexedDb.get_model(model_name, i) ?: break
                                val buffer = create_vertex_buffer(gl, data.raw)
                                model.nodes.add(ModelFace(
                                        path_to_server_gl_indices[data.texture_name]!!.gl_textures[0].server_gl_index,
                                        buffer,
                                        data.vertex_count
                                ))
                            }
                            model_index_to_models[server_index] = model
                        }
                        renderer.models = model_index_to_models

                        welcome_msg = null
                        state = ApppState.ReceivingModelInstances
                        socket.send(JSON.stringify(object {
                            val send_me_model_instances = true
                        }))
                    } else {
                        while (reader.has_next()) {
                            val model_name = reader.next_string_with_length()
                            console.info("$model_name was downloaded")

                            val node_count = reader.next_u16()
                            var index = 0
                            for (i in 0 until node_count) {
                                val face_count = reader.next_u16()
                                for (j in 0 until face_count) {
                                    val texture_name = reader.next_string_with_length()
                                    val vertex_count = reader.next_u32()
                                    val raw_len = reader.next_u32()
                                    val raw_data = reader.read(raw_len)
                                    IndexedDb.store_model(model_name,
                                                          index,
                                                          vertex_count,
                                                          texture_name,
                                                          raw_data)
                                    ++index
                                }
                            }
                        }
                    }
                }
            }
            ApppState.ReceivingModelInstances -> {
                GlobalScope.launch {
                    val reader = BufferReader(event.data as ArrayBuffer)
                    while (reader.has_next()) {
                        val model_index = reader.next_u32()
                        val matrix = reader.next_4x4matrix()
                        renderer.model_instances.add(ModelInstance(model_index, matrix))
                    }

                    socket.send(JSON.stringify(object {
                        val ready = true
                    }))
                    state = ApppState.ReceivingRenderCommands
                    start_frame(socket, renderer)
                }
            }
            ApppState.ReceivingRenderCommands -> {
                renderer.clear()
                val reader = BufferReader(event.data as ArrayBuffer)
                VIEW_MATRIX = reader.next_4x4matrix()
                NORMAL_MATRIX = reader.next_3x3matrix()

                while (reader.has_next()) {
                    parse_partial_circle_2d_render_commands(reader, renderer)
                    parse_texture2d_render_commands(reader, renderer)
                    parse_rectangle3d_render_commands(reader, renderer)
                    parse_circle3d_render_commands(reader, renderer)
                    parse_sprite_render_commands(reader, renderer)
                    parse_number_render_commands(reader, renderer)
                    parse_model3d_render_commands(reader, renderer)
                }
            }
        }
    }
}

fun create_vertex_buffer(gl: WebGL2RenderingContext, raw: Uint8Array): WebGLBuffer {
    val buffer = gl.createBuffer()!!
    gl.bindBuffer(WebGLRenderingContext.ARRAY_BUFFER, buffer)
    try {
        gl.bufferData(WebGLRenderingContext.ARRAY_BUFFER,
                      Float32Array(raw.buffer.slice(raw.byteOffset, raw.byteOffset + raw.byteLength)),
                      WebGLRenderingContext.STATIC_DRAW)
    } catch (e: Throwable) {
        js("debugger")
    }
    return buffer
}


private fun parse_model3d_render_commands(reader: BufferReader, renderer: Renderer) {
    for (i in 0 until reader.next_u32()) {
        val (is_transparent, model_instance_index) = reader.next_packed_bool1_int31()
        renderer.model3d_render_commands.add(RenderCommand.Model3D(
                is_transparent = is_transparent,
                model_instance_index = model_instance_index
        ))
    }
}

private fun parse_number_render_commands(reader: BufferReader, renderer: Renderer) {
    for (i in 0 until reader.next_u32()) {
        renderer.number_render_commands.add(RenderCommand.Number3D(
                size = reader.next_f32(),
                color = reader.next_color4_u8(),
                matrix = reader.next_4x4matrix(),
                value = reader.next_u32()))
    }
}

private fun parse_sprite_render_commands(reader: BufferReader, renderer: Renderer) {
    for (i in 0 until reader.next_u32()) {
        val color = reader.next_color4_u8()
        val offset = Float32Array(arrayOf(reader.next_i16() * ONE_SPRITE_PIXEL_SIZE_IN_3D,
                                          reader.next_i16() * ONE_SPRITE_PIXEL_SIZE_IN_3D))
        val matrix = reader.next_4x4matrix()
        val size = reader.next_f32()
        val (is_vertically_flipped, server_texture_id) = reader.next_packed_bool1_int31()
        renderer.sprite_render_commands.add(RenderCommand.Sprite3D(
                color = color,
                offset = offset,
                matrix = matrix,
                size = size,
                server_texture_id = server_texture_id,
                is_vertically_flipped = is_vertically_flipped))
    }
}

private fun parse_partial_circle_2d_render_commands(reader: BufferReader, renderer: Renderer) {
    for (i in 0 until reader.next_u32()) {
        val color = reader.next_color4_u8()
        val matrix = reader.next_4x4matrix()
        val size = reader.next_f32()
        val layer = reader.next_u16()
        val index = reader.next_u16()
        renderer.partial_circle2d_render_commands.add(RenderCommand.PartialCircle2D(
                color = color,
                matrix = matrix,
                size = size,
                layer = layer,
                index = index
        ))
    }
}

private fun parse_circle3d_render_commands(reader: BufferReader, renderer: Renderer) {
    for (i in 0 until reader.next_u32()) {
        val color = reader.next_color4_u8()
        val matrix = reader.next_4x4matrix()
        val radius = reader.next_f32()
        renderer.circle3d_render_commands.add(RenderCommand.Circle3D(
                color = color,
                matrix = matrix,
                radius = radius
        ))
    }
}

private fun parse_rectangle3d_render_commands(reader: BufferReader, renderer: Renderer) {
    for (i in 0 until reader.next_u32()) {
        val color = reader.next_color4_u8()
        val matrix = reader.next_4x4matrix()
        val w = reader.next_f32()
        val h = reader.next_f32()
        renderer.rectangle3d_render_commands.add(RenderCommand.Rectangle3D(
                color = color,
                matrix = matrix,
                w = w,
                h = h
        ))
    }
}

private fun parse_texture2d_render_commands(reader: BufferReader, renderer: Renderer) {
    for (i in 0 until reader.next_u32()) {
        val color = reader.next_color4_u8()
        val offset = arrayOf(reader.next_i16(),
                             reader.next_i16())
        val matrix = reader.next_4x4matrix()
        val (layer, server_texture_id) = reader.next_packed_int8_int24()
        val size = reader.next_f32()
        renderer.texture2d_render_commands.add(RenderCommand.Texture2D(
                color = color,
                offset = offset,
                matrix = matrix,
                server_texture_id = server_texture_id,
                size = size,
                layer = layer
        ))
    }
}

private var welcome_msg: dynamic = null
private fun process_welcome_msg(result: dynamic): Job {
    welcome_msg = result
    VIDEO_WIDTH = result.screen_width
    VIDEO_HEIGHT = result.screen_height
    map_name = result.map_name
    canvas.width = VIDEO_WIDTH
    canvas.height = VIDEO_HEIGHT
    ORTHO_MATRIX = Float32Array(result.ortho_mat as Array<Float>)
    PROJECTION_MATRIX = Float32Array(result.projection_mat as Array<Float>)

    ground_render_command = RenderCommand.Ground3D(
            light_dir = result.ground.light_dir,
            light_ambient = result.ground.light_ambient,
            light_diffuse = result.ground.light_diffuse,
            light_opacity = result.ground.light_opacity,
            server_texture_atlas_id = 0,
            server_lightmap_texture_id = 0,
            server_tile_color_texture_id = 0
    )


    val texture_db = result.asset_database.texture_db.entries
    val keys: Array<String> = js("Object").keys(texture_db)
    val map = hashMapOf<String, DatabaseTextureEntry>()
    for (key in keys) {
        val databaseTextureEntry: DatabaseTextureEntry = texture_db[key]
        for (i in databaseTextureEntry.gl_textures.indices) {
            databaseTextureEntry.gl_textures[i] = ServerTextureData(databaseTextureEntry.gl_textures[i])
        }
        map[key] = databaseTextureEntry
        path_to_server_gl_indices[key] = databaseTextureEntry
        for ((i, glTexture) in databaseTextureEntry.gl_textures.withIndex()) {
            server_texture_index_to_path[glTexture.server_gl_index] = Triple(glTexture, key, i)
            when (key) {
                "ground_texture_atlas" -> ground_render_command = ground_render_command.copy(
                        server_texture_atlas_id = glTexture.server_gl_index
                )
                "ground_lightmap_texture" -> ground_render_command = ground_render_command.copy(
                        server_lightmap_texture_id = glTexture.server_gl_index
                )
                "ground_tile_color_texture" -> ground_render_command = ground_render_command.copy(
                        server_tile_color_texture_id = glTexture.server_gl_index
                )
            }
        }
    }
    return GlobalScope.launch {
        val mismatched_textures = IndexedDb.collect_mismatched_textures(map)
        console.log(mismatched_textures)
        socket.send(JSON.stringify(object {
            val mismatched_textures = mismatched_textures
        }))
    }
}

fun start_frame(socket: WebSocket, renderer: Renderer) {
    console.log("start_frame")

    gl.viewport(0, 0, VIDEO_WIDTH, VIDEO_HEIGHT)
    gl.clearColor(0.3f, 0.3f, 0.5f, 1.0f)
    gl.enable(WebGLRenderingContext.DEPTH_TEST)
    gl.depthFunc(WebGLRenderingContext.LEQUAL)
    gl.enable(WebGLRenderingContext.BLEND)
    gl.blendFunc(WebGLRenderingContext.SRC_ALPHA, WebGLRenderingContext.ONE_MINUS_SRC_ALPHA)
    gl.lineWidth(2.0f)

    var last_tick = 0.0
    var tickrate = 1000 / 20
    var tick = { s: Double ->

    }
    Input.register_event_handlers(canvas, document)

    tick = { s: Double ->
        renderer.render(gl)

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


private suspend fun load_texture(glServerTexture: ServerTextureData,
                                 path: String,
                                 i: Int,
                                 min_mag: Int): WebGLTexture? {
    val raw_data = IndexedDb.get_texture(path, i) ?: return null
    val texture_obj = gl.createTexture()!!
    gl.bindTexture(WebGLRenderingContext.TEXTURE_2D, texture_obj)
    gl.texParameteri(WebGLRenderingContext.TEXTURE_2D,
                     WebGLRenderingContext.TEXTURE_MIN_FILTER,
                     min_mag)
    gl.texParameteri(WebGLRenderingContext.TEXTURE_2D,
                     WebGLRenderingContext.TEXTURE_MAG_FILTER,
                     min_mag)
    gl.texParameteri(WebGLRenderingContext.TEXTURE_2D,
                     WebGLRenderingContext.TEXTURE_WRAP_S,
                     WebGLRenderingContext.CLAMP_TO_EDGE)
    gl.texParameteri(WebGLRenderingContext.TEXTURE_2D,
                     WebGLRenderingContext.TEXTURE_WRAP_T,
                     WebGLRenderingContext.CLAMP_TO_EDGE)

    val canvas = document.createElement("canvas") as HTMLCanvasElement
    canvas.width = glServerTexture.width
    canvas.height = glServerTexture.height
    val ctx = canvas.getContext("2d") as CanvasRenderingContext2D
    val imageData =
            ImageData(Uint8ClampedArray(raw_data.raw.buffer, raw_data.raw.byteOffset, raw_data.raw.byteLength),
                      glServerTexture.width,
                      glServerTexture.height)
    ctx.putImageData(imageData,
                     0.0, 0.0, 0.0, 0.0, canvas.width.toDouble(), canvas.height.toDouble());

    // TODO: this is too slow
    val img = ctx.getImageData(0.0, 0.0, canvas.width.toDouble(), canvas.height.toDouble())
    gl.texImage2D(WebGLRenderingContext.TEXTURE_2D,
                  level = 0,
                  internalformat = WebGLRenderingContext.RGBA,
                  width = glServerTexture.width,
                  height = glServerTexture.height,
                  border = 0,
                  format = WebGLRenderingContext.RGBA,
                  type = WebGLRenderingContext.UNSIGNED_BYTE,
                  pixels = img.data)
    return texture_obj
}

fun get_or_load_server_texture(server_texture_id: Int, min_mag: Int): BrowserTextureData {
    val texture_id = server_to_client_gl_indices[server_texture_id]
    if (texture_id == null) {
        console.log("Loading texture $server_texture_id")
        // put dummy value into it so it won't trigger loading again
        server_to_client_gl_indices[server_texture_id] = dummy_texture
        jobs.add {
            val maybe = server_texture_index_to_path[server_texture_id]
            if (maybe == null) {
                console.error("No path data for $server_texture_id")
            } else {
                val (glTexture, path, i) = maybe
                val new_texture_id = load_texture(glTexture, path, i, min_mag)
                if (new_texture_id == null) {
                    console.error("Texture was not found: $path")
                } else {
                    console.log("Texture was loaded: $path, $i")
                    server_to_client_gl_indices[server_texture_id] =
                            BrowserTextureData(new_texture_id, glTexture.width.toFloat(), glTexture.height.toFloat())
                }
            }
        }
        return dummy_texture
    } else {
        return texture_id
    }
}

class BufferReader(val buffer: ArrayBuffer) {
    var offset = 0
    val view = DataView(buffer)

    fun next_u8(): Int {
        val ret = view.getUint8(offset)
        offset += 1
        return ret.toInt()
    }

    fun next_i16(): Int {
        val ret = view.getInt16(offset, true)
        offset += 2
        return ret as Int
    }

    fun next_u16(): Int {
        val ret = view.getUint16(offset, true)
        offset += 2
        return ret as Int
    }

    fun next_packed_bool1_int31(): Pair<Boolean, Int> {
        val packed_int = next_u32()
        val int = packed_int.shl(1).ushr(1)
        val bool = packed_int.ushr(31) == 1
        return bool to int
    }

    fun next_packed_int8_int24(): Pair<Int, Int> {
        val packed_int = next_u32()
        val in24 = packed_int.shl(8).ushr(8)
        val in8 = packed_int.ushr(24)
        return in8 to in24
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

    fun next_4x4matrix(): Float32Array {
        val ret = Float32Array(buffer, offset, 16)
        offset += 4 * 4 * 4
        return ret
    }

    fun next_3x3matrix(): Float32Array {
        val ret = Float32Array(buffer, offset, 9)
        offset += 3 * 3 * 4
        return ret
    }

    fun next_v4(): Float32Array {
        val ret = Float32Array(buffer, offset, 4)
        offset += 4 * 4
        return ret
    }

    fun next_color4_u8(): Float32Array {
        val bytes = Uint8Array(buffer, offset, 4)
        val ret =
                Float32Array(arrayOf(bytes[0].toFloat() / 255f,
                                     bytes[1].toFloat() / 255f,
                                     bytes[2].toFloat() / 255f,
                                     bytes[3].toFloat() / 255f))
        offset += 4
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