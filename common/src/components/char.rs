use crate::attack::{BasicAttackType, WeaponType};
use crate::char_attr::CharAttributes;
use crate::common::{float_cmp, v2, LocalTime, ServerTime, Vec2};
use crate::components::controller::PlayerIntention;
use crate::components::job_ids::JobSpriteId;
use crate::config::CommonConfigs;
use crate::packets::SocketBuffer;
use serde::export::fmt::{Debug, Display, Error};
use serde::export::Formatter;
use serde::{Deserialize, Serialize};
use specs::prelude::*;
use std::collections::HashMap;
use std::hash::Hash;
use strum_macros::Display;
use strum_macros::EnumCount;
use strum_macros::EnumIter;
use strum_macros::EnumString;

// TODO: now that I don'T have controller entity, this might be unnecessary
// any entity that moves and visible on the map
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LocalCharEntityId(u64);

impl TargetId for LocalCharEntityId {
    fn as_u64(&self) -> u64 {
        self.0
    }
}
impl TargetId for ServerEntityId {
    fn as_u64(&self) -> u64 {
        (self.0).0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ServerEntityId(LocalCharEntityId);

impl Display for LocalCharEntityId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

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

impl LocalCharEntityId {
    pub fn new(id: specs::Entity) -> LocalCharEntityId {
        LocalCharEntityId(unsafe { std::mem::transmute(id) })
    }
}

impl Into<specs::Entity> for LocalCharEntityId {
    fn into(self) -> specs::Entity {
        unsafe { std::mem::transmute(self.0) }
    }
}

impl From<specs::Entity> for LocalCharEntityId {
    fn from(entity: specs::Entity) -> Self {
        LocalCharEntityId::new(entity)
    }
}

#[derive(Eq, PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Sex {
    Male,
    Female,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CharState<T: TargetId> {
    Idle,
    Walking(Vec2),
    StandBy,
    Attacking {
        target: T,
        damage_occurs_at: LocalTime,
        basic_attack: BasicAttackType,
    },
    ReceivingDamage,
    Dead,
    //    CastingSkill(CastingSkillData),
}

impl<T: TargetId> Display for CharState<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CharState::Idle => write!(f, "Idle"),
            CharState::Walking(pos) => write!(f, "Walking({:.2}, {:.2})", pos.x, pos.y),
            CharState::Dead => write!(f, "Dead"),
            CharState::ReceivingDamage => write!(f, "ReceivingDamage"),
            CharState::StandBy => write!(f, "StandBy"),
            CharState::Attacking {
                target,
                damage_occurs_at,
                basic_attack,
            } => write!(f, "Attacking({})", target.as_u64()),
        }
    }
}

impl<T: TargetId> CharState<T> {
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
            CharState::Dead => false,
            _ => true,
        }
    }

    pub fn is_dead(&self) -> bool {
        match self {
            CharState::Dead => true,
            _ => false,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            CharState::Idle => "Idle",
            CharState::Walking(..) => "Walking",
            CharState::StandBy => "StandBy",
            CharState::Attacking { .. } => "Attacking",
            CharState::ReceivingDamage => "ReceivingDamage",
            CharState::Dead => "Dead",
        }
    }
}

unsafe impl<T: TargetId> Sync for CharState<T> {}

unsafe impl<T: TargetId> Send for CharState<T> {}

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

/// It determines the skills/roles of an entity
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

impl JobId {
    pub fn get_basic_attack_type(&self) -> BasicAttackType {
        match self {
            JobId::GUNSLINGER => BasicAttackType::Ranged {
                bullet_type: WeaponType::SilverBullet,
            },
            JobId::RangedMinion => BasicAttackType::Ranged {
                bullet_type: WeaponType::Arrow,
            },
            JobId::RANGER => BasicAttackType::Ranged {
                bullet_type: WeaponType::Arrow,
            },
            JobId::Turret => BasicAttackType::Ranged {
                bullet_type: WeaponType::SilverBullet,
            },
            _ => BasicAttackType::MeleeSimple,
        }
    }
}

/// It determines the behaviour of some skill etc, e.g. if skills cannot be casted on Guards
#[derive(Eq, PartialEq, Debug, Serialize, Deserialize, Clone, Copy)]
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
    Human {
        job_sprite_id: JobSpriteId,
        head_index: usize,
        sex: Sex,
    },
}

pub trait TargetId: Clone + Copy + Debug + PartialEq + Eq + Hash + Serialize {
    fn as_u64(&self) -> u64;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntityTarget<T: TargetId> {
    OtherEntity(T),
    Pos(Vec2),
    // TODO: is not it pos OR target?
    PosWhileAttacking(Vec2, Option<T>),
}

impl Display for EntityTarget<LocalCharEntityId> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            EntityTarget::Pos(pos) => write!(f, "Walking({:.2}, {:.2})", pos.x, pos.y),
            EntityTarget::OtherEntity(id) => write!(f, "OtherEntity({})", id),
            EntityTarget::PosWhileAttacking(pos, target) => write!(
                f,
                "PosWhileAttacking(({:.2}, {:.2})|{})",
                pos.x,
                pos.y,
                target.map(|it| it.to_string()).unwrap_or("None".to_owned())
            ),
        }
    }
}

#[derive(Component, Clone, Debug, Serialize, Deserialize)]
pub struct StaticCharDataComponent {
    pub name: String,
    pub team: Team,
    pub basic_attack_type: BasicAttackType,
    pub typ: CharType,
    pub outlook: CharOutlook,
    pub job_id: JobId,
}

impl StaticCharDataComponent {
    pub fn new(
        name: String,
        team: Team,
        typ: CharType,
        job_id: JobId,
        outlook: CharOutlook,
    ) -> StaticCharDataComponent {
        StaticCharDataComponent {
            name,
            team,
            basic_attack_type: job_id.get_basic_attack_type(),
            typ,
            outlook,
            job_id,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerCharState {
    pub pos: Vec2,
    pub dir: CharDir,
    pub state: CharState<ServerEntityId>,
    pub target: Option<EntityTarget<ServerEntityId>>,
    pub calculated_attribs: CharAttributes,
    pub attack_delay_ends_at: ServerTime,
    // TODO [SkillKey::Count]
    pub skill_cast_allowed_at: [ServerTime; 6],
    pub cannot_control_until: ServerTime,
    pub hp: i32,
}

#[derive(Component, Clone, Debug, Serialize, Deserialize)]
pub struct LocalCharStateComp {
    pos: Vec2,
    dir: CharDir,
    state: CharState<LocalCharEntityId>,
    pub target: Option<EntityTarget<LocalCharEntityId>>,
    calculated_attribs: CharAttributes,
    pub attack_delay_ends_at: LocalTime,
    // TODO [SkillKey::Count]
    pub skill_cast_allowed_at: [LocalTime; 6],
    pub cannot_control_until: LocalTime,
    pub hp: i32,
}

impl PartialEq for LocalCharStateComp {
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

impl Default for LocalCharStateComp {
    fn default() -> Self {
        LocalCharStateComp {
            pos: v2(0.0, 0.0),
            dir: CharDir::South,
            state: CharState::Idle,
            target: None,
            calculated_attribs: Default::default(),
            attack_delay_ends_at: LocalTime::from(0.0),
            skill_cast_allowed_at: [LocalTime::from(0.0); 6],
            cannot_control_until: LocalTime::from(0.0),
            hp: 0,
        }
    }
}

impl LocalCharStateComp {
    pub fn new(start_pos: Vec2, base_attributes: CharAttributes) -> LocalCharStateComp {
        LocalCharStateComp {
            pos: start_pos,
            state: CharState::Idle,
            target: None,
            dir: CharDir::South,
            hp: base_attributes.max_hp,
            calculated_attribs: base_attributes,
            attack_delay_ends_at: LocalTime::from(0.0),
            skill_cast_allowed_at: [LocalTime::from(0.0); 6],
            cannot_control_until: LocalTime::from(0.0),
        }
    }

    // it is here so that the client module does not have to have access to all the fields
    pub fn server_to_local(
        server_char_state: ServerCharState,
        now: LocalTime,
        server_to_local_time_diff: i64,
        map: &HashMap<ServerEntityId, LocalCharEntityId>,
    ) -> LocalCharStateComp {
        LocalCharStateComp {
            pos: server_char_state.pos,
            dir: server_char_state.dir,
            state: match server_char_state.state {
                CharState::Idle => CharState::Idle,
                CharState::Walking(pos) => CharState::Walking(pos),
                CharState::StandBy => CharState::StandBy,
                CharState::Attacking {
                    target,
                    damage_occurs_at,
                    basic_attack,
                } => CharState::Attacking {
                    target: map[&target],
                    damage_occurs_at,
                    basic_attack,
                },
                CharState::ReceivingDamage => CharState::ReceivingDamage,
                CharState::Dead => CharState::Dead,
            },
            target: match server_char_state.target {
                None => None,
                Some(EntityTarget::Pos(v)) => Some(EntityTarget::Pos(v)),
                Some(EntityTarget::PosWhileAttacking(v, maybe_target_id)) => Some(
                    EntityTarget::PosWhileAttacking(v, maybe_target_id.map(|it| map[&it])),
                ),
                Some(EntityTarget::OtherEntity(target_id)) => {
                    Some(EntityTarget::OtherEntity(map[&target_id]))
                }
            },
            calculated_attribs: server_char_state.calculated_attribs,
            attack_delay_ends_at: server_char_state
                .attack_delay_ends_at
                .to_local_time(now, server_to_local_time_diff),
            skill_cast_allowed_at: [
                server_char_state.skill_cast_allowed_at[0]
                    .to_local_time(now, server_to_local_time_diff),
                server_char_state.skill_cast_allowed_at[1]
                    .to_local_time(now, server_to_local_time_diff),
                server_char_state.skill_cast_allowed_at[2]
                    .to_local_time(now, server_to_local_time_diff),
                server_char_state.skill_cast_allowed_at[3]
                    .to_local_time(now, server_to_local_time_diff),
                server_char_state.skill_cast_allowed_at[4]
                    .to_local_time(now, server_to_local_time_diff),
                server_char_state.skill_cast_allowed_at[5]
                    .to_local_time(now, server_to_local_time_diff),
            ],
            cannot_control_until: server_char_state
                .cannot_control_until
                .to_local_time(now, server_to_local_time_diff),
            hp: server_char_state.hp,
        }
    }

    pub fn can_cast(&self, sys_time: LocalTime) -> bool {
        let can_cast_by_state = match &self.state {
            // TODO2
            //        CharState::CastingSkill(_) => false,
            CharState::Idle => true,
            CharState::Walking(_pos) => true,
            CharState::StandBy => true,
            CharState::Attacking { .. } => false,
            CharState::ReceivingDamage => false,
            CharState::Dead => false,
        };
        can_cast_by_state && self.cannot_control_until.has_already_passed(sys_time)
        // TODO2
        //        && char_state.statuses.can_cast()
    }

    pub fn can_move(&self, sys_time: LocalTime) -> bool {
        let can_move_by_state = match &self.state {
            // TODO2
            //        CharState::CastingSkill(casting_info) => casting_info.can_move,
            CharState::Idle => true,
            CharState::Walking(_pos) => true,
            CharState::StandBy => true,
            CharState::Attacking { .. } => false,
            CharState::ReceivingDamage => true,
            CharState::Dead => false,
        };
        can_move_by_state && self.cannot_control_until.has_already_passed(sys_time)
        // TODO2
        //        && char_state.statuses.can_move()
    }

    pub fn recalc_attribs_based_on_statuses(&mut self, job_id: JobId, dev_configs: &CommonConfigs) {
        // TODO2
        let base_attributes = CharAttributes::get_base_attributes(job_id, dev_configs);
        //        let modifier_collector = self.statuses.calc_attributes();
        //        self.calculated_attribs = base_attributes.apply(modifier_collector);
        self.calculated_attribs = base_attributes.clone();
        //
        //        self.attrib_bonuses = self
        //            .calculated_attribs
        //            .differences(&base_attributes, modifier_collector);
    }

    pub fn calculated_attribs(&self) -> &CharAttributes {
        &self.calculated_attribs
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

    pub fn set_state(&mut self, state: CharState<LocalCharEntityId>, dir: CharDir) {
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

    pub fn state(&self) -> &CharState<LocalCharEntityId> {
        &self.state
    }

    pub fn set_receiving_damage(&mut self) {
        match &self.state {
            // TODO2
            //            | CharState::CastingSkill(_)
            CharState::Idle
            | CharState::Walking(_)
            | CharState::StandBy
            | CharState::ReceivingDamage => {
                self.state = CharState::ReceivingDamage;
            }
            CharState::Attacking { .. } | CharState::Dead => {
                // denied
            }
        };
    }
}

/// The values that should be added to the sprite direction based on the camera
/// direction (the index is the camera direction, which is floor(angle/45)
pub const DIRECTION_TABLE: [usize; 8] = [6, 5, 4, 3, 2, 1, 0, 7];
#[repr(u8)]
#[derive(Clone, Copy, Debug, Display, PartialEq, Eq, Serialize, Deserialize)]
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

    pub fn get_opponent_team(&self) -> Team {
        match self {
            Team::Left => Team::Right,
            Team::Right => Team::Left,
            Team::Neutral => Team::Right,
            Team::EnemyForAll => Team::Right,
            Team::AllyForAll => Team::Right,
        }
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

    pub fn to_str(&self) -> &'static str {
        match self {
            Team::Right => "Right",
            Team::Left => "Left",
            Team::Neutral => "Neutral",
            Team::EnemyForAll => "EnemyForAll",
            Team::AllyForAll => "AllyForAll",
        }
    }
}

pub fn create_common_player_entity(
    name: String,
    world: &mut specs::World,
    typ: CharType,
    job_id: JobId,
    pos: Vec2,
    team: Team,
    outlook: CharOutlook,
) -> EntityBuilder {
    let base_attributes =
        CharAttributes::get_base_attributes(job_id, &world.read_resource::<CommonConfigs>())
            .clone();
    return world
        .create_entity()
        .with(LocalCharStateComp::new(pos, base_attributes))
        .with(StaticCharDataComponent::new(
            name, team, typ, job_id, outlook,
        ));
}
