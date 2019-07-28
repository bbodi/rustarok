var canvas = document.getElementById('main_canvas');
var ctx = canvas.getContext('2d');
var last_tick = 0;
var tickrate = 1000 / 20;
var network_packet_buffer = new ArrayBuffer(1024);
var buffer_offset = 0;
var network_packet = new Uint8Array(network_packet_buffer);

let socket = new WebSocket("ws://127.0.0.1:6969");
socket.binaryType = "arraybuffer";

function packet_write_i16(value) {
    network_packet[buffer_offset] = (value >> 8) & 0xFF;
    network_packet[buffer_offset + 1] = value & 0xFF;
    buffer_offset += 2;
}

function packet_write_i8(value) {
    network_packet[buffer_offset] = value;
    buffer_offset++;
}

function flush_packets() {
    socket.send(new Uint8Array(network_packet_buffer, 0, buffer_offset));
    buffer_offset = 0;
}

socket.onopen = function(e) {
    last_tick = Date.now();
    canvas.addEventListener('mousemove', handleMouseMove);
    canvas.addEventListener('mouseup', handleMouseUp);
    canvas.addEventListener('mousedown', handleMouseDown);
    canvas.addEventListener('contextmenu', function(e) {
        e.preventDefault();
        e.stopPropagation();
        return false;
    });
    document.addEventListener('keydown', handleKeyDown);
    document.addEventListener('keyup', handleKeyUp);
    tick();
};

function tick() {
    // request another frame
    requestAnimationFrame(tick);

    // calc elapsed time since last loop

    var now = Date.now();
    var elapsed = now - last_tick;

    if (elapsed > tickrate) {
        last_tick = now - (elapsed % tickrate);
        flush_packets();
    }
}

socket.onmessage = function(event) {
    var blob = event.data;
    var imageData = new ImageData(new Uint8ClampedArray(blob), 1024, 768);
    ctx.putImageData(imageData, 0, 0);
};

socket.onclose = function(event) {
    if (event.wasClean) {
        alert(`[close] Connection closed cleanly, code=${event.code} reason=${event.reason}`);
    } else {
        // e.g. server process killed or network down
        // event.code is usually 1006 in this case
        alert('[close] Connection died');
    }
};

socket.onerror = function(error) {
    alert(`[error] ${error.message}`);
};

function handleMouseDown(e) {
    e.preventDefault();
    e.stopPropagation();

//    var left_mouse = e.button == 0;
//    var middle = e.button == 1;
//    var right = e.button == 2;
    console.info("down: ", e.button);
    var packet = e.button << 4;
    packet |= 2; // this is the "id" of the packet
    packet_write_i8(packet);
}

function handleMouseUp(e) {
    e.preventDefault();
    e.stopPropagation();

    console.info("up: ", e.button);
    var packet = e.button << 4;
    packet |= 3; // this is the "id" of the packet
    packet_write_i8(packet);
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
function handleKeyDown(e) {
    packet_write_i8(4);
    packet_write_i8(code_to_sdl_scancode(e.code));
    // var modifiers = skip for now
    packet_write_i16(code_to_sdl_scancode(e.key.charCodeAt(0)));
}

function handleKeyUp(e) {
    packet_write_i8(5);
    packet_write_i8(code_to_sdl_scancode(e.code));
}

function code_to_sdl_scancode(code) {
    switch (code) {
        case "Again":
            return 121;
        case "AltLeft":
            return 226;
        case "AltRight":
            return 230;
        case "ArrowDown":
            return 81;
        case "ArrowLeft":
            return 80;
        case "ArrowRight":
            return 79;
        case "ArrowUp":
            return 82;
        case "AudioVolumeDown":
            return 0;
        case "AudioVolumeMute":
            return 262;
        case "AudioVolumeUp":
            return 0;
        case "Backquote":
            return 53;
        case "Backslash":
            return 49;
        case "Backspace":
            return 42;
        case "BracketLeft":
            return 47;
        case "BracketRight":
            return 48;
        case "CapsLock":
            return 57;
        case "Comma":
            return 54;
        case "ControlLeft":
            return 224;
        case "ControlRight":
            return 228;
        case "Copy":
            return 124;
        case "Cut":
            return 123;
        case "Delete":
            return 76;
        case "Digit0":
            return 39;
        case "Digit1":
            return 30;
        case "Digit2":
            return 31;
        case "Digit3":
            return 32;
        case "Digit4":
            return 33;
        case "Digit5":
            return 34;
        case "Digit6":
            return 35;
        case "Digit7":
            return 36;
        case "Digit8":
            return 37;
        case "Digit9":
            return 38;
        case "Eject":
            return 281;
        case "End":
            return 77;
        case "Enter":
            return 40;
        case "Equal":
            return 46;
        case "Escape":
            return 41;
        case "F1":
            return 58;
        case "F10":
            return 67;
        case "F11":
            return 68;
        case "F12":
            return 69;
        case "F2":
            return 59;
        case "F3":
            return 60;
        case "F4":
            return 61;
        case "F5":
            return 62;
        case "F6":
            return 63;
        case "F7":
            return 64;
        case "F8":
            return 65;
        case "F9":
            return 66;
        case "Find":
            return 126;
        case "Fn":
            return 0;
        case "FnLock":
            return 0;
        case "Help":
            return 117;
        case "Home":
            return 74;
        case "Insert":
            return 73;
        case "KeyA":
            return 4;
        case "KeyB":
            return 5;
        case "KeyC":
            return 6;
        case "KeyD":
            return 7;
        case "KeyE":
            return 8;
        case "KeyF":
            return 9;
        case "KeyG":
            return 10;
        case "KeyH":
            return 11;
        case "KeyI":
            return 12;
        case "KeyJ":
            return 13;
        case "KeyK":
            return 14;
        case "KeyL":
            return 15;
        case "KeyM":
            return 16;
        case "KeyN":
            return 17;
        case "KeyO":
            return 18;
        case "KeyP":
            return 19;
        case "KeyQ":
            return 20;
        case "KeyR":
            return 21;
        case "KeyS":
            return 22;
        case "KeyT":
            return 23;
        case "KeyU":
            return 24;
        case "KeyV":
            return 25;
        case "KeyW":
            return 26;
        case "KeyX":
            return 27;
        case "KeyY":
            return 28;
        case "KeyZ":
            return 29;
        case "MediaPlayPause":
            return 261;
        case "MediaSelect":
            return 263;
        case "MediaStop":
            return 260;
        case "MediaTrackNext":
            return 258;
        case "MediaTrackPrevious":
            return 259;
        case "MetaLeft":
            return 227;
        case "MetaRight":
            return 231;
        case "Minus":
            return 45;
        case "NumLock":
            return 83;
        case "Numpad0":
            return 98;
        case "Numpad1":
            return 89;
        case "Numpad2":
            return 90;
        case "Numpad3":
            return 91;
        case "Numpad4":
            return 92;
        case "Numpad5":
            return 93;
        case "Numpad6":
            return 94;
        case "Numpad7":
            return 95;
        case "Numpad8":
            return 96;
        case "Numpad9":
            return 97;
        case "NumpadAdd":
            return 0;
        case "NumpadBackspace":
            return 187;
        case "NumpadClear":
            return 0;
        case "NumpadClearEntry":
            return 217;
        case "NumpadComma":
            return 133;
        case "NumpadDecimal":
            return 220;
        case "NumpadDivide":
            return 84;
        case "NumpadEnter":
            return 88;
        case "NumpadEqual":
            return 103;
        case "NumpadHash":
            return 204;
        case "NumpadMemoryAdd":
            return 211;
        case "NumpadMemoryClear":
            return 216;
        case "NumpadMemoryRecall":
            return 209;
        case "NumpadMemoryStore":
            return 208;
        case "NumpadMemorySubtract":
            return 212;
        case "NumpadMultiply":
            return 213;
        case "NumpadParenLeft":
            return 182;
        case "NumpadParenRight":
            return 183;
        case "NumpadStar":
            return 85;
        case "NumpadSubtract":
            return 0;
        case "Open":
            return 0;
        case "PageDown":
            return 87;
        case "PageUp":
            return 75;
        case "Paste":
            return 125;
        case "Pause":
            return 72;
        case "Period":
            return 55;
        case "Power":
            return 102;
        case "PrintScreen":
            return 70;
        case "Quote":
            return 52;
        case "ScrollLock":
            return 71;
        case "Select":
            return 119;
        case "Semicolon":
            return 51;
        case "ShiftLeft":
            return 225;
        case "ShiftRight":
            return 229;
        case "Slash":
            return 56;
        case "Sleep":
            return 282;
        case "Space":
            return 44;
        case "Super":
            return 0;
        case "Suspend":
            return 0;
        case "Tab":
            return 43;
        case "Turbo":
            return 0;
        case "Undo":
            return 122;
        case "Unidentified":
            return 0;
        case "WakeUp":
            return 0;
    }
}

function handleMouseMove(e) {
    e.preventDefault();
    e.stopPropagation();

    packet_write_i8(1);
    packet_write_i16(e.clientX);
    packet_write_i16(e.clientY);
}