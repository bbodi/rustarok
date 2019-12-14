use crate::common::Vec2;
use specs::prelude::*;

// TODO: it should be independent from Serde, th server should map this ID to an Entity
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CharEntityId(specs::Entity);

impl CharEntityId {
    pub fn new(id: specs::Entity) -> CharEntityId {
        CharEntityId(id)
    }
}

impl Into<specs::Entity> for CharEntityId {
    fn into(self) -> specs::Entity {
        self.0
    }
}

impl From<specs::Entity> for CharEntityId {
    fn from(entity: specs::Entity) -> Self {
        CharEntityId(entity)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum CharState {
    Idle,
    Walking(Vec2),
    //    StandBy,
    //    Attacking {
    //        target: CharEntityId,
    //        damage_occurs_at: ElapsedTime,
    //        basic_attack: BasicAttackType,
    //    },
    //    ReceivingDamage,
    //    Dead,
    //    CastingSkill(CastingSkillData),
}

unsafe impl Sync for CharState {}

unsafe impl Send for CharState {}

#[derive(Debug, Clone)]
pub enum EntityTarget {
    OtherEntity(CharEntityId),
    Pos(Vec2),
    PosWhileAttacking(Vec2, Option<CharEntityId>),
}

#[derive(Component)]
pub struct AuthorizedCharStateComponent {
    pos: Vec2,
    state: CharState,
    pub target: Option<EntityTarget>,
}

/// The values that should be added to the sprite direction based on the camera
/// direction (the index is the camera direction, which is floor(angle/45)
pub const DIRECTION_TABLE: [usize; 8] = [6, 5, 4, 3, 2, 1, 0, 7];
#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum CharDir {
    South,
    SouthWest,
    West,
    NorthWest,
    North,
    NorthEast,
    East,
    SouthEast,
}

impl From<usize> for CharDir {
    fn from(dir: usize) -> Self {
        unsafe { std::mem::transmute(dir as u8) }
    }
}

impl CharDir {
    pub fn as_usize(&self) -> usize {
        (*self) as usize
    }

    pub fn determine_dir(&target_pos: &Vec2, pos: &Vec2) -> CharDir {
        let dir_vec = target_pos - pos;
        // "- 90.0"
        // The calculated yaw for the camera are 90 at [0;1] and 180 at [1;0] etc,
        // this calculation gives a different result which is shifted 90 degrees clockwise,
        // so it is 90 at [1;0].
        let dd = dir_vec.x.atan2(dir_vec.y).to_degrees() - 90.0;
        let dd = if dd < 0.0 {
            dd + 360.0
        } else if dd > 360.0 {
            dd - 360.0
        } else {
            dd
        };
        let dir_index = (dd / 45.0 + 0.5) as usize % 8;
        return unsafe { std::mem::transmute(DIRECTION_TABLE[dir_index] as u8) };
    }
}
