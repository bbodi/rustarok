package rustarok

import kotlinx.coroutines.*
import kotlinx.coroutines.channels.Channel
import org.khronos.webgl.*
import org.w3c.dom.*
import org.w3c.files.Blob
import org.w3c.files.FileReader
import rustarok.render.ONE_SPRITE_PIXEL_SIZE_IN_3D
import rustarok.render.Renderer
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
    val path: String
        get() = native.path

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

class StoredEffect(private val native: dynamic) {
    val raw: Uint8Array
        get() = native.raw

    val vertex_count: Int
        get() = native.vertex_count

    val texture_name: String
        get() = native.texture_name
}

class StrKeyFrame(
        val frame: Int,
        val typ: Int,
        val pos_x: Float,
        val pos_y: Float,
        val xy: Array<Float>,
        val color: Float32Array,
        val angle: Float,
        val src_alpha: Int,
        val dst_alpha: Int,
        val texture_index: Int
)

val COLOR_WHITE = Float32Array(arrayOf(1f, 1f, 1f, 1f))

class StrLayer(val key_frames: Array<StrKeyFrame>)

class StrFile(
        val max_key: Int,
        val fps: Int,
        val layers: Array<StrLayer>,
        val server_texture_indices: Array<Int>
)


abstract external class WebGL2RenderingContext : WebGLRenderingContext

val server_to_client_gl_indices = hashMapOf<Int, BrowserTextureData>()
val path_to_server_gl_indices = hashMapOf<String, TextureId>()
val server_texture_index_to_path = hashMapOf<Int, String>()
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

enum class ApppStateName {
    WaitingForWelcomeMsg,
    ReceivingMismatchingTextures,
    ReceivingMissingEffects,
    ReceivingGroundVertexBuffer,
    ReceivingModels,
    ReceivingModelInstances,
    ReceivingRenderCommands,
}

data class AppState(val name: ApppStateName,
                    val downloading_textures: Int,
                    val downloaded_textures: Int,
                    val downloading_models: Int,
                    val downloaded_models: Int,
                    val downloading_effects: Int,
                    val downloaded_effects: Int)

sealed class AppCommand {
    class TextureDownloaded : AppCommand()
    class ModelDownloaded : AppCommand()
    class EffectDownloaded : AppCommand()
    data class ChangeState(val new_state: ApppStateName, val param: Any?) : AppCommand()
}

var state = AppState(
        ApppStateName.WaitingForWelcomeMsg,
        0, 0, 0, 0, 0, 0
)

sealed class RenderCommand {
    data class Sprite3D(val server_texture_id: Int,
                        val x: Float,
                        val y: Float,
                        val z: Float,
                        val color: Float32Array,
                        val scale: Float,
                        val rot_radian: Float,
                        val offset: Float32Array,
                        val is_vertically_flipped: Boolean) : RenderCommand()

    data class Texture2D(val server_texture_id: Int,
                         val rotation_rad: Float,
                         val x: Float,
                         val y: Float,
                         val color: Float32Array,
                         val offset: Array<Int>,
                         val layer: Int,
                         val scale: Float) : RenderCommand()


    data class Number3D(val value: Int,
                        val x: Float,
                        val y: Float,
                        val z: Float,
                        val color: Float32Array,
                        val scale: Float) : RenderCommand()

    data class Effect3D(val x: Float,
                        val y: Float,
                        val key_index: Int,
                        val effect_id: Int) : RenderCommand()


    data class Rectangle3D(val x: Float,
                           val y: Float,
                           val z: Float,
                           val rotation_rad: Float,
                           val color: Float32Array,
                           val w: Float,
                           val h: Float) : RenderCommand()

    data class Trimesh3D(val x: Float,
                         val y: Float,
                         val z: Float
    ) : RenderCommand()

    data class HorizontalTexture3D(val x: Float,
                                   val z: Float,
                                   val color: Float32Array,
                                   val rotation_rad: Float,
                                   val size: TextureSize,
                                   val server_texture_id: Int
    ) : RenderCommand()

    data class Circle3D(val x: Float,
                        val y: Float,
                        val z: Float,
                        val color: Float32Array,
                        val radius: Float) : RenderCommand()

    data class PartialCircle2D(val screen_pos_x: Short,
                               val screen_pos_y: Short,
                               val color: Float32Array,
                               val layer: Int,
                               val index: Int) : RenderCommand()

    data class Rectangle2D(val color: Float32Array,
                           val layer: Int,
                           val screen_pos_x: Short,
                           val screen_pos_y: Short,
                           val rotation_rad: Float,
                           val w: Int,
                           val h: Int) : RenderCommand()


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
val job_channel = Channel<suspend () -> Unit>(Channel.UNLIMITED)
val app_state_channel = Channel<AppCommand>(2048)
val packet_channel = Channel<ArrayBuffer>(Channel.UNLIMITED)

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
            for (job in job_channel) {
                GlobalScope.launch {
                    job.invoke()
                }
            }
        }
    }

    val renderer = Renderer(gl)

    GlobalScope.launch {
        while (state.name != ApppStateName.ReceivingRenderCommands) {
            delay(100)
            for (command in app_state_channel) {
                when (command) {
                    is AppCommand.EffectDownloaded -> {
                        state = state.copy(downloaded_effects = state.downloaded_effects + 1)
                    }
                    is AppCommand.ModelDownloaded -> {
                        state = state.copy(downloaded_models = state.downloaded_models + 1)
                    }
                    is AppCommand.TextureDownloaded -> {
                        state =
                                state.copy(downloaded_textures = kotlin.math.min(state.downloaded_textures + 1,
                                        state.downloading_textures))
                        if (state.downloaded_textures >= state.downloading_textures) {
                            console.log("All textures have been downloaded")

                            val effect_names: Array<String> = welcome_msg.effect_names

                            val missing_effects = effect_names.filter { effect_name ->
                                IndexedDb.get_effect(effect_name) == null
                            }

                            app_state_channel.offer(AppCommand.ChangeState(ApppStateName.ReceivingMissingEffects,
                                    missing_effects.size))
                            socket.send(JSON.stringify(object {
                                val missing_effects = missing_effects
                            }))
                        }
                    }
                    is AppCommand.ChangeState -> {
                        state = state.copy(name = command.new_state)
                        when (state.name) {
                            ApppStateName.ReceivingMismatchingTextures -> {
                                state = state.copy(downloading_textures = command.param as Int)
                            }
                            ApppStateName.WaitingForWelcomeMsg -> {
                            }
                            ApppStateName.ReceivingMissingEffects -> {
                                state = state.copy(downloading_effects = command.param as Int)
                            }
                            ApppStateName.ReceivingGroundVertexBuffer -> {
                            }
                            ApppStateName.ReceivingModels -> {
                                state = state.copy(downloading_models = command.param as Int)
                            }
                            ApppStateName.ReceivingModelInstances -> {
                            }
                            ApppStateName.ReceivingRenderCommands -> {
                            }
                        }
                    }
                }
                update_dom(state, renderer)
            }
        }
    }

    socket.onmessage = {
        packet_channel.offer(it.data as ArrayBuffer)
    }

    GlobalScope.launch {
        while (state.name != ApppStateName.ReceivingRenderCommands) {
            for (packet in packet_channel) {
                process_handshake_packet(renderer, packet)
            }
        }
    }
}

private suspend fun process_handshake_packet(renderer: Renderer,
                                             packet: ArrayBuffer) {
    val reader = BufferReader(packet)
    when (state.name) {
        ApppStateName.WaitingForWelcomeMsg -> {
            val blob = Blob(arrayOf(Uint8Array(packet)))
            FileReader().apply {
                this.onload = {
                    process_welcome_msg(JSON.parse(this.result))
                }
            }.readAsText(blob)
        }
        ApppStateName.ReceivingMismatchingTextures -> {
            console.info("Received missing textures")
            while (reader.has_next()) {
                val path = reader.next_string_with_length()
                console.info("Download $path")
                val w = reader.next_u16()
                val h = reader.next_u16()
                val raw_data = reader.read(w * h * 4)

                IndexedDb.store_textures(path, Triple(w, h, raw_data)).then {
                    console.info("Stored: $path")
                    app_state_channel.offer(AppCommand.TextureDownloaded())
                }.catch {
                    js("debugger")
                    console.error("store_textures error: $it")
                }.await()
            }
        }
        ApppStateName.ReceivingMissingEffects -> {
            console.info("Received missing effects")
            if (reader.view.byteLength >= 4 && reader.view.getUint32(reader.offset).asDynamic() == js(
                            "0xB16B00B5")) {
                console.info("DONE")
                reader.next_f32()
                console.log("All effects have been downloaded")
                val effect_names: Array<String> = welcome_msg.effect_names

                renderer.effects = effect_names.map { effect_name ->
                    IndexedDb.get_effect(effect_name)!!
                }.toTypedArray()


                val mismatched_vertex_buffers = arrayListOf<String>()
                val ground_vertex_array_data =
                        IndexedDb.get_vertex_array("${map_name}_ground")
                if (ground_vertex_array_data == null) {
                    console.info("${map_name}_ground is missing")
                    mismatched_vertex_buffers.add("3d_ground")
                } else {
                    renderer.ground_renderer.set_vertex_buffer(gl,
                            ground_vertex_array_data.raw,
                            ground_vertex_array_data.vertex_count)
                }

                app_state_channel.offer(AppCommand.ChangeState(ApppStateName.ReceivingGroundVertexBuffer,
                        null))
                socket.send(JSON.stringify(object {
                    val mismatched_vertex_buffers = mismatched_vertex_buffers
                }))
            } else {
                while (reader.has_next()) {
                    val name = reader.next_string_with_length()
                    val max_key = reader.next_u32()
                    val fps = reader.next_u32()
                    val layer_count = reader.next_u16()
                    val texture_count = reader.next_u16()
                    val server_texture_indices = (0 until texture_count).map {
                        reader.next_u32()
                    }.toTypedArray()
                    val layers = (0 until layer_count).map {
                        val frame_count = reader.next_u16()
                        val key_frames = (0 until frame_count).map {
                            val frame = reader.next_i32()
                            val typ = reader.next_u8()
                            val posx = reader.next_f32()
                            val posy = reader.next_f32()
                            val xy = (0 until 8).map {
                                reader.next_f32()
                            }.toTypedArray()
                            val color = reader.next_color4_u8()
                            val angle = reader.next_f32()
                            val src_alpha = reader.next_i32()
                            val dst_alpha = reader.next_i32()
                            val texture_index = reader.next_u16()
                            StrKeyFrame(frame,
                                    typ,
                                    posx,
                                    posy,
                                    xy,
                                    color,
                                    angle,
                                    src_alpha,
                                    dst_alpha,
                                    texture_index)
                        }.toTypedArray()
                        StrLayer(key_frames)
                    }.toTypedArray()
                    val str_file = StrFile(max_key, fps, layers, server_texture_indices)
                    console.info("Download effect: $name")
                    IndexedDb.store_effect(name, str_file).await()
                    app_state_channel.offer(AppCommand.EffectDownloaded())
                }
            }

        }
        ApppStateName.ReceivingGroundVertexBuffer -> {
            if (reader.view.byteLength >= 4 && reader.view.getUint32(reader.offset).asDynamic() == js(
                            "0xB16B00B5")) {
                console.info("ground DONE")
                reader.next_f32()

                val model_name_to_index = welcome_msg.asset_db.model_name_to_index
                val model_names: Array<String> = js("Object").keys(model_name_to_index)
                val map = hashMapOf<String, Int>()
                for (model_name in model_names) {
                    map[model_name] = model_name_to_index[model_name]
                }
                welcome_msg.asset_db.model_name_to_index = map
                val missing_models = map.filter { (model_name, _) ->
                    IndexedDb.get_model(model_name, 0) == null
                }.map { it.key }
                socket.send(JSON.stringify(object {
                    val missing_models = missing_models
                }))
                app_state_channel.offer(AppCommand.ChangeState(ApppStateName.ReceivingModels,
                        missing_models.size))
            } else {
                while (reader.has_next()) {
                    when (reader.next_u8()) {
                        1 -> { // ground
                            console.info("${map_name}_ground was downloaded")
                            val vertex_count = reader.next_u32()
                            val buffer_len = reader.next_u32()
                            val raw_data = reader.read(buffer_len)
                            IndexedDb.store_vertex_array("${map_name}_ground",
                                    vertex_count,
                                    raw_data).await()
                            renderer.ground_renderer.set_vertex_buffer(gl,
                                    raw_data,
                                    vertex_count)
                        }
                    }
                }
            }

        }
        ApppStateName.ReceivingModels -> {
            if (reader.view.byteLength >= 4 && reader.view.getUint32(reader.offset).asDynamic() == js(
                            "0xB16B00B5")) {
                console.info("Models DONE")
                reader.next_f32()

                // prepare models
                val model_name_to_index: Map<String, Int> =
                        welcome_msg.asset_db.model_name_to_index

                val model_index_to_models =
                        Array<ModelData>(model_name_to_index.size) { 0.asDynamic() }
                model_name_to_index.forEach { (model_name, server_index) ->
                    val model = ModelData()
                    for (i in 0 until 1000) {
                        val data = IndexedDb.get_model(model_name, i) ?: break
                        val buffer = create_vertex_buffer(gl, data.raw)
                        model.nodes.add(ModelFace(
                                path_to_server_gl_indices[data.texture_name]!!.id,
                                buffer,
                                data.vertex_count
                        ))
                    }
                    model_index_to_models[server_index] = model
                }
                renderer.models = model_index_to_models

                welcome_msg = null
                app_state_channel.offer(AppCommand.ChangeState(ApppStateName.ReceivingModelInstances,
                        null))
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
                                    raw_data).await()
                            ++index
                        }
                    }
                    app_state_channel.offer(AppCommand.ModelDownloaded())
                }
            }

        }
        ApppStateName.ReceivingModelInstances -> {
            while (reader.has_next()) {
                val model_index = reader.next_u32()
                val matrix = reader.next_4x4matrix()
                renderer.model_instances.add(ModelInstance(model_index, matrix))
            }

            socket.send(JSON.stringify(object {
                val ready = true
            }))
            app_state_channel.offer(AppCommand.ChangeState(ApppStateName.ReceivingRenderCommands,
                    null))
        }
    }
}

fun update_dom(state: AppState, renderer: Renderer) {
    if (state.name == ApppStateName.ReceivingRenderCommands) {
        document.getElementById("status_text").asDynamic().style.display = "none"
        document.getElementById("main_canvas").asDynamic().style.display = "block"
        start_frame(socket, renderer)
    } else {
        document.getElementById("status_text")!!.innerHTML = ""
        if (state.downloading_textures > 0) {
            document.getElementById("status_text")!!.innerHTML += "Textures: ${state.downloaded_textures}/${state.downloading_textures}<br/>"
        }
        if (state.downloading_effects > 0) {
            document.getElementById("status_text")!!.innerHTML += "Effects: ${state.downloaded_effects}/${state.downloading_effects}<br/>"
        }
        if (state.downloading_models > 0) {
            document.getElementById("status_text")!!.innerHTML += "Models: ${state.downloaded_models}/${state.downloading_models}<br/>"
        }
    }
}

fun create_vertex_buffer(gl: WebGL2RenderingContext, raw: Uint8Array): WebGLBuffer {
    val buffer = gl.createBuffer()!!
    gl.bindBuffer(WebGLRenderingContext.ARRAY_BUFFER, buffer)
    try {
        gl.bufferData(WebGLRenderingContext.ARRAY_BUFFER,
                Float32Array(raw.buffer.slice(raw.byteOffset,
                        raw.byteOffset + raw.byteLength)),
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
                scale = reader.next_f32(),
                color = reader.next_color4_u8(),
                x = reader.next_f32(),
                y = reader.next_f32(),
                z = reader.next_f32(),
                value = reader.next_u32()))
    }
}

private fun parse_effect_3d_render_commands(reader: BufferReader, renderer: Renderer) {
    for (i in 0 until reader.next_u32()) {
        renderer.effect3d_render_commands.add(RenderCommand.Effect3D(
                effect_id = reader.next_u16(),
                key_index = reader.next_u16(),
                x = reader.next_f32(),
                y = reader.next_f32()))
    }
}

private fun parse_sprite_render_commands(reader: BufferReader, renderer: Renderer) {
    for (i in 0 until reader.next_u32()) {
        val color = reader.next_color4_u8()
        val offset = Float32Array(arrayOf(reader.next_i16() * ONE_SPRITE_PIXEL_SIZE_IN_3D,
                reader.next_i16() * ONE_SPRITE_PIXEL_SIZE_IN_3D))
        val rot_radian = reader.next_f32()
        val x = reader.next_f32()
        val y = reader.next_f32()
        val z = reader.next_f32()
        val scale = reader.next_f32()
        val (is_vertically_flipped, server_texture_id) = reader.next_packed_bool1_int31()
        renderer.sprite_render_commands.add(RenderCommand.Sprite3D(
                color = color,
                offset = offset,
                x = x,
                y = y,
                z = z,
                scale = scale,
                rot_radian = rot_radian,
                server_texture_id = server_texture_id,
                is_vertically_flipped = is_vertically_flipped))
    }
}

private fun parse_partial_circle_2d_render_commands(reader: BufferReader, renderer: Renderer) {
    for (i in 0 until reader.next_u32()) {
        val color = reader.next_color4_u8()
        val x = reader.next_i16()
        val y = reader.next_i16()
        val layer = reader.next_u16()
        val index = reader.next_u16()
        renderer.partial_circle2d_render_commands.add(RenderCommand.PartialCircle2D(
                color = color,
                screen_pos_x = x.toShort(),
                screen_pos_y = y.toShort(),
                layer = layer,
                index = index
        ))
    }
}

private fun parse_rectangle_2d_render_commands(reader: BufferReader, renderer: Renderer) {
    for (i in 0 until reader.next_u32()) {
        val color = reader.next_color4_u8()
        val rotation_rad = reader.next_f32()
        val x = reader.next_i16()
        val y = reader.next_i16()
        val packed_int2 = reader.next_u32()
        val packed_int = packed_int2.toUInt()
        val h = packed_int.and(0b1111_11111111u) // lower 12 bit is h
        val w = packed_int.and(0b11111111_11110000_00000000u).shr(12) // next 12 bit is w
        val layer =
                packed_int.and(0b11111111_00000000_00000000_00000000u)
                        .shr(12 + 12) // next 8 bit is layer

        renderer.rectangle2d_render_commands.add(RenderCommand.Rectangle2D(
                color = color,
                layer = layer.toInt(),
                w = w.toInt(),
                h = h.toInt(),
                screen_pos_x = x.toShort(),
                screen_pos_y = y.toShort(),
                rotation_rad = rotation_rad
        ))
    }
}


private fun parse_circle3d_render_commands(reader: BufferReader, renderer: Renderer) {
    for (i in 0 until reader.next_u32()) {
        val color = reader.next_color4_u8()
        val x = reader.next_f32()
        val y = reader.next_f32()
        val z = reader.next_f32()
        val radius = reader.next_f32()
        renderer.circle3d_render_commands.add(RenderCommand.Circle3D(
                color = color,
                x = x,
                y = y,
                z = z,
                radius = radius
        ))
    }
}

sealed class TextureSize {
    data class Fixed(val fixed: Float) : TextureSize()
    data class Scaled(val scaled: Float) : TextureSize()
}

private fun parse_horizontal_texture_3d_commands(reader: BufferReader, renderer: Renderer) {
    for (i in 0 until reader.next_u32()) {
        val color = reader.next_color4_u8()
        val x = reader.next_f32()
        val z = reader.next_f32()
        val rotation_rad = reader.next_f32()
        val texture_index = reader.next_u32()
        val size_type = reader.next_u32()
        val size = when (size_type) {
            1 -> TextureSize.Fixed(reader.next_f32())
            else -> TextureSize.Scaled(reader.next_f32())
        }

        renderer.horizontal_texture_3d_commands.add(RenderCommand.HorizontalTexture3D(
                color = color,
                x = x,
                z = z,
                rotation_rad = rotation_rad,
                server_texture_id = texture_index,
                size = size
        ))
    }
}

private fun parse_trimesh_3d_commands(reader: BufferReader, renderer: Renderer) {
    val (cylinders, sanctuaries) = reader.next_packed_int16_int16();
    for (i in 0 until cylinders) {
        renderer.trimesh_3d_commands[1].add(RenderCommand.Trimesh3D(
                x = reader.next_f32(),
                y = reader.next_f32(),
                z = reader.next_f32()
        ))
    }
    for (i in 0 until sanctuaries) {
        renderer.trimesh_3d_commands[0].add(RenderCommand.Trimesh3D(
                x = reader.next_f32(),
                y = reader.next_f32(),
                z = reader.next_f32()
        ))
    }
}


private fun parse_rectangle3d_render_commands(reader: BufferReader, renderer: Renderer) {
    for (i in 0 until reader.next_u32()) {
        val color = reader.next_color4_u8()
        val x = reader.next_f32()
        val y = reader.next_f32()
        val z = reader.next_f32()
        val rotation_rad = reader.next_f32()
        val w = reader.next_f32()
        val h = reader.next_f32()
        renderer.rectangle3d_render_commands.add(RenderCommand.Rectangle3D(
                color = color,
                x = x,
                y = y,
                z = z,
                rotation_rad = rotation_rad,
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
        val rotation_rad = reader.next_f32()
        val x = reader.next_i16()
        val y = reader.next_i16()
        val (layer, server_texture_id) = reader.next_packed_int8_int24()
        val scale = reader.next_f32()
        renderer.texture2d_render_commands.add(RenderCommand.Texture2D(
                color = color,
                offset = offset,
                rotation_rad = rotation_rad,
                x = x.toFloat(),
                y = y.toFloat(),
                server_texture_id = server_texture_id,
                scale = scale,
                layer = layer
        ))
    }
}

data class TextureId(val id: Int)

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

    val texture_db = result.asset_db.texture_db.entries
    val keys: Array<String> = js("Object").keys(texture_db)
    val map = hashMapOf<String, TextureId>()
    for (key in keys) {
        val databaseTextureEntry = TextureId(texture_db[key])
        map[key] = databaseTextureEntry
        path_to_server_gl_indices[key] = databaseTextureEntry
        server_texture_index_to_path[databaseTextureEntry.id] = key
        when (key) {
            "ground_texture_atlas" -> ground_render_command = ground_render_command.copy(
                    server_texture_atlas_id = databaseTextureEntry.id
            )
            "ground_lightmap_texture" -> ground_render_command = ground_render_command.copy(
                    server_lightmap_texture_id = databaseTextureEntry.id
            )
            "ground_tile_color_texture" -> ground_render_command = ground_render_command.copy(
                    server_tile_color_texture_id = databaseTextureEntry.id
            )
        }
    }
    return GlobalScope.launch {
        val mismatched_textures = IndexedDb.collect_mismatched_textures(map)
        console.log(mismatched_textures)
        app_state_channel.offer(AppCommand.ChangeState(ApppStateName.ReceivingMismatchingTextures,
                mismatched_textures.size))
        mismatched_textures.chunked(10).forEach { mismatched_textures_chunk ->
            socket.send(JSON.stringify(object {
                val mismatched_textures = mismatched_textures_chunk
            }))
        }
        if (mismatched_textures.isEmpty()) {
            app_state_channel.offer(AppCommand.TextureDownloaded())
        }
    }
}

var mouse_x = 0
var mouse_y = 0

fun start_frame(socket: WebSocket, renderer: Renderer) {
    console.log("start_frame")

    gl.viewport(0, 0, VIDEO_WIDTH, VIDEO_HEIGHT)
    gl.clearColor(0.3f, 0.3f, 0.5f, 1.0f)
    gl.enable(WebGLRenderingContext.DEPTH_TEST)
    gl.depthFunc(WebGLRenderingContext.LEQUAL)
    gl.enable(WebGLRenderingContext.BLEND)
    gl.blendFunc(WebGLRenderingContext.SRC_ALPHA, WebGLRenderingContext.ONE_MINUS_SRC_ALPHA)
    gl.lineWidth(2.0f)

    var last_input_tick = 0.0
    var last_fps_tick = 0.0
    var last_server_render_tick = 0.0
    val input_tickrate = 1000 / 20
    var fps = 0
    var server_fps = 0
    var server_fps_counter = 0
    var fps_counter = 0;
    var tick = { s: Double ->

    }
    Input.register_event_handlers(canvas, document)

    tick = { s: Double ->
        fps_counter++
        renderer.render(gl)

        val now = Date.now()
        if (now - last_input_tick > input_tickrate) {
            last_input_tick = now
            Input.send_input_data(socket)
        }
        if (now - last_fps_tick > 1000) {
            last_fps_tick = now
            fps = fps_counter
            fps_counter = 0
            console.info("FPS: $fps, server_fps: $server_fps")
        }
        window.requestAnimationFrame(tick)
    }
    window.requestAnimationFrame(tick)

    socket.onmessage = {
        server_fps_counter++
        val now = Date.now()
        if (now - last_server_render_tick > 1000) {
            last_server_render_tick = now
            server_fps = server_fps_counter
            server_fps_counter = 0
        }
        val reader = BufferReader(it.data as ArrayBuffer)
        renderer.clear()
        VIEW_MATRIX = reader.next_4x4matrix()
        NORMAL_MATRIX = reader.next_3x3matrix()

        while (reader.has_next()) {
            parse_partial_circle_2d_render_commands(reader, renderer)
            parse_texture2d_render_commands(reader, renderer)
            parse_rectangle_2d_render_commands(reader, renderer)
            parse_rectangle3d_render_commands(reader, renderer)
            parse_circle3d_render_commands(reader, renderer)
            parse_sprite_render_commands(reader, renderer)
            parse_number_render_commands(reader, renderer)
            parse_effect_3d_render_commands(reader, renderer)
            parse_model3d_render_commands(reader, renderer)
            parse_horizontal_texture_3d_commands(reader, renderer)
            parse_trimesh_3d_commands(reader, renderer)
        }
    }
}


private suspend fun load_texture(path: String,
                                 min_mag: Int): Triple<WebGLTexture?, Int, Int> {
    val raw_data = IndexedDb.get_texture(path) ?: return Triple(null, 0, 0)
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
    canvas.width = raw_data.w
    canvas.height = raw_data.h
    val ctx = canvas.getContext("2d") as CanvasRenderingContext2D
    val imageData =
            ImageData(Uint8ClampedArray(raw_data.raw.buffer,
                    raw_data.raw.byteOffset,
                    raw_data.raw.byteLength),
                    raw_data.w,
                    raw_data.h)
    ctx.putImageData(imageData,
            0.0, 0.0, 0.0, 0.0, canvas.width.toDouble(), canvas.height.toDouble());

    // TODO: this is too slow
    val img = ctx.getImageData(0.0, 0.0, canvas.width.toDouble(), canvas.height.toDouble())
    gl.texImage2D(WebGLRenderingContext.TEXTURE_2D,
            level = 0,
            internalformat = WebGLRenderingContext.RGBA,
            width = raw_data.w,
            height = raw_data.h,
            border = 0,
            format = WebGLRenderingContext.RGBA,
            type = WebGLRenderingContext.UNSIGNED_BYTE,
            pixels = img.data)
    return Triple(texture_obj, raw_data.w, raw_data.h)
}

fun get_or_load_server_texture(server_texture_id: Int, min_mag: Int): BrowserTextureData {
    val texture_id = server_to_client_gl_indices[server_texture_id]
    if (texture_id == null) {
        console.log("Loading texture $server_texture_id")
        // put dummy value into it so it won't trigger loading again
        server_to_client_gl_indices[server_texture_id] = dummy_texture

        job_channel.offer() {
            val maybe = server_texture_index_to_path[server_texture_id]
            if (maybe == null) {
                console.error("No path data for $server_texture_id")
            } else {
                val path = maybe
                val (new_texture_id, w, h) = load_texture(path, min_mag)
                if (new_texture_id == null) {
                    console.error("Texture was not found: $path")
                } else {
                    console.log("Texture was loaded: $path")
                    server_to_client_gl_indices[server_texture_id] =
                            BrowserTextureData(new_texture_id,
                                    w.toFloat(),
                                    h.toFloat())
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

    fun next_i32(): Int {
        val ret = view.getInt32(offset, true)
        offset += 4
        return ret
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

    fun next_packed_int16_int16(): Pair<Int, Int> {
        val packed_int = next_u32()
        val lower16 = packed_int.shl(16).ushr(16)
        val upper16 = packed_int.ushr(16)
        return upper16 to lower16
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