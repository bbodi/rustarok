use crate::cam::Camera;
use crate::components::char::{SpriteBoundingRect, SpriteRenderDescriptorComponent, Team};
use crate::components::skills::skill::Skills;
use crate::ElapsedTime;
use nalgebra::{Matrix3, Matrix4, Point3, Vector2};
use sdl2::keyboard::Scancode;
use specs::prelude::*;
use std::collections::HashMap;
use strum_macros::EnumIter;

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

pub type ScreenCoords = Vector2<u16>;
pub type WorldCoords = Vector2<f32>;

#[derive(PartialEq, Eq, Copy, Clone, EnumIter, Debug, Hash)]
pub enum SkillKey {
    Q,
    W,
    E,
    R,
    D,
    Y,
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
            SkillKey::Y => Scancode::Y,
            SkillKey::Num1 => Scancode::Num1,
            SkillKey::Num2 => Scancode::Num2,
            SkillKey::Num3 => Scancode::Num3,
        }
    }
}

#[derive(Clone)]
pub enum PlayerIntention {
    MoveTowardsMouse(WorldCoords),
    /// Move to the coordination, or if an enemy stands there, attack her.
    MoveTo(WorldCoords),
    Attack(CharEntityId),
    /// Move to the coordination, attack any enemy on the way.
    AttackTowards(WorldCoords),
    /// bool = is self cast
    Casting(Skills, bool, WorldCoords),
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

// Camera follows a controller, a Controller controls a Character
#[derive(Component)]
pub struct ControllerComponent {
    pub select_skill_target: Option<(SkillKey, Skills)>,
    pub controlled_entity: CharEntityId,
    pub next_action: Option<PlayerIntention>,
    pub last_action: Option<PlayerIntention>,
    pub repeat_next_action: bool,
    pub entities_below_cursor: EntitiesBelowCursor,
    pub bounding_rect_2d: HashMap<CharEntityId, (SpriteBoundingRect, Team)>,
    pub cell_below_cursor_walkable: bool,
    pub cursor_anim_descr: SpriteRenderDescriptorComponent,
    pub cursor_color: [u8; 3],
}

impl ControllerComponent {
    pub fn new(controlled_entity: CharEntityId) -> ControllerComponent {
        ControllerComponent {
            select_skill_target: None,
            controlled_entity,
            repeat_next_action: false,
            next_action: None,
            last_action: None,
            entities_below_cursor: EntitiesBelowCursor::new(),
            bounding_rect_2d: HashMap::new(),
            cell_below_cursor_walkable: false,
            cursor_color: [255, 255, 255],
            cursor_anim_descr: SpriteRenderDescriptorComponent {
                action_index: 0,
                animation_started: ElapsedTime(0.0),
                animation_ends_at: ElapsedTime(0.0),
                forced_duration: None,
                direction: 0,
                fps_multiplier: 1.0,
            },
        }
    }

    pub fn calc_entities_below_cursor(&mut self, self_team: Team, mouse_x: u16, mouse_y: u16) {
        self.entities_below_cursor.clear();
        for (entity_id, (bounding_rect, entity_team)) in &self.bounding_rect_2d {
            let mx = mouse_x as i32;
            let my = mouse_y as i32;
            if mx >= bounding_rect.bottom_left[0]
                && mx <= bounding_rect.top_right[0]
                && my <= bounding_rect.bottom_left[1]
                && my >= bounding_rect.top_right[1]
            {
                if *entity_team == self_team {
                    self.entities_below_cursor.add_friend(*entity_id);
                } else {
                    self.entities_below_cursor.add_enemy(*entity_id);
                }
            }
        }
        self.bounding_rect_2d.clear();
    }
}

// Camera follows a controller, a Controller controls a Character
#[derive(Component, Clone)]
pub struct CameraComponent {
    pub followed_controller: Option<ControllerEntityId>,
    pub view_matrix: Matrix4<f32>,
    pub normal_matrix: Matrix3<f32>,
    pub camera: Camera,
    pub yaw: f32,
    pub pitch: f32,
}

impl CameraComponent {
    const YAW: f32 = 270.0;
    const PITCH: f32 = -60.0;
    pub fn new(followed_controller: Option<ControllerEntityId>) -> CameraComponent {
        let camera = Camera::new(Point3::new(0.0, 40.0, 0.0));
        return CameraComponent {
            followed_controller,
            view_matrix: Matrix4::identity(), // it is filled before every frame
            normal_matrix: Matrix3::identity(), // it is filled before every frame
            camera,
            yaw: 0.0,
            pitch: 0.0,
        };
    }

    pub fn reset_y_and_angle(&mut self, projection: &Matrix4<f32>) {
        self.pitch = CameraComponent::PITCH;
        self.yaw = CameraComponent::YAW;
        self.camera.set_y(40.0);
        self.camera.rotate(self.pitch, self.yaw);
        self.camera.update_visible_z_range(projection);
    }
}

pub enum CameraMode {
    Free,
    FreeMoveButFixedAngle,
    FollowChar,
}

pub struct EntitiesBelowCursor {
    friendly: Vec<CharEntityId>,
    enemy: Vec<CharEntityId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CharEntityId(pub Entity);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ControllerEntityId(pub Entity);

impl EntitiesBelowCursor {
    pub fn new() -> EntitiesBelowCursor {
        EntitiesBelowCursor {
            friendly: Vec::with_capacity(12),
            enemy: Vec::with_capacity(12),
        }
    }

    pub fn clear(&mut self) {
        self.friendly.clear();
        self.enemy.clear();
    }

    pub fn add_friend(&mut self, entity_id: CharEntityId) {
        self.friendly.push(entity_id);
    }

    pub fn add_enemy(&mut self, entity_id: CharEntityId) {
        self.enemy.push(entity_id);
    }

    pub fn get_enemy_or_friend(&self) -> Option<CharEntityId> {
        self.enemy.get(0).or(self.friendly.get(0)).map(|it| *it)
    }

    pub fn get_friend_except(&self, except_id: CharEntityId) -> Option<CharEntityId> {
        self.friendly
            .iter()
            .filter(|it| **it != except_id)
            .next()
            .map(|it| *it)
    }

    pub fn get_friend(&self) -> Option<CharEntityId> {
        self.friendly.get(0).map(|it| *it)
    }

    pub fn get_enemy(&self) -> Option<CharEntityId> {
        self.enemy.get(0).map(|it| *it)
    }
}

#[derive(Component)]
pub struct HumanInputComponent {
    pub is_console_open: bool,
    pub username: String,
    pub inputs: Vec<sdl2::event::Event>,
    skills_for_keys: [Option<Skills>; 9],
    pub cast_mode: CastMode,
    keys: HashMap<Scancode, KeyState>,
    keys_released_in_prev_frame: Vec<Scancode>,
    keys_pressed_in_prev_frame: Vec<Scancode>,
    pub text: String,
    pub mouse_wheel: i32,
    pub camera_movement_mode: CameraMode,
    pub left_mouse_down: bool,
    pub right_mouse_down: bool,
    pub left_mouse_pressed: bool,
    pub right_mouse_pressed: bool,
    pub left_mouse_released: bool,
    pub right_mouse_released: bool,
    pub last_mouse_x: u16,
    pub last_mouse_y: u16,
    pub delta_mouse_x: i32,
    pub delta_mouse_y: i32,
    pub mouse_world_pos: WorldCoords,
}

impl Drop for HumanInputComponent {
    fn drop(&mut self) {
        log::info!("HumanInputComponent DROPPED");
    }
}

impl HumanInputComponent {
    pub fn new(username: &str) -> HumanInputComponent {
        HumanInputComponent {
            is_console_open: false,
            username: username.to_owned(),
            cast_mode: CastMode::Normal,
            inputs: vec![],
            skills_for_keys: Default::default(),
            camera_movement_mode: CameraMode::FollowChar,
            keys: HumanInputComponent::init_keystates(),
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
            mouse_world_pos: v2!(0, 0),
            mouse_wheel: 0,
            delta_mouse_x: 0,
            delta_mouse_y: 0,
            text: String::new(),
        }
    }

    pub fn get_skill_for_key(&self, skill_key: SkillKey) -> Option<Skills> {
        self.skills_for_keys[skill_key as usize]
    }

    pub fn assign_skill(&mut self, skill_key: SkillKey, skill: Skills) {
        self.skills_for_keys[skill_key as usize] = Some(skill);
    }

    pub fn mouse_pos(&self) -> ScreenCoords {
        Vector2::new(self.last_mouse_x, self.last_mouse_y)
    }

    pub fn cleanup_released_keys(&mut self) {
        for key in self.keys_released_in_prev_frame.drain(..) {
            self.keys.get_mut(&key).unwrap().just_released = false;
        }
        for key in self.keys_pressed_in_prev_frame.drain(..) {
            self.keys.get_mut(&key).unwrap().just_pressed = false;
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
