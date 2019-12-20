use crate::cam::Camera;
use crate::components::char::{SpriteBoundingRect, SpriteRenderDescriptorComponent};
use crate::components::skills::skills::Skills;

use crate::ElapsedTime;
use rustarok_common::common::{v2, v3, Mat3, Mat4, Vec2, Vec2u};
use rustarok_common::components::char::{CharDir, CharEntityId, ControllerEntityId, Team};
use rustarok_common::components::controller::PlayerIntention;
use sdl2::keyboard::Scancode;
use serde::Deserialize;
use specs::prelude::*;
use std::collections::HashMap;
use strum_macros::Display;
use strum_macros::EnumCount;
use strum_macros::EnumIter;

#[derive(Default, Copy, Clone)]
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

#[derive(PartialEq, Eq, Copy, Clone, EnumIter, Display, Hash, EnumCount)]
pub enum SkillKey {
    A,
    Q,
    W,
    E,
    R,
    D,
    Y,
    #[strum(serialize = "1")]
    Num1,
    #[strum(serialize = "2")]
    Num2,
    #[strum(serialize = "3")]
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
            SkillKey::A => Scancode::A,
        }
    }
}

#[derive(PartialEq, Eq, Debug, Deserialize, Clone, Copy)]
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
pub struct LocalPlayerControllerComponent {
    pub select_skill_target: Option<(SkillKey, Skills)>,
    // only client
    pub last_intention: Option<PlayerIntention>,
    pub repeat_next_action: bool,
    pub entities_below_cursor: EntitiesBelowCursor,
    pub bounding_rect_2d: HashMap<CharEntityId, (SpriteBoundingRect, Team)>,
    pub cell_below_cursor_walkable: bool,
    pub cursor_anim_descr: SpriteRenderDescriptorComponent,
    pub cursor_color: [u8; 3],
}

impl LocalPlayerControllerComponent {
    pub fn new() -> LocalPlayerControllerComponent {
        LocalPlayerControllerComponent {
            select_skill_target: None,
            repeat_next_action: false,
            last_intention: None,
            entities_below_cursor: EntitiesBelowCursor::new(),
            bounding_rect_2d: HashMap::new(),
            cell_below_cursor_walkable: false,
            cursor_color: [255, 255, 255],
            cursor_anim_descr: SpriteRenderDescriptorComponent {
                action_index: 0,
                animation_started: ElapsedTime(0.0),
                animation_ends_at: ElapsedTime(0.0),
                forced_duration: None,
                direction: CharDir::South,
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
                if entity_team.is_ally_to(self_team) {
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
    pub view_matrix: Mat4,
    pub normal_matrix: Mat3,
    pub camera: Camera,
    pub yaw: f32,
    pub pitch: f32,
}

impl CameraComponent {
    const YAW: f32 = 270.0;
    const PITCH: f32 = -60.0;
    pub fn new(followed_controller: Option<ControllerEntityId>) -> CameraComponent {
        let camera = Camera::new(v3(0.0, 40.0, 0.0));
        return CameraComponent {
            followed_controller,
            view_matrix: Mat4::identity(), // it is filled before every frame
            normal_matrix: Mat3::identity(), // it is filled before every frame
            camera,
            yaw: 0.0,
            pitch: 0.0,
        };
    }

    pub fn reset_y_and_angle(&mut self, projection: &Mat4, resolution_w: u32, resolution_h: u32) {
        self.pitch = CameraComponent::PITCH;
        self.yaw = CameraComponent::YAW;
        self.camera.set_y(40.0);
        self.camera.rotate(self.pitch, self.yaw);
        self.camera
            .update_visible_z_range(projection, resolution_w, resolution_h);
    }
}

pub enum CameraMode {
    Free,
    FreeMoveButFixedAngle,
    FollowChar,
}

#[derive(Debug)]
pub struct EntitiesBelowCursor {
    friendly: Vec<CharEntityId>,
    enemy: Vec<CharEntityId>,
}

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

// Singleton Component
#[derive(Component)]
pub struct HumanInputComponent {
    pub is_console_open: bool,
    pub username: String,
    pub inputs: Vec<sdl2::event::Event>,
    skills_for_keys: [Option<Skills>; SKILLKEY_COUNT],
    pub key_bindings: Vec<([Option<Scancode>; 4], String)>,
    pub cast_mode: CastMode,
    keys: [KeyState; 284],
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
    pub alt_down: bool,
    pub ctrl_down: bool,
    pub shift_down: bool,
    pub last_mouse_x: u16,
    pub last_mouse_y: u16,
    pub delta_mouse_x: i32,
    pub delta_mouse_y: i32,
    pub mouse_world_pos: Vec2,
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
            keys_released_in_prev_frame: vec![],
            keys_pressed_in_prev_frame: vec![],
            left_mouse_down: false,
            right_mouse_down: false,
            left_mouse_released: false,
            left_mouse_pressed: false,
            right_mouse_pressed: false,
            right_mouse_released: false,
            alt_down: false,
            ctrl_down: false,
            shift_down: false,
            last_mouse_x: 400,
            last_mouse_y: 300,
            mouse_world_pos: v2(0.0, 0.0),
            mouse_wheel: 0,
            delta_mouse_x: 0,
            delta_mouse_y: 0,
            text: String::new(),
            keys: [KeyState::default(); 284],
            key_bindings: Vec::with_capacity(64),
        }
    }

    pub fn get_skill_for_key(&self, skill_key: SkillKey) -> Option<Skills> {
        self.skills_for_keys[skill_key as usize]
    }

    pub fn assign_skill(&mut self, skill_key: SkillKey, skill: Skills) {
        self.skills_for_keys[skill_key as usize] = Some(skill);
    }

    pub fn mouse_pos(&self) -> Vec2u {
        Vec2u::new(self.last_mouse_x, self.last_mouse_y)
    }

    pub fn cleanup_released_keys(&mut self) {
        for key in self.keys_released_in_prev_frame.drain(..) {
            self.keys[key as usize].just_released = false;
        }
        for key in self.keys_pressed_in_prev_frame.drain(..) {
            self.keys[key as usize].just_pressed = false;
        }
    }

    pub fn key_pressed(&mut self, key: Scancode) {
        if self.keys[key as usize].pressed() {
            self.keys_pressed_in_prev_frame.push(key);
        }
    }

    pub fn key_released(&mut self, key: Scancode) {
        if self.keys[key as usize].released() {
            self.keys_released_in_prev_frame.push(key);
        }
    }

    pub fn is_key_down(&self, key: Scancode) -> bool {
        self.keys[key as usize].down
    }

    #[allow(dead_code)]
    pub fn is_key_up(&self, key: Scancode) -> bool {
        !self.keys[key as usize].down
    }

    pub fn is_key_just_released(&self, key: Scancode) -> bool {
        self.keys[key as usize].just_released
    }

    pub fn is_key_just_pressed(&self, key: Scancode) -> bool {
        self.keys[key as usize].just_pressed
    }
}
