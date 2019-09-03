package rustarok

import org.khronos.webgl.ArrayBuffer
import org.khronos.webgl.DataView
import org.khronos.webgl.Uint8Array
import org.w3c.dom.Document
import org.w3c.dom.HTMLCanvasElement
import org.w3c.dom.WebSocket
import org.w3c.dom.events.Event
import org.w3c.dom.events.EventListener


object Input {

    enum class InputPacket {
        MouseMove,
        MouseDown,
        MouseUp,
        KeyDown,
        KeyUp,
        MouseWheel
    }


    private const val BUFFER_SIZE = 2048
    private val network_packet_buffer = ArrayBuffer(BUFFER_SIZE)
    private var buffer_offset = 0
    private val network_packet = DataView(network_packet_buffer)

    fun send_input_data(socket: WebSocket) {
        if (buffer_offset > 0) {
            socket.send(Uint8Array(network_packet_buffer, 0, buffer_offset))
            buffer_offset = 0
        }
    }

    fun register_event_handlers(canvas: HTMLCanvasElement, document: Document) {
        canvas.addEventListener("mousemove", Input::handleMouseMove)
        canvas.addEventListener("mouseup", Input::handleMouseUp)
        canvas.addEventListener("mousedown", Input::handleMouseDown)
        canvas.addEventListener("wheel", Input::handleMouseWheel)
        canvas.addEventListener("contextmenu", object : EventListener {
            override fun handleEvent(event: Event) {
                event.preventDefault()
                event.stopPropagation()
            }
        })
        document.addEventListener("keydown", Input::handleKeyDown)
        document.addEventListener("keyup", Input::handleKeyUp)
    }

    fun packet_write_i16(value: Int) {
        network_packet.setInt16(buffer_offset, value.toShort())
        buffer_offset += 2
    }

    fun packet_write_i8(value: Int) {
        network_packet.setInt8(buffer_offset, value.toByte())
        buffer_offset++
    }

    fun handleMouseWheel(e: Event) {
        e.preventDefault()
        e.stopPropagation()

        packet_write_i8(InputPacket.MouseWheel.ordinal + 1)
        packet_write_i16(-e.asDynamic().deltaY / 100)
    }


    fun handleMouseDown(e: Event) {
        e.preventDefault()
        e.stopPropagation()

//    var left_mouse = e.button == 0;
//    var middle = e.button == 1;
//    var right = e.button == 2;
        var packet = (e.asDynamic().button as Int).shl(4)
        packet = packet.or(InputPacket.MouseDown.ordinal + 1)
        packet_write_i8(packet)
    }

    fun handleMouseUp(e: Event) {
        e.preventDefault()
        e.stopPropagation()

        var packet = (e.asDynamic().button as Int).shl(4)
        packet = packet.or(InputPacket.MouseUp.ordinal + 1)
        packet_write_i8(packet)
    }

/*
pub struct Mod: u16 {
        const NOMOD = 0x0000;
        const LSHIFTMOD = 0x0001;
        const RSHIFTMOD = 0x0002;
        const LCTRLMOD = 0x0040;
        const RCTRLMOD = 0x0080;
        const LALTMOD = 0x0100;
        const RALTMOD = 0x0200;
        const LGUIMOD = 0x0400;
        const RGUIMOD = 0x0800;
        const NUMMOD = 0x1000;
        const CAPSMOD = 0x2000;
        const MODEMOD = 0x4000;
        const RESERVEDMOD = 0x8000;
    }
*/

    fun handleKeyDown(e: Event) {
        e.preventDefault()
        e.stopPropagation()

        packet_write_i8(InputPacket.KeyDown.ordinal + 1)
        packet_write_i8(code_to_sdl_scancode(e.asDynamic().code))
        // var modifiers = skip for now
        packet_write_i16(code_to_sdl_scancode(e.asDynamic().key.charCodeAt(0)))
    }

    fun handleKeyUp(e: Event) {
        e.preventDefault()
        e.stopPropagation()

        packet_write_i8(InputPacket.KeyUp.ordinal + 1)
        packet_write_i8(code_to_sdl_scancode(e.asDynamic().code))
    }

    fun handleMouseMove(e: Event) {
        e.preventDefault()
        e.stopPropagation()

        packet_write_i8(InputPacket.MouseMove.ordinal + 1)
        packet_write_i16(e.asDynamic().layerX)
        packet_write_i16(e.asDynamic().layerY)
    }

    fun code_to_sdl_scancode(code: String): Int {
        return when (code) {
            "Again" -> 121
            "AltLeft" -> 226
            "AltRight" -> 230
            "ArrowDown" -> 81
            "ArrowLeft" -> 80
            "ArrowRight" -> 79
            "ArrowUp" -> 82
            "AudioVolumeDown" -> 0
            "AudioVolumeMute" -> 262
            "AudioVolumeUp" -> 0
            "Backquote" -> 53
            "Backslash" -> 49
            "Backspace" -> 42
            "BracketLeft" -> 47
            "BracketRight" -> 48
            "CapsLock" -> 57
            "Comma" -> 54
            "ControlLeft" -> 224
            "ControlRight" -> 228
            "Copy" -> 124
            "Cut" -> 123
            "Delete" -> 76
            "Digit0" -> 39
            "Digit1" -> 30
            "Digit2" -> 31
            "Digit3" -> 32
            "Digit4" -> 33
            "Digit5" -> 34
            "Digit6" -> 35
            "Digit7" -> 36
            "Digit8" -> 37
            "Digit9" -> 38
            "Eject" -> 281
            "End" -> 77
            "Enter" -> 40
            "Equal" -> 46
            "Escape" -> 41
            "F1" -> 58
            "F10" -> 67
            "F11" -> 68
            "F12" -> 69
            "F2" -> 59
            "F3" -> 60
            "F4" -> 61
            "F5" -> 62
            "F6" -> 63
            "F7" -> 64
            "F8" -> 65
            "F9" -> 66
            "Find" -> 126
            "Fn" -> 0
            "FnLock" -> 0
            "Help" -> 117
            "Home" -> 74
            "Insert" -> 73
            "KeyA" -> 4
            "KeyB" -> 5
            "KeyC" -> 6
            "KeyD" -> 7
            "KeyE" -> 8
            "KeyF" -> 9
            "KeyG" -> 10
            "KeyH" -> 11
            "KeyI" -> 12
            "KeyJ" -> 13
            "KeyK" -> 14
            "KeyL" -> 15
            "KeyM" -> 16
            "KeyN" -> 17
            "KeyO" -> 18
            "KeyP" -> 19
            "KeyQ" -> 20
            "KeyR" -> 21
            "KeyS" -> 22
            "KeyT" -> 23
            "KeyU" -> 24
            "KeyV" -> 25
            "KeyW" -> 26
            "KeyX" -> 27
            "KeyY" -> 28
            "KeyZ" -> 29
            "MediaPlayPause" -> 261
            "MediaSelect" -> 263
            "MediaStop" -> 260
            "MediaTrackNext" -> 258
            "MediaTrackPrevious" -> 259
            "MetaLeft" -> 227
            "MetaRight" -> 231
            "Minus" -> 45
            "NumLock" -> 83
            "Numpad0" -> 98
            "Numpad1" -> 89
            "Numpad2" -> 90
            "Numpad3" -> 91
            "Numpad4" -> 92
            "Numpad5" -> 93
            "Numpad6" -> 94
            "Numpad7" -> 95
            "Numpad8" -> 96
            "Numpad9" -> 97
            "NumpadAdd" -> 0
            "NumpadBackspace" -> 187
            "NumpadClear" -> 0
            "NumpadClearEntry" -> 217
            "NumpadComma" -> 133
            "NumpadDecimal" -> 220
            "NumpadDivide" -> 84
            "NumpadEnter" -> 88
            "NumpadEqual" -> 103
            "NumpadHash" -> 204
            "NumpadMemoryAdd" -> 211
            "NumpadMemoryClear" -> 216
            "NumpadMemoryRecall" -> 209
            "NumpadMemoryStore" -> 208
            "NumpadMemorySubtract" -> 212
            "NumpadMultiply" -> 213
            "NumpadParenLeft" -> 182
            "NumpadParenRight" -> 183
            "NumpadStar" -> 85
            "NumpadSubtract" -> 0
            "Open" -> 0
            "PageDown" -> 87
            "PageUp" -> 75
            "Paste" -> 125
            "Pause" -> 72
            "Period" -> 55
            "Power" -> 102
            "PrintScreen" -> 70
            "Quote" -> 52
            "ScrollLock" -> 71
            "Select" -> 119
            "Semicolon" -> 51
            "ShiftLeft" -> 225
            "ShiftRight" -> 229
            "Slash" -> 56
            "Sleep" -> 282
            "Space" -> 44
            "Super" -> 0
            "Suspend" -> 0
            "Tab" -> 43
            "Turbo" -> 0
            "Undo" -> 122
            "Unidentified" -> 0
            "WakeUp" -> 0
            else -> 0
        }
    }
}