use crate::common::{float_cmp, v2, Vec2};
use crate::components::job_ids::JobSpriteId;
use crate::components::snapshot::CharSnapshot;
use crate::packets::SocketBuffer;
use serde::{Deserialize, Serialize};
use specs::prelude::*;
use strum_macros::Display;
use strum_macros::EnumCount;
use strum_macros::EnumIter;
use strum_macros::EnumString;

// TODO: now that I don1T have controller entity, this might be unnecessary
// any entity that moves and visible on the map
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CharEntityId(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ServerEntityId(CharEntityId);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ControllerEntityId(Entity);

impl Into<specs::Entity> for ControllerEntityId {
    fn into(self) -> specs::Entity {
        self.0
    }
}

impl From<specs::Entity> for ControllerEntityId {
    fn from(entity: specs::Entity) -> Self {
        ControllerEntityId(entity)
    }
}

impl ControllerEntityId {
    pub fn new(id: specs::Entity) -> ControllerEntityId {
        ControllerEntityId(id)
    }
}

impl CharEntityId {
    pub fn new(id: specs::Entity) -> CharEntityId {
        CharEntityId(unsafe { std::mem::transmute(id) })
    }
}

impl Into<specs::Entity> for CharEntityId {
    fn into(self) -> specs::Entity {
        unsafe { std::mem::transmute(self.0) }
    }
}

impl From<specs::Entity> for CharEntityId {
    fn from(entity: specs::Entity) -> Self {
        CharEntityId::new(entity)
    }
}

#[derive(Eq, PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Sex {
    Male,
    Female,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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

impl CharState {
    pub fn discriminant_eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }

    pub fn is_walking(&self) -> bool {
        match self {
            CharState::Walking(_pos) => true,
            _ => false,
        }
    }

    pub fn is_alive(&self) -> bool {
        match self {
            // TODO2
            //            CharState::Dead => false,
            _ => true,
        }
    }

    pub fn is_dead(&self) -> bool {
        match self {
            // TODO2
            //            CharState::Dead => true,
            _ => false,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            CharState::Idle => "Idle",
            CharState::Walking(..) => "Walking",
        }
    }
}

unsafe impl Sync for CharState {}

unsafe impl Send for CharState {}

// Sprites are loaded based on the enum names, so non-camelcase names must be allowed
#[allow(non_camel_case_types)]
#[derive(
    EnumIter, EnumString, Display, Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize,
)]
pub enum MonsterId {
    Baphomet,
    Poring,
    Barricade,
    GEFFEN_MAGE_6,
    GEFFEN_MAGE_12, // red
    GEFFEN_MAGE_9,  // blue
    Dimik,
}

#[derive(
    EnumIter, EnumString, Display, Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize,
)]
pub enum JobId {
    CRUSADER,
    SWORDMAN,
    ARCHER,
    RANGER,
    ASSASSIN,
    ROGUE,
    KNIGHT,
    WIZARD,
    SAGE,
    ALCHEMIST,
    BLACKSMITH,
    PRIEST,
    MONK,
    GUNSLINGER,

    TargetDummy,
    HealingDummy,
    MeleeMinion,
    Barricade,
    RangedMinion,
    Turret,
    Guard,
}

#[derive(Eq, PartialEq, Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub enum CharType {
    Player,
    Minion,
    Mercenary,
    Boss,
    Guard,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(variant_size_differences)]
pub enum CharOutlook {
    Monster(MonsterId),
    // TODO: this variant can be smaller, e.g sex 1 bit, head_index ~8 bit etc
    Player {
        job_sprite_id: JobSpriteId,
        head_index: usize,
        sex: Sex,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntityTarget {
    OtherEntity(CharEntityId),
    Pos(Vec2),
    PosWhileAttacking(Vec2, Option<CharEntityId>),
}

#[derive(Component, Clone, Debug, Serialize, Deserialize)]
pub struct AuthorizedCharStateComponent {
    pos: Vec2,
    dir: CharDir,
    state: CharState,
    pub target: Option<EntityTarget>,
}

impl PartialEq for AuthorizedCharStateComponent {
    fn eq(&self, other: &Self) -> bool {
        let mut result =
            float_cmp(self.pos().x, other.pos().x) && float_cmp(self.pos().y, other.pos().y);
        result &= self.dir == other.dir;
        // TODO: think about it
        result &= self.state.discriminant_eq(other.state());
        result &= match &self.target {
            None => other.target.is_none(),
            Some(s) => match &other.target {
                Some(o) => std::mem::discriminant(s) == std::mem::discriminant(o),
                None => false,
            },
        };

        result
    }
}

impl Default for AuthorizedCharStateComponent {
    fn default() -> Self {
        AuthorizedCharStateComponent {
            pos: v2(0.0, 0.0),
            dir: CharDir::South,
            state: CharState::Idle,
            target: None,
        }
    }
}

impl AuthorizedCharStateComponent {
    pub fn new(start_pos: Vec2) -> AuthorizedCharStateComponent {
        AuthorizedCharStateComponent {
            pos: start_pos,
            state: CharState::Idle,
            target: None,
            dir: CharDir::South,
        }
    }

    pub fn overwrite(&mut self, other: &AuthorizedCharStateComponent) {
        *self = other.clone();
    }

    pub fn pos(&self) -> Vec2 {
        self.pos
    }

    pub fn set_pos(&mut self, new_pos: Vec2) {
        self.pos = new_pos;
    }

    pub fn set_dir(&mut self, new_dir: CharDir) {
        self.dir = new_dir;
    }

    pub fn add_pos(&mut self, new_pos: Vec2) {
        self.pos += new_pos;
    }

    pub fn dir(&self) -> CharDir {
        self.dir
    }

    pub fn set_state(&mut self, state: CharState, dir: CharDir) {
        //        match self.state {
        //            CharState::Walking(..) => match state {
        //                CharState::Idle => panic!("kurva anyád"),
        //                _ => {}
        //            },
        //            _ => {}
        //        }
        self.state = state;
        self.dir = dir;
    }

    pub fn state(&self) -> &CharState {
        &self.state
    }

    pub fn set_receiving_damage(&mut self) {
        match &self.state {
            CharState::Idle | CharState::Walking(_) => {
                // TODO2
                //            | CharState::StandBy
                //            | CharState::ReceivingDamage
                //            | CharState::CastingSkill(_) => {
                //                self.state = CharState::ReceivingDamage;
            } //            CharState::Attacking { .. } | CharState::Dead => {
              // denied
              //            }
        };
    }
}

/// The values that should be added to the sprite direction based on the camera
/// direction (the index is the camera direction, which is floor(angle/45)
pub const DIRECTION_TABLE: [usize; 8] = [6, 5, 4, 3, 2, 1, 0, 7];
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Clone, Copy)]
pub enum CollisionGroup {
    StaticModel,
    LeftPlayer,
    RightPlayer,
    LeftBarricade,
    RightBarricade,
    NeutralPlayerPlayer,
    NonCollidablePlayer,
    Minion,
    Turret,
    Guard,
    SkillArea,
}

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub enum StatusNature {
    Supportive,
    Harmful,
}

#[derive(Eq, Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
#[allow(dead_code)]
pub enum Team {
    Left,  // red
    Right, // blue
    Neutral,
    EnemyForAll,
    AllyForAll,
}

impl Team {
    pub fn get_collision_group(&self) -> CollisionGroup {
        match self {
            Team::Left => CollisionGroup::LeftPlayer,
            Team::Right => CollisionGroup::RightPlayer,
            _ => CollisionGroup::NeutralPlayerPlayer,
        }
    }

    pub fn get_barricade_collision_group(&self) -> CollisionGroup {
        match self {
            Team::Left => CollisionGroup::LeftBarricade,
            Team::Right => CollisionGroup::RightBarricade,
            _ => panic!(),
        }
    }

    #[allow(dead_code)]
    pub fn get_enemy_collision_group(&self) -> CollisionGroup {
        match self {
            Team::Left => CollisionGroup::RightPlayer,
            Team::Right => CollisionGroup::LeftPlayer,
            _ => CollisionGroup::NeutralPlayerPlayer,
        }
    }

    pub fn is_compatible(&self, nature: StatusNature, other_team: Team) -> bool {
        match nature {
            StatusNature::Harmful => self.can_attack(other_team),
            StatusNature::Supportive => self.can_support(other_team),
        }
    }

    pub fn is_ally_to(&self, other_team: Team) -> bool {
        match self {
            Team::Left => match other_team {
                Team::Left => true,
                Team::Right => false,
                Team::Neutral => false,
                Team::EnemyForAll => false,
                Team::AllyForAll => true,
            },
            Team::Right => match other_team {
                Team::Left => false,
                Team::Right => true,
                Team::Neutral => false,
                Team::EnemyForAll => false,
                Team::AllyForAll => true,
            },
            Team::Neutral => false,
            Team::EnemyForAll => false,
            Team::AllyForAll => true,
        }
    }

    pub fn get_palette_index(&self, other_team: Team) -> usize {
        self.is_ally_to(other_team) as usize
    }

    pub fn is_enemy_to(&self, other_team: Team) -> bool {
        match self {
            Team::Left => match other_team {
                Team::Left => false,
                Team::Right => true,
                Team::Neutral => false,
                Team::EnemyForAll => true,
                Team::AllyForAll => false,
            },
            Team::Right => match other_team {
                Team::Left => true,
                Team::Right => false,
                Team::Neutral => false,
                Team::EnemyForAll => true,
                Team::AllyForAll => false,
            },
            Team::Neutral => false,
            Team::EnemyForAll => true,
            Team::AllyForAll => false,
        }
    }

    #[inline]
    pub fn can_attack(&self, other: Team) -> bool {
        !self.is_ally_to(other)
    }

    #[inline]
    pub fn can_support(&self, other: Team) -> bool {
        !self.is_enemy_to(other)
    }
}
