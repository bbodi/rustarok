// animation index, action index

pub struct CursorFrame(pub usize, pub usize);
pub const CURSOR_NORMAL: CursorFrame = CursorFrame(0, 0);
// pub const CURSOR_TALK: CursorFrame = CursorFrame(0, 1);
pub const CURSOR_CLICK: CursorFrame = CursorFrame(0, 2);
// pub const CURSOR_LOCK: CursorFrame = CursorFrame(0, 3);
// pub const CURSOR_ROTATE: CursorFrame = CursorFrame(0, 4);
// pub const CURSOR_ATTACK: CursorFrame = CursorFrame(0, 5);
// pub const CURSOR_DOOR: CursorFrame = CursorFrame(0, 7);
pub const CURSOR_STOP: CursorFrame = CursorFrame(0, 8);
// pub const CURSOR_PICK: CursorFrame = CursorFrame(0, 9);
pub const CURSOR_TARGET: CursorFrame = CursorFrame(1, 10);
// pub const CURSOR_NO: CursorFrame = CursorFrame(1, 13);
