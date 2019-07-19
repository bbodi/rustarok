use specs::Entity;
use crate::cam::Camera;
use std::collections::{HashSet, HashMap};
use sdl2::keyboard::Scancode;
use nalgebra::{Point3, Point2};
use specs::prelude::*;

#[derive(Default)]
pub struct KeyState {
    pub down: bool,
    pub just_pressed: bool,
    pub just_released: bool,
}

impl KeyState {
    pub fn pressed(&mut self) -> bool {
        self.just_pressed = !self.down;
        self.just_released = false;
        self.down = true;
        return self.just_pressed;
    }

    pub fn released(&mut self) -> bool {
        self.just_pressed = false;
        self.just_released = self.down;
        self.down = false;
        return self.just_released;
    }
}

pub type ScreenCoords = Point2<u16>;
pub type WorldCoords = Point2<f32>;

#[derive(PartialEq, Eq, Copy, Clone)]
pub enum SkillKey {
    Q,
    W,
    E,
    R,
    D,
    Num1,
    Num2,
    Num3,
}

impl SkillKey {
    pub fn scancode(&self) -> Scancode {
        match self {
            SkillKey::Q => Scancode::Q,
            SkillKey::W => Scancode::W,
            SkillKey::E => Scancode::E,
            SkillKey::R => Scancode::R,
            SkillKey::D => Scancode::D,
            SkillKey::Num1 => Scancode::Num1,
            SkillKey::Num2 => Scancode::Num2,
            SkillKey::Num3 => Scancode::Num3,
        }
    }
}

pub const ALL_SKILL_KEYS: [SkillKey; 8] = [SkillKey::Q,
    SkillKey::W,
    SkillKey::E,
    SkillKey::R,
    SkillKey::D,
    SkillKey::Num1,
    SkillKey::Num2,
    SkillKey::Num3
];

pub enum ControllerAction {
    MoveTowardsMouse(ScreenCoords),
    /// Move to the coordination, or if an enemy stands there, attack her.
    MoveOrAttackTo(ScreenCoords),
    /// Move to the coordination, attack any enemy on the way.
    AttackTo(ScreenCoords),
    CastingSelectTarget(SkillKey),
    CancelCastingSelectTarget,
    Casting(SkillKey),
    LeftClick,
}

#[derive(PartialEq, Eq)]
pub enum CastMode {
    /// Pressing the skill key moves you into target selection mode, then
    /// pressing LMB will cast the skill
    Normal,
    /// Pressing the skill key moves you into target selection mode, then
    ///  releasing the key will cast the skill
    OnKeyRelease,
    /// Pressing the skill key casts the skill immediately
    OnKeyPress,
}

#[derive(Component)]
pub struct ControllerComponent {
    pub char: Entity,
    pub camera: Camera,
    pub inputs: Vec<sdl2::event::Event>,
    pub next_action: Option<ControllerAction>,
    pub last_action: Option<ControllerAction>,
    pub is_casting_selection: Option<SkillKey>,
    pub cast_mode: CastMode,
    keys: HashMap<Scancode, KeyState>,
    keys_released_in_prev_frame: Vec<Scancode>,
    keys_pressed_in_prev_frame: Vec<Scancode>,
    pub left_mouse_down: bool,
    pub right_mouse_down: bool,
    pub left_mouse_pressed: bool,
    pub right_mouse_pressed: bool,
    pub left_mouse_released: bool,
    pub right_mouse_released: bool,
    pub last_mouse_x: u16,
    pub last_mouse_y: u16,
    pub mouse_world_pos: WorldCoords,
    pub entity_below_cursor: Option<Entity>,
    pub cell_below_cursor_walkable: bool,
    pub yaw: f32,
    pub pitch: f32,
}

impl ControllerComponent {
    pub fn mouse_pos(&self) -> ScreenCoords {
        Point2::new(self.last_mouse_x, self.last_mouse_x)
    }

    pub fn cleanup_released_keys(&mut self) {
        for key in self.keys_released_in_prev_frame.drain(..) {
            self.keys.get_mut(&key).unwrap().just_released = false;
        }
        for key in self.keys_pressed_in_prev_frame.drain(..) {
            self.keys.get_mut(&key).unwrap().just_pressed = false;
        }
    }


    pub fn new(char: Entity, x: f32, z: f32) -> ControllerComponent {
        let pitch = -60.0;
        let yaw = 270.0;
        let mut camera = Camera::new(Point3::new(x, 20.0, z));

        camera.rotate(pitch, yaw);
        ControllerComponent {
            char,
            camera,
            cast_mode: CastMode::Normal,
            inputs: vec![],
            keys: ControllerComponent::init_keystates(),
            keys_released_in_prev_frame: vec![],
            keys_pressed_in_prev_frame: vec![],
            left_mouse_down: false,
            right_mouse_down: false,
            left_mouse_released: false,
            left_mouse_pressed: false,
            right_mouse_pressed: false,
            right_mouse_released: false,
            last_mouse_x: 400,
            last_mouse_y: 300,
            yaw,
            pitch,
            next_action: None,
            last_action: None,
            entity_below_cursor: None,
            cell_below_cursor_walkable: false,
            is_casting_selection: None,
            mouse_world_pos: Point2::new(0.0, 0.0)
        }
    }

    pub fn key_pressed(&mut self, key: Scancode) {
        if self.keys.get_mut(&key).unwrap().pressed() {
            self.keys_pressed_in_prev_frame.push(key);
        }
    }

    pub fn key_released(&mut self, key: Scancode) {
        if self.keys.get_mut(&key).unwrap().released() {
            self.keys_released_in_prev_frame.push(key);
        }
    }

    pub fn is_key_down(&self, key: Scancode) -> bool {
        self.keys[&key].down
    }

    pub fn is_key_up(&self, key: Scancode) -> bool {
        !self.keys[&key].down
    }

    pub fn is_key_just_released(&self, key: Scancode) -> bool {
        self.keys[&key].just_released
    }

    pub fn is_key_just_pressed(&self, key: Scancode) -> bool {
        self.keys[&key].just_pressed
    }

    fn init_keystates() -> HashMap<Scancode, KeyState> {
        let mut key_map = HashMap::<Scancode, KeyState>::new();
        key_map.insert(Scancode::A, KeyState::default());
        key_map.insert(Scancode::B, KeyState::default());
        key_map.insert(Scancode::C, KeyState::default());
        key_map.insert(Scancode::D, KeyState::default());
        key_map.insert(Scancode::E, KeyState::default());
        key_map.insert(Scancode::F, KeyState::default());
        key_map.insert(Scancode::G, KeyState::default());
        key_map.insert(Scancode::H, KeyState::default());
        key_map.insert(Scancode::I, KeyState::default());
        key_map.insert(Scancode::J, KeyState::default());
        key_map.insert(Scancode::K, KeyState::default());
        key_map.insert(Scancode::L, KeyState::default());
        key_map.insert(Scancode::M, KeyState::default());
        key_map.insert(Scancode::N, KeyState::default());
        key_map.insert(Scancode::O, KeyState::default());
        key_map.insert(Scancode::P, KeyState::default());
        key_map.insert(Scancode::Q, KeyState::default());
        key_map.insert(Scancode::R, KeyState::default());
        key_map.insert(Scancode::S, KeyState::default());
        key_map.insert(Scancode::T, KeyState::default());
        key_map.insert(Scancode::U, KeyState::default());
        key_map.insert(Scancode::V, KeyState::default());
        key_map.insert(Scancode::W, KeyState::default());
        key_map.insert(Scancode::X, KeyState::default());
        key_map.insert(Scancode::Y, KeyState::default());
        key_map.insert(Scancode::Z, KeyState::default());
        key_map.insert(Scancode::Num1, KeyState::default());
        key_map.insert(Scancode::Num2, KeyState::default());
        key_map.insert(Scancode::Num3, KeyState::default());
        key_map.insert(Scancode::Num4, KeyState::default());
        key_map.insert(Scancode::Num5, KeyState::default());
        key_map.insert(Scancode::Num6, KeyState::default());
        key_map.insert(Scancode::Num7, KeyState::default());
        key_map.insert(Scancode::Num8, KeyState::default());
        key_map.insert(Scancode::Num9, KeyState::default());
        key_map.insert(Scancode::Num0, KeyState::default());
        key_map.insert(Scancode::Return, KeyState::default());
        key_map.insert(Scancode::Escape, KeyState::default());
        key_map.insert(Scancode::Backspace, KeyState::default());
        key_map.insert(Scancode::Tab, KeyState::default());
        key_map.insert(Scancode::Space, KeyState::default());
        key_map.insert(Scancode::Minus, KeyState::default());
        key_map.insert(Scancode::Equals, KeyState::default());
        key_map.insert(Scancode::LeftBracket, KeyState::default());
        key_map.insert(Scancode::RightBracket, KeyState::default());
        key_map.insert(Scancode::Backslash, KeyState::default());
        key_map.insert(Scancode::NonUsHash, KeyState::default());
        key_map.insert(Scancode::Semicolon, KeyState::default());
        key_map.insert(Scancode::Apostrophe, KeyState::default());
        key_map.insert(Scancode::Grave, KeyState::default());
        key_map.insert(Scancode::Comma, KeyState::default());
        key_map.insert(Scancode::Period, KeyState::default());
        key_map.insert(Scancode::Slash, KeyState::default());
        key_map.insert(Scancode::CapsLock, KeyState::default());
        key_map.insert(Scancode::F1, KeyState::default());
        key_map.insert(Scancode::F2, KeyState::default());
        key_map.insert(Scancode::F3, KeyState::default());
        key_map.insert(Scancode::F4, KeyState::default());
        key_map.insert(Scancode::F5, KeyState::default());
        key_map.insert(Scancode::F6, KeyState::default());
        key_map.insert(Scancode::F7, KeyState::default());
        key_map.insert(Scancode::F8, KeyState::default());
        key_map.insert(Scancode::F9, KeyState::default());
        key_map.insert(Scancode::F10, KeyState::default());
        key_map.insert(Scancode::F11, KeyState::default());
        key_map.insert(Scancode::F12, KeyState::default());
        key_map.insert(Scancode::PrintScreen, KeyState::default());
        key_map.insert(Scancode::ScrollLock, KeyState::default());
        key_map.insert(Scancode::Pause, KeyState::default());
        key_map.insert(Scancode::Insert, KeyState::default());
        key_map.insert(Scancode::Home, KeyState::default());
        key_map.insert(Scancode::PageUp, KeyState::default());
        key_map.insert(Scancode::Delete, KeyState::default());
        key_map.insert(Scancode::End, KeyState::default());
        key_map.insert(Scancode::PageDown, KeyState::default());
        key_map.insert(Scancode::Right, KeyState::default());
        key_map.insert(Scancode::Left, KeyState::default());
        key_map.insert(Scancode::Down, KeyState::default());
        key_map.insert(Scancode::Up, KeyState::default());
        key_map.insert(Scancode::NumLockClear, KeyState::default());
        key_map.insert(Scancode::KpDivide, KeyState::default());
        key_map.insert(Scancode::KpMultiply, KeyState::default());
        key_map.insert(Scancode::KpMinus, KeyState::default());
        key_map.insert(Scancode::KpPlus, KeyState::default());
        key_map.insert(Scancode::KpEnter, KeyState::default());
        key_map.insert(Scancode::Kp1, KeyState::default());
        key_map.insert(Scancode::Kp2, KeyState::default());
        key_map.insert(Scancode::Kp3, KeyState::default());
        key_map.insert(Scancode::Kp4, KeyState::default());
        key_map.insert(Scancode::Kp5, KeyState::default());
        key_map.insert(Scancode::Kp6, KeyState::default());
        key_map.insert(Scancode::Kp7, KeyState::default());
        key_map.insert(Scancode::Kp8, KeyState::default());
        key_map.insert(Scancode::Kp9, KeyState::default());
        key_map.insert(Scancode::Kp0, KeyState::default());
        key_map.insert(Scancode::KpPeriod, KeyState::default());
        key_map.insert(Scancode::NonUsBackslash, KeyState::default());
        key_map.insert(Scancode::Application, KeyState::default());
        key_map.insert(Scancode::Power, KeyState::default());
        key_map.insert(Scancode::KpEquals, KeyState::default());
        key_map.insert(Scancode::F13, KeyState::default());
        key_map.insert(Scancode::F14, KeyState::default());
        key_map.insert(Scancode::F15, KeyState::default());
        key_map.insert(Scancode::F16, KeyState::default());
        key_map.insert(Scancode::F17, KeyState::default());
        key_map.insert(Scancode::F18, KeyState::default());
        key_map.insert(Scancode::F19, KeyState::default());
        key_map.insert(Scancode::F20, KeyState::default());
        key_map.insert(Scancode::F21, KeyState::default());
        key_map.insert(Scancode::F22, KeyState::default());
        key_map.insert(Scancode::F23, KeyState::default());
        key_map.insert(Scancode::F24, KeyState::default());
        key_map.insert(Scancode::Execute, KeyState::default());
        key_map.insert(Scancode::Help, KeyState::default());
        key_map.insert(Scancode::Menu, KeyState::default());
        key_map.insert(Scancode::Select, KeyState::default());
        key_map.insert(Scancode::Stop, KeyState::default());
        key_map.insert(Scancode::Again, KeyState::default());
        key_map.insert(Scancode::Undo, KeyState::default());
        key_map.insert(Scancode::Cut, KeyState::default());
        key_map.insert(Scancode::Copy, KeyState::default());
        key_map.insert(Scancode::Paste, KeyState::default());
        key_map.insert(Scancode::Find, KeyState::default());
        key_map.insert(Scancode::Mute, KeyState::default());
        key_map.insert(Scancode::VolumeUp, KeyState::default());
        key_map.insert(Scancode::VolumeDown, KeyState::default());
        key_map.insert(Scancode::KpComma, KeyState::default());
        key_map.insert(Scancode::KpEqualsAS400, KeyState::default());
        key_map.insert(Scancode::International1, KeyState::default());
        key_map.insert(Scancode::International2, KeyState::default());
        key_map.insert(Scancode::International3, KeyState::default());
        key_map.insert(Scancode::International4, KeyState::default());
        key_map.insert(Scancode::International5, KeyState::default());
        key_map.insert(Scancode::International6, KeyState::default());
        key_map.insert(Scancode::International7, KeyState::default());
        key_map.insert(Scancode::International8, KeyState::default());
        key_map.insert(Scancode::International9, KeyState::default());
        key_map.insert(Scancode::Lang1, KeyState::default());
        key_map.insert(Scancode::Lang2, KeyState::default());
        key_map.insert(Scancode::Lang3, KeyState::default());
        key_map.insert(Scancode::Lang4, KeyState::default());
        key_map.insert(Scancode::Lang5, KeyState::default());
        key_map.insert(Scancode::Lang6, KeyState::default());
        key_map.insert(Scancode::Lang7, KeyState::default());
        key_map.insert(Scancode::Lang8, KeyState::default());
        key_map.insert(Scancode::Lang9, KeyState::default());
        key_map.insert(Scancode::AltErase, KeyState::default());
        key_map.insert(Scancode::SysReq, KeyState::default());
        key_map.insert(Scancode::Cancel, KeyState::default());
        key_map.insert(Scancode::Clear, KeyState::default());
        key_map.insert(Scancode::Prior, KeyState::default());
        key_map.insert(Scancode::Return2, KeyState::default());
        key_map.insert(Scancode::Separator, KeyState::default());
        key_map.insert(Scancode::Out, KeyState::default());
        key_map.insert(Scancode::Oper, KeyState::default());
        key_map.insert(Scancode::ClearAgain, KeyState::default());
        key_map.insert(Scancode::CrSel, KeyState::default());
        key_map.insert(Scancode::ExSel, KeyState::default());
        key_map.insert(Scancode::Kp00, KeyState::default());
        key_map.insert(Scancode::Kp000, KeyState::default());
        key_map.insert(Scancode::ThousandsSeparator, KeyState::default());
        key_map.insert(Scancode::DecimalSeparator, KeyState::default());
        key_map.insert(Scancode::CurrencyUnit, KeyState::default());
        key_map.insert(Scancode::CurrencySubUnit, KeyState::default());
        key_map.insert(Scancode::KpLeftParen, KeyState::default());
        key_map.insert(Scancode::KpRightParen, KeyState::default());
        key_map.insert(Scancode::KpLeftBrace, KeyState::default());
        key_map.insert(Scancode::KpRightBrace, KeyState::default());
        key_map.insert(Scancode::KpTab, KeyState::default());
        key_map.insert(Scancode::KpBackspace, KeyState::default());
        key_map.insert(Scancode::KpA, KeyState::default());
        key_map.insert(Scancode::KpB, KeyState::default());
        key_map.insert(Scancode::KpC, KeyState::default());
        key_map.insert(Scancode::KpD, KeyState::default());
        key_map.insert(Scancode::KpE, KeyState::default());
        key_map.insert(Scancode::KpF, KeyState::default());
        key_map.insert(Scancode::KpXor, KeyState::default());
        key_map.insert(Scancode::KpPower, KeyState::default());
        key_map.insert(Scancode::KpPercent, KeyState::default());
        key_map.insert(Scancode::KpLess, KeyState::default());
        key_map.insert(Scancode::KpGreater, KeyState::default());
        key_map.insert(Scancode::KpAmpersand, KeyState::default());
        key_map.insert(Scancode::KpDblAmpersand, KeyState::default());
        key_map.insert(Scancode::KpVerticalBar, KeyState::default());
        key_map.insert(Scancode::KpDblVerticalBar, KeyState::default());
        key_map.insert(Scancode::KpColon, KeyState::default());
        key_map.insert(Scancode::KpHash, KeyState::default());
        key_map.insert(Scancode::KpSpace, KeyState::default());
        key_map.insert(Scancode::KpAt, KeyState::default());
        key_map.insert(Scancode::KpExclam, KeyState::default());
        key_map.insert(Scancode::KpMemStore, KeyState::default());
        key_map.insert(Scancode::KpMemRecall, KeyState::default());
        key_map.insert(Scancode::KpMemClear, KeyState::default());
        key_map.insert(Scancode::KpMemAdd, KeyState::default());
        key_map.insert(Scancode::KpMemSubtract, KeyState::default());
        key_map.insert(Scancode::KpMemMultiply, KeyState::default());
        key_map.insert(Scancode::KpMemDivide, KeyState::default());
        key_map.insert(Scancode::KpPlusMinus, KeyState::default());
        key_map.insert(Scancode::KpClear, KeyState::default());
        key_map.insert(Scancode::KpClearEntry, KeyState::default());
        key_map.insert(Scancode::KpBinary, KeyState::default());
        key_map.insert(Scancode::KpOctal, KeyState::default());
        key_map.insert(Scancode::KpDecimal, KeyState::default());
        key_map.insert(Scancode::KpHexadecimal, KeyState::default());
        key_map.insert(Scancode::LCtrl, KeyState::default());
        key_map.insert(Scancode::LShift, KeyState::default());
        key_map.insert(Scancode::LAlt, KeyState::default());
        key_map.insert(Scancode::LGui, KeyState::default());
        key_map.insert(Scancode::RCtrl, KeyState::default());
        key_map.insert(Scancode::RShift, KeyState::default());
        key_map.insert(Scancode::RAlt, KeyState::default());
        key_map.insert(Scancode::RGui, KeyState::default());
        key_map.insert(Scancode::Mode, KeyState::default());
        key_map.insert(Scancode::AudioNext, KeyState::default());
        key_map.insert(Scancode::AudioPrev, KeyState::default());
        key_map.insert(Scancode::AudioStop, KeyState::default());
        key_map.insert(Scancode::AudioPlay, KeyState::default());
        key_map.insert(Scancode::AudioMute, KeyState::default());
        key_map.insert(Scancode::MediaSelect, KeyState::default());
        key_map.insert(Scancode::Www, KeyState::default());
        key_map.insert(Scancode::Mail, KeyState::default());
        key_map.insert(Scancode::Calculator, KeyState::default());
        key_map.insert(Scancode::Computer, KeyState::default());
        key_map.insert(Scancode::AcSearch, KeyState::default());
        key_map.insert(Scancode::AcHome, KeyState::default());
        key_map.insert(Scancode::AcBack, KeyState::default());
        key_map.insert(Scancode::AcForward, KeyState::default());
        key_map.insert(Scancode::AcStop, KeyState::default());
        key_map.insert(Scancode::AcRefresh, KeyState::default());
        key_map.insert(Scancode::AcBookmarks, KeyState::default());
        key_map.insert(Scancode::BrightnessDown, KeyState::default());
        key_map.insert(Scancode::BrightnessUp, KeyState::default());
        key_map.insert(Scancode::DisplaySwitch, KeyState::default());
        key_map.insert(Scancode::KbdIllumToggle, KeyState::default());
        key_map.insert(Scancode::KbdIllumDown, KeyState::default());
        key_map.insert(Scancode::KbdIllumUp, KeyState::default());
        key_map.insert(Scancode::Eject, KeyState::default());
        key_map.insert(Scancode::Sleep, KeyState::default());
        key_map.insert(Scancode::App1, KeyState::default());
        key_map.insert(Scancode::App2, KeyState::default());
        key_map.insert(Scancode::Num, KeyState::default());
        return key_map;
    }
}
