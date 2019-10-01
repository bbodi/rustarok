use std::collections::HashMap;

use nalgebra::{Matrix4, Vector2};
use ncollide2d::pipeline::CollisionGroups;
use ncollide2d::shape::ShapeHandle;
use nphysics2d::object::{
    BodyPartHandle, BodyStatus, ColliderDesc, DefaultBodyHandle, DefaultColliderHandle,
    RigidBodyDesc,
};
use serde::Deserialize;
use specs::prelude::*;

use crate::asset::SpriteResource;
use crate::components::controller::{
    CameraComponent, CharEntityId, ControllerComponent, ControllerEntityId, HumanInputComponent,
    SkillKey, WorldCoord,
};
use crate::components::skills::basic_attack::{BasicAttack, WeaponType};
use crate::components::skills::skills::Skills;
use crate::components::status::status::{StatusNature, Statuses};
use crate::configs::DevConfig;
use crate::consts::{JobId, JobSpriteId, MonsterId};
use crate::runtime_assets::map::{CollisionGroup, PhysicEngine};
use crate::systems::render::render_command::RenderCommandCollector;
use crate::systems::sound_sys::AudioCommandCollectorComponent;
use crate::systems::{Sex, Sprites, SystemVariables};
use crate::ElapsedTime;

#[derive(Clone, Copy)]
pub enum CharActionIndex {
    Idle = 0,
    Walking = 8,
    Sitting = 16,
    PickingItem = 24,
    StandBy = 32,
    Attacking1 = 40,
    ReceivingDamage = 48,
    Freeze1 = 56,
    Dead = 65,
    Freeze2 = 72,
    Attacking2 = 80,
    Attacking3 = 88,
    CastingSpell = 96,
}

#[derive(Clone, Copy)]
pub enum MonsterActionIndex {
    Idle = 0,
    Walking = 8,
    Attack = 16,
    ReceivingDamage = 24,
    Die = 32,
}

pub fn attach_human_player_components(
    username: &str,
    char_entity_id: CharEntityId,
    controller_id: ControllerEntityId,
    updater: &LazyUpdate,
    physics_world: &mut PhysicEngine,
    projection_mat: Matrix4<f32>,
    pos2d: WorldCoord,
    sex: Sex,
    job_id: JobId,
    head_index: usize,
    team: Team,
    dev_configs: &DevConfig,
) {
    CharacterEntityBuilder::new(char_entity_id, username)
        .insert_sprite_render_descr_component(updater)
        .physics(pos2d, physics_world, |builder| {
            builder
                .collision_group(team.get_collision_group())
                .circle(1.0)
        })
        .char_state(updater, dev_configs, |ch| {
            ch.outlook_player(sex, JobSpriteId::from_job_id(job_id), head_index)
                .job_id(job_id)
                .team(team)
        });

    let mut human_player = HumanInputComponent::new(username);
    human_player.cast_mode = dev_configs.cast_mode;
    human_player.assign_skill(SkillKey::A, Skills::AttackMove);

    human_player.assign_skill(SkillKey::Q, Skills::FireWall);
    human_player.assign_skill(SkillKey::W, Skills::AbsorbShield);
    human_player.assign_skill(SkillKey::E, Skills::Heal);
    human_player.assign_skill(SkillKey::R, Skills::BrutalTestSkill);
    human_player.assign_skill(SkillKey::Y, Skills::Mounting);

    updater.insert(controller_id.0, RenderCommandCollector::new());
    updater.insert(controller_id.0, AudioCommandCollectorComponent::new());
    updater.insert(controller_id.0, human_player);
    updater.insert(controller_id.0, ControllerComponent::new(char_entity_id));
    // camera
    {
        let mut camera_component = CameraComponent::new(Some(controller_id));
        camera_component.reset_y_and_angle(&projection_mat);
        updater.insert(controller_id.0, camera_component);
    }
}

//pub fn assign_skills(human: &mut HumanInputComponent, job_id: JobId) {
//    match job_id {
//        JobId::CRUSADER => {
//
//        }
//    }
//}

pub struct CharPhysicsEntityBuilder<'a> {
    pos2d: WorldCoord,
    self_group: CollisionGroup,
    collider_shape: ShapeHandle<f32>,
    blacklist_groups: &'a [CollisionGroup],
    body_status: BodyStatus,
}

impl<'a> CharPhysicsEntityBuilder<'a> {
    pub fn new(pos2d: WorldCoord) -> CharPhysicsEntityBuilder<'a> {
        CharPhysicsEntityBuilder {
            pos2d,
            self_group: CollisionGroup::StaticModel,
            collider_shape: ShapeHandle::new(ncollide2d::shape::Ball::new(1.0)),
            blacklist_groups: &[],
            body_status: BodyStatus::Dynamic,
        }
    }

    pub fn collision_group(mut self, self_group: CollisionGroup) -> CharPhysicsEntityBuilder<'a> {
        self.self_group = self_group;
        self.blacklist_groups = match self_group {
            CollisionGroup::Guard => &[
                CollisionGroup::Minion,
                CollisionGroup::NonCollidablePlayer,
                CollisionGroup::StaticModel,
                CollisionGroup::LeftPlayer,
                CollisionGroup::RightPlayer,
                CollisionGroup::Guard,
                CollisionGroup::SkillArea,
                CollisionGroup::Turret,
                CollisionGroup::NeutralPlayerPlayer,
                CollisionGroup::LeftBarricade,
                CollisionGroup::RightBarricade,
            ],
            CollisionGroup::StaticModel => panic!(),
            CollisionGroup::LeftPlayer | CollisionGroup::RightPlayer => &[
                CollisionGroup::Minion,
                CollisionGroup::NonCollidablePlayer,
                CollisionGroup::Guard,
                CollisionGroup::Turret,
            ],
            CollisionGroup::NonCollidablePlayer => &[
                CollisionGroup::Minion,
                CollisionGroup::NonCollidablePlayer,
                CollisionGroup::StaticModel,
                CollisionGroup::LeftPlayer,
                CollisionGroup::RightPlayer,
                CollisionGroup::Guard,
                CollisionGroup::Turret,
                CollisionGroup::NeutralPlayerPlayer,
            ],
            CollisionGroup::Minion => &[
                CollisionGroup::LeftPlayer,
                CollisionGroup::RightPlayer,
                CollisionGroup::StaticModel,
                CollisionGroup::NonCollidablePlayer,
                CollisionGroup::Turret,
                CollisionGroup::NeutralPlayerPlayer,
            ],
            CollisionGroup::SkillArea => panic!(),
            CollisionGroup::Turret => &[
                CollisionGroup::Minion,
                CollisionGroup::NonCollidablePlayer,
                CollisionGroup::StaticModel,
                CollisionGroup::LeftPlayer,
                CollisionGroup::RightPlayer,
                CollisionGroup::Guard,
                CollisionGroup::Turret,
                CollisionGroup::NeutralPlayerPlayer,
            ],
            CollisionGroup::NeutralPlayerPlayer => &[
                CollisionGroup::Minion,
                CollisionGroup::NonCollidablePlayer,
                CollisionGroup::Guard,
                CollisionGroup::Turret,
            ],
            CollisionGroup::LeftBarricade => &[
                CollisionGroup::LeftPlayer,
                CollisionGroup::Minion,
                CollisionGroup::NonCollidablePlayer,
                CollisionGroup::Guard,
                CollisionGroup::Turret,
            ],
            CollisionGroup::RightBarricade => &[
                CollisionGroup::RightPlayer,
                CollisionGroup::Minion,
                CollisionGroup::NonCollidablePlayer,
                CollisionGroup::Guard,
                CollisionGroup::Turret,
            ],
        };
        self
    }

    pub fn body_status(mut self, body_status: BodyStatus) -> CharPhysicsEntityBuilder<'a> {
        self.body_status = body_status;
        self
    }

    pub fn circle(mut self, radius: f32) -> CharPhysicsEntityBuilder<'a> {
        self.collider_shape = ShapeHandle::new(ncollide2d::shape::Ball::new(radius));
        self
    }

    pub fn rectangle(mut self, w: f32, h: f32) -> CharPhysicsEntityBuilder<'a> {
        self.collider_shape = ShapeHandle::new(ncollide2d::shape::Cuboid::new(Vector2::new(
            w / 2.0,
            h / 2.0,
        )));
        self
    }
}

pub struct CharStateComponentBuilder {
    job_id: JobId,
    y: f32,
    outlook: CharOutlook,
    team: Team,
}

impl CharStateComponentBuilder {
    pub fn new() -> CharStateComponentBuilder {
        CharStateComponentBuilder {
            job_id: JobId::CRUSADER,
            y: 0.0,
            outlook: CharOutlook::Monster(MonsterId::Poring),
            team: Team::Left,
        }
    }

    pub fn job_id(mut self, job_id: JobId) -> CharStateComponentBuilder {
        self.job_id = job_id;
        self
    }

    pub fn y_coord(mut self, y: f32) -> CharStateComponentBuilder {
        self.y = y;
        self
    }

    pub fn outlook(mut self, outlook: CharOutlook) -> CharStateComponentBuilder {
        self.outlook = outlook;
        self
    }

    pub fn outlook_player(
        mut self,
        sex: Sex,
        job_sprite_id: JobSpriteId,
        head_index: usize,
    ) -> CharStateComponentBuilder {
        self.outlook = CharOutlook::Player {
            sex,
            job_sprite_id,
            head_index,
        };
        self
    }

    pub fn outlook_monster(mut self, monster_id: MonsterId) -> CharStateComponentBuilder {
        self.outlook = CharOutlook::Monster(monster_id);
        self
    }

    pub fn team(mut self, team: Team) -> CharStateComponentBuilder {
        self.team = team;
        self
    }
}

pub struct CharacterEntityBuilder {
    char_id: CharEntityId,
    name: String,
    pub physics_handles: Option<(DefaultColliderHandle, DefaultBodyHandle)>,
}

impl CharacterEntityBuilder {
    pub fn new(char_id: CharEntityId, name: &str) -> CharacterEntityBuilder {
        CharacterEntityBuilder {
            char_id,
            name: name.to_owned(),
            physics_handles: None,
        }
    }

    pub fn insert_npc_component(self, updater: &LazyUpdate) -> CharacterEntityBuilder {
        updater.insert(self.char_id.0, NpcComponent);
        self
    }

    pub fn insert_turret_component(
        self,
        owner_entity_id: CharEntityId,
        updater: &LazyUpdate,
    ) -> CharacterEntityBuilder {
        updater.insert(
            self.char_id.0,
            TurretComponent {
                owner_entity_id,
                preferred_target: None,
            },
        );
        self
    }

    pub fn insert_sprite_render_descr_component(
        self,
        updater: &LazyUpdate,
    ) -> CharacterEntityBuilder {
        updater.insert(self.char_id.0, SpriteRenderDescriptorComponent::new());
        self
    }

    pub fn char_state<F>(self, updater: &LazyUpdate, dev_configs: &DevConfig, char_builder_func: F)
    where
        F: Fn(CharStateComponentBuilder) -> CharStateComponentBuilder,
    {
        let char_builder = char_builder_func(CharStateComponentBuilder::new());
        updater.insert(
            self.char_id.0,
            CharacterStateComponent::new(
                self.name,
                char_builder.y,
                match char_builder.job_id {
                    JobId::Guard => CharType::Guard,
                    JobId::TargetDummy => CharType::Player,
                    JobId::HealingDummy => CharType::Player,
                    JobId::MeleeMinion => CharType::Minion,
                    JobId::RangedMinion => CharType::Minion,
                    JobId::Turret => CharType::Minion,
                    JobId::CRUSADER | JobId::SWORDMAN | JobId::ARCHER | JobId::HUNTER | JobId::ASSASSIN | JobId::ROGUE | JobId::KNIGHT | JobId::WIZARD | JobId::SAGE | JobId::ALCHEMIST | JobId::BLACKSMITH | JobId::PRIEST | JobId::MONK | JobId::GUNSLINGER =>
                        CharType::Player,
                    JobId::Barricade => CharType::Minion,
                },
                char_builder.outlook,
                char_builder.job_id,
                char_builder.team,
                dev_configs,
                self.physics_handles.expect("Initialize the physics component on this entity by calling 'physics()' on the builder!"),
            ),
        );
    }

    pub fn physics<F>(
        mut self,
        pos2d: WorldCoord,
        world: &mut PhysicEngine,
        phys_builder_fn: F,
    ) -> CharacterEntityBuilder
    where
        F: Fn(CharPhysicsEntityBuilder) -> CharPhysicsEntityBuilder,
    {
        let physics_builder = phys_builder_fn(CharPhysicsEntityBuilder::new(pos2d));
        let body_handle = world.bodies.insert(
            RigidBodyDesc::new()
                .user_data(self.char_id)
                .gravity_enabled(false)
                .status(physics_builder.body_status)
                .linear_damping(5.0)
                .set_translation(physics_builder.pos2d)
                .build(),
        );
        let collider_handle = world.colliders.insert(
            ColliderDesc::new(physics_builder.collider_shape)
                .collision_groups(
                    CollisionGroups::new()
                        .with_membership(&[physics_builder.self_group as usize])
                        .with_blacklist(
                            physics_builder
                                .blacklist_groups
                                .iter()
                                .map(|it| *it as usize)
                                .collect::<Vec<_>>()
                                .as_slice(),
                        ),
                )
                .density(500.0) // TODO
                .user_data(self.char_id)
                .build(BodyPartHandle(body_handle, 0)),
        );
        self.physics_handles = Some((collider_handle, body_handle));
        self
    }
}

// radius = ComponentRadius * 0.5f32
#[derive(Eq, PartialEq, Hash)]
pub struct ComponentRadius(pub i32);

impl ComponentRadius {
    pub fn get(&self) -> f32 {
        self.0 as f32 * 0.5
    }
}

#[derive(Clone, Debug)]
pub struct CastingSkillData {
    pub target_area_pos: Option<Vector2<f32>>,
    pub char_to_skill_dir_when_casted: Vector2<f32>,
    pub target_entity: Option<CharEntityId>,
    pub cast_started: ElapsedTime,
    pub cast_ends: ElapsedTime,
    pub can_move: bool,
    pub skill: Skills,
}

#[derive(Clone, Debug)]
pub enum CharState {
    Idle,
    Walking(Vector2<f32>),
    Sitting,
    PickingItem,
    StandBy,
    Attacking {
        target: CharEntityId,
        damage_occurs_at: ElapsedTime,
        basic_attack: BasicAttack,
    },
    ReceivingDamage,
    Freeze,
    Dead,
    CastingSkill(CastingSkillData),
}

unsafe impl Sync for CharState {}

unsafe impl Send for CharState {}

impl PartialEq for CharState {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

impl Eq for CharState {}

impl CharState {
    pub fn is_attacking(&self) -> bool {
        match self {
            CharState::Attacking { .. } => true,
            _ => false,
        }
    }

    pub fn is_casting(&self) -> bool {
        match self {
            CharState::CastingSkill { .. } => true,
            _ => false,
        }
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

    pub fn get_sprite_index(&self, is_monster: bool) -> usize {
        match (self, is_monster) {
            (CharState::Idle, false) => CharActionIndex::Idle as usize,
            (CharState::Walking(_pos), false) => CharActionIndex::Walking as usize,
            (CharState::Sitting, false) => CharActionIndex::Sitting as usize,
            (CharState::PickingItem, false) => CharActionIndex::PickingItem as usize,
            (CharState::StandBy, false) => CharActionIndex::StandBy as usize,
            (CharState::Attacking { .. }, false) => CharActionIndex::Attacking3 as usize,
            (CharState::ReceivingDamage, false) => CharActionIndex::ReceivingDamage as usize,
            (CharState::Freeze, false) => CharActionIndex::Freeze1 as usize,
            (CharState::Dead, false) => CharActionIndex::Dead as usize,
            (CharState::CastingSkill { .. }, false) => CharActionIndex::CastingSpell as usize,

            // monster
            (CharState::Idle, true) => MonsterActionIndex::Idle as usize,
            (CharState::Walking(_pos), true) => MonsterActionIndex::Walking as usize,
            (CharState::Sitting, true) => MonsterActionIndex::Idle as usize,
            (CharState::PickingItem, true) => MonsterActionIndex::Idle as usize,
            (CharState::StandBy, true) => MonsterActionIndex::Idle as usize,
            (CharState::Attacking { .. }, true) => MonsterActionIndex::Attack as usize,
            (CharState::ReceivingDamage, true) => MonsterActionIndex::ReceivingDamage as usize,
            (CharState::Freeze, true) => MonsterActionIndex::Idle as usize,
            (CharState::Dead, true) => MonsterActionIndex::Die as usize,
            (CharState::CastingSkill { .. }, true) => MonsterActionIndex::Attack as usize,
        }
    }
}

#[derive(Default, Debug)]
pub struct SpriteBoundingRect {
    pub bottom_left: [i32; 2],
    pub top_right: [i32; 2],
}

impl SpriteBoundingRect {
    pub fn merge(&mut self, other: &SpriteBoundingRect) {
        self.bottom_left[0] = self.bottom_left[0].min(other.bottom_left[0]);
        self.bottom_left[1] = self.bottom_left[1].max(other.bottom_left[1]);

        self.top_right[0] = self.top_right[0].max(other.top_right[0]);
        self.top_right[1] = self.top_right[1].min(other.top_right[1]);
    }
}

#[derive(Debug, Clone)]
pub enum EntityTarget {
    OtherEntity(CharEntityId),
    Pos(WorldCoord),
    PosWhileAttacking(WorldCoord, Option<CharEntityId>),
}

const PERCENTAGE_FACTOR: i32 = 1000;

#[derive(Copy, Clone, Debug, Deserialize)]
#[serde(from = "i32")]
pub struct Percentage {
    value: i32,
}

impl From<i32> for Percentage {
    fn from(value: i32) -> Self {
        Percentage(value)
    }
}

// able to represent numbers in 0.1% discrete steps
#[allow(non_snake_case)]
pub fn Percentage(value: i32) -> Percentage {
    Percentage {
        value: value * PERCENTAGE_FACTOR,
    }
}

impl Percentage {
    pub fn is_not_zero(&self) -> bool {
        self.value != 0
    }

    pub fn as_i16(&self) -> i16 {
        (self.value / PERCENTAGE_FACTOR) as i16
    }

    pub fn limit(&mut self, min: Percentage, max: Percentage) {
        self.value = self.value.min(max.value).max(min.value);
    }

    pub fn apply(&mut self, modifier: &CharAttributeModifier) {
        match modifier {
            CharAttributeModifier::AddPercentage(p) => {
                self.value += p.value;
            }
            CharAttributeModifier::AddValue(_v) => panic!(
                "{:?} += {:?}, you cannot add value to a percentage",
                self, modifier
            ),
            CharAttributeModifier::IncreaseByPercentage(p) => {
                self.value = self.increase_by(*p).value;
            }
        }
    }

    pub fn as_f32(&self) -> f32 {
        (self.value as f32 / PERCENTAGE_FACTOR as f32) / 100.0
    }

    pub fn increase_by(&self, p: Percentage) -> Percentage {
        let change = self.value / 100 * p.value;
        Percentage {
            value: self.value + change / PERCENTAGE_FACTOR,
        }
    }

    pub fn add_me_to(&self, num: i32) -> i32 {
        let f = PERCENTAGE_FACTOR as i64;
        let change = (num as i64) * f / 100 * (self.value as i64) / f / f;
        return num + (change as i32);
    }

    pub fn subtract_me_from(&self, num: i32) -> i32 {
        let f = PERCENTAGE_FACTOR as i64;
        let change = (num as i64) * f / 100 * (self.value as i64) / f / f;
        return num - (change as i32);
    }

    pub fn add(&mut self, p: Percentage) {
        self.value += p.value;
    }

    pub fn divp(&self, other: Percentage) -> Percentage {
        Percentage {
            value: self.value / other.value,
        }
    }

    pub fn div(&self, other: i32) -> Percentage {
        Percentage {
            value: self.value / other,
        }
    }

    pub fn subtract(&self, other: Percentage) -> Percentage {
        Percentage {
            value: self.value - other.value,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_percentages() {
        assert_eq!(Percentage(70).increase_by(Percentage(10)).as_i16(), 77);
        assert_eq!(Percentage(70).increase_by(Percentage(-10)).as_i16(), 63);
        assert_eq!(Percentage(100).increase_by(Percentage(200)).as_i16(), 300);
        assert_eq!(Percentage(10).add_me_to(200), 220);
        assert_eq!(Percentage(70).add_me_to(600), 1020);
        assert_eq!(Percentage(70).div(10).add_me_to(600), 642);
        assert_eq!(Percentage(-10).add_me_to(200), 180);
        assert_eq!(Percentage(50).add_me_to(76), 114);
        assert_eq!(Percentage(50).add_me_to(10_000), 15_000);
        assert_eq!(Percentage(10).subtract_me_from(200), 180);
        assert_eq!(Percentage(40).subtract_me_from(10_000), 6_000);
        assert_eq!(Percentage(70).subtract_me_from(600), 180);
        assert_eq!(Percentage(50).subtract_me_from(76), 38);
        assert_eq!(Percentage(100).as_f32(), 1.0);
        assert_eq!(Percentage(50).as_f32(), 0.5);
        assert_eq!(Percentage(5).as_f32(), 0.05);
        assert_eq!(Percentage(5).div(10).as_f32(), 0.005);
        assert_eq!(Percentage(-5).div(10).as_f32(), -0.005);
    }
}

#[derive(Eq, PartialEq)]
pub enum CharType {
    Player,
    Minion,
    Mercenary,
    Boss,
    Guard,
}

#[derive(Clone)]
pub enum CharOutlook {
    Monster(MonsterId),
    Player {
        job_sprite_id: JobSpriteId,
        head_index: usize,
        sex: Sex,
    },
}

impl CharOutlook {
    pub fn get_sprite_and_action_index<'a>(
        &self,
        sprites: &'a Sprites,
        char_state: &CharState,
    ) -> (&'a SpriteResource, usize) {
        return match self {
            CharOutlook::Player {
                job_sprite_id,
                head_index: _,
                sex,
            } => {
                let sprites = &sprites.character_sprites;
                (
                    &sprites[&job_sprite_id][*sex as usize],
                    char_state.get_sprite_index(false),
                )
            }
            CharOutlook::Monster(monster_id) => (
                &sprites.monster_sprites[&monster_id],
                char_state.get_sprite_index(true),
            ),
        };
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct CharAttributes {
    pub max_hp: i32,
    pub attack_damage: u16,
    pub walking_speed: Percentage,
    pub attack_range: Percentage,
    pub attack_speed: Percentage,
    pub armor: Percentage,
    pub healing: Percentage,
    pub hp_regen: Percentage,
    pub mana_regen: Percentage,
}

#[derive(Clone, Debug)]
pub struct CharAttributesBonuses {
    pub attrs: CharAttributes,
    pub durations: BonusDurations,
}

impl CharAttributes {
    pub fn zero() -> CharAttributes {
        CharAttributes {
            walking_speed: Percentage(0),
            attack_range: Percentage(0),
            attack_speed: Percentage(0),
            attack_damage: 0,
            armor: Percentage(0),
            healing: Percentage(0),
            hp_regen: Percentage(0),
            max_hp: 0,
            mana_regen: Percentage(0),
        }
    }

    pub fn differences(
        &self,
        other: &CharAttributes,
        collector: &CharAttributeModifierCollector,
    ) -> CharAttributesBonuses {
        return CharAttributesBonuses {
            attrs: CharAttributes {
                max_hp: self.max_hp - other.max_hp,
                attack_damage: self.attack_damage - other.attack_damage,
                walking_speed: self.walking_speed.subtract(other.walking_speed),
                attack_range: self.attack_range.subtract(other.attack_range),
                attack_speed: self.attack_speed.subtract(other.attack_speed),
                armor: (self.armor).subtract(other.armor),
                healing: self.healing.subtract(other.healing),
                hp_regen: self.hp_regen.subtract(other.hp_regen),
                mana_regen: self.mana_regen.subtract(other.mana_regen),
            },
            durations: collector.durations.clone(),
        };
    }

    pub fn apply(&self, modifiers: &CharAttributeModifierCollector) -> CharAttributes {
        let mut attr = self.clone();
        for m in &modifiers.max_hp {
            match m {
                CharAttributeModifier::AddPercentage(_p) => {
                    panic!("max_hp += {:?}, you cannot add percentage to a value", m)
                }
                CharAttributeModifier::AddValue(v) => {
                    attr.max_hp += *v as i32;
                }
                CharAttributeModifier::IncreaseByPercentage(p) => {
                    attr.max_hp = p.add_me_to(attr.max_hp);
                }
            }
        }
        for m in &modifiers.attack_damage {
            match m {
                CharAttributeModifier::AddPercentage(_p) => panic!(
                    "attack_damage += {:?}, you cannot add percentage to a value",
                    m
                ),
                CharAttributeModifier::AddValue(v) => {
                    attr.attack_damage += *v as u16;
                }
                CharAttributeModifier::IncreaseByPercentage(p) => {
                    attr.attack_damage = p.add_me_to(attr.attack_damage as i32) as u16;
                }
            }
        }

        for m in &modifiers.walking_speed {
            attr.walking_speed.apply(m);
        }
        for m in &modifiers.attack_range {
            attr.attack_range.apply(m);
        }
        for m in &modifiers.attack_speed {
            attr.attack_speed.apply(m);
        }
        attr.attack_speed.limit(Percentage(-300), Percentage(500));
        for m in &modifiers.armor {
            attr.armor.apply(m);
        }
        attr.armor.limit(Percentage(-100), Percentage(100));
        for m in &modifiers.healing {
            attr.healing.apply(m);
        }
        for m in &modifiers.hp_regen {
            attr.hp_regen.apply(m);
        }
        for m in &modifiers.mana_regen {
            attr.mana_regen.apply(m);
        }
        return attr;
    }
}

#[derive(Clone, Debug)]
pub enum CharAttributeModifier {
    AddPercentage(Percentage),
    AddValue(f32),
    IncreaseByPercentage(Percentage),
}

#[derive(Clone, Debug)]
pub struct BonusDurations {
    pub max_hp_bonus_ends_at: ElapsedTime,
    pub walking_speed_bonus_ends_at: ElapsedTime,
    pub attack_range_bonus_ends_at: ElapsedTime,
    pub attack_speed_bonus_ends_at: ElapsedTime,
    pub attack_damage_bonus_ends_at: ElapsedTime,
    pub armor_bonus_ends_at: ElapsedTime,
    pub healing_bonus_ends_at: ElapsedTime,
    pub hp_regen_bonus_ends_at: ElapsedTime,
    pub mana_regen_bonus_ends_at: ElapsedTime,

    pub max_hp_bonus_started_at: ElapsedTime,
    pub walking_speed_bonus_started_at: ElapsedTime,
    pub attack_range_bonus_started_at: ElapsedTime,
    pub attack_speed_bonus_started_at: ElapsedTime,
    pub attack_damage_bonus_started_at: ElapsedTime,
    pub armor_bonus_started_at: ElapsedTime,
    pub healing_bonus_started_at: ElapsedTime,
    pub hp_regen_bonus_started_at: ElapsedTime,
    pub mana_regen_bonus_started_at: ElapsedTime,
}

impl BonusDurations {
    pub fn with_invalid_times() -> BonusDurations {
        BonusDurations {
            max_hp_bonus_ends_at: ElapsedTime(std::f32::MAX),
            walking_speed_bonus_ends_at: ElapsedTime(std::f32::MAX),
            attack_range_bonus_ends_at: ElapsedTime(std::f32::MAX),
            attack_speed_bonus_ends_at: ElapsedTime(std::f32::MAX),
            attack_damage_bonus_ends_at: ElapsedTime(std::f32::MAX),
            armor_bonus_ends_at: ElapsedTime(std::f32::MAX),
            healing_bonus_ends_at: ElapsedTime(std::f32::MAX),
            hp_regen_bonus_ends_at: ElapsedTime(std::f32::MAX),
            mana_regen_bonus_ends_at: ElapsedTime(std::f32::MAX),

            max_hp_bonus_started_at: ElapsedTime(std::f32::MAX),
            walking_speed_bonus_started_at: ElapsedTime(std::f32::MAX),
            attack_range_bonus_started_at: ElapsedTime(std::f32::MAX),
            attack_speed_bonus_started_at: ElapsedTime(std::f32::MAX),
            attack_damage_bonus_started_at: ElapsedTime(std::f32::MAX),
            armor_bonus_started_at: ElapsedTime(std::f32::MAX),
            healing_bonus_started_at: ElapsedTime(std::f32::MAX),
            hp_regen_bonus_started_at: ElapsedTime(std::f32::MAX),
            mana_regen_bonus_started_at: ElapsedTime(std::f32::MAX),
        }
    }
}

#[derive(Clone, Debug)]
pub struct CharAttributeModifierCollector {
    max_hp: Vec<CharAttributeModifier>,
    walking_speed: Vec<CharAttributeModifier>,
    attack_range: Vec<CharAttributeModifier>,
    attack_speed: Vec<CharAttributeModifier>,
    attack_damage: Vec<CharAttributeModifier>,
    armor: Vec<CharAttributeModifier>,
    healing: Vec<CharAttributeModifier>,
    hp_regen: Vec<CharAttributeModifier>,
    mana_regen: Vec<CharAttributeModifier>,
    durations: BonusDurations,
}

impl CharAttributeModifierCollector {
    pub fn new() -> CharAttributeModifierCollector {
        CharAttributeModifierCollector {
            max_hp: Vec::with_capacity(8),
            walking_speed: Vec::with_capacity(8),
            attack_range: Vec::with_capacity(8),
            attack_speed: Vec::with_capacity(8),
            attack_damage: Vec::with_capacity(8),
            armor: Vec::with_capacity(8),
            healing: Vec::with_capacity(8),
            hp_regen: Vec::with_capacity(8),
            mana_regen: Vec::with_capacity(8),
            durations: BonusDurations::with_invalid_times(),
        }
    }

    pub fn change_attack_damage(
        &mut self,
        modifier: CharAttributeModifier,
        started: ElapsedTime,
        until: ElapsedTime,
    ) {
        CharAttributeModifierCollector::set_durations(
            started,
            until,
            &mut self.durations.attack_damage_bonus_started_at,
            &mut self.durations.attack_damage_bonus_ends_at,
        );
        self.attack_damage.push(modifier);
    }

    pub fn change_attack_speed(
        &mut self,
        modifier: CharAttributeModifier,
        started: ElapsedTime,
        until: ElapsedTime,
    ) {
        CharAttributeModifierCollector::set_durations(
            started,
            until,
            &mut self.durations.attack_speed_bonus_started_at,
            &mut self.durations.attack_speed_bonus_ends_at,
        );
        self.attack_speed.push(modifier);
    }

    pub fn change_armor(
        &mut self,
        modifier: CharAttributeModifier,
        started: ElapsedTime,
        until: ElapsedTime,
    ) {
        CharAttributeModifierCollector::set_durations(
            started,
            until,
            &mut self.durations.armor_bonus_started_at,
            &mut self.durations.armor_bonus_ends_at,
        );
        self.armor.push(modifier);
    }

    fn set_durations(
        new_started_at: ElapsedTime,
        new_ends_at: ElapsedTime,
        current_started_at: &mut ElapsedTime,
        current_ends_at: &mut ElapsedTime,
    ) {
        if current_ends_at.has_not_passed_yet(new_ends_at) {
            *current_ends_at = new_ends_at;
            *current_started_at = new_started_at;
        }
    }

    pub fn change_attack_range(
        &mut self,
        modifier: CharAttributeModifier,
        started: ElapsedTime,
        until: ElapsedTime,
    ) {
        CharAttributeModifierCollector::set_durations(
            started,
            until,
            &mut self.durations.attack_range_bonus_started_at,
            &mut self.durations.attack_range_bonus_ends_at,
        );
        self.attack_range.push(modifier);
    }

    pub fn change_walking_speed(
        &mut self,
        modifier: CharAttributeModifier,
        started: ElapsedTime,
        until: ElapsedTime,
    ) {
        CharAttributeModifierCollector::set_durations(
            started,
            until,
            &mut self.durations.walking_speed_bonus_started_at,
            &mut self.durations.walking_speed_bonus_ends_at,
        );
        self.walking_speed.push(modifier);
    }

    pub fn clear(&mut self) {
        self.max_hp.clear();
        self.walking_speed.clear();
        self.attack_range.clear();
        self.attack_speed.clear();
        self.attack_damage.clear();
        self.armor.clear();
        self.healing.clear();
        self.hp_regen.clear();
        self.mana_regen.clear();
        self.durations = BonusDurations::with_invalid_times();
    }
}

#[derive(Eq, Debug, PartialEq, Clone, Copy)]
pub enum Team {
    Left,
    Right,
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
            StatusNature::Neutral => true,
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

pub struct TurretControllerComponent;

impl Component for TurretControllerComponent {
    type Storage = FlaggedStorage<Self, DenseVecStorage<Self>>;
}

#[derive(Component)]
pub struct TurretComponent {
    pub owner_entity_id: CharEntityId,
    pub preferred_target: Option<CharEntityId>,
}

pub struct NpcComponent;

impl Component for NpcComponent {
    type Storage = FlaggedStorage<Self, DenseVecStorage<Self>>;
}

//fn can_target_move(&self) -> bool {
//    false
//}
//
//fn can_target_be_controlled(&self) -> bool {
//    true
//}
//
//fn can_target_cast(&self) -> bool {
//    false
//}
//fn get_status_completion_percent(&self, now: ElapsedTime) -> Option<(ElapsedTime, f32)> {
//    Some((self.until, now.percentage_between(self.started, self.until)))
//}

#[derive(Component)]
pub struct CharacterStateComponent {
    pub name: String,
    // characters also has names so it is possible to follow them with a camera
    pub basic_attack: BasicAttack,
    pos: WorldCoord,
    y: f32,
    pub team: Team,
    pub target: Option<EntityTarget>,
    pub typ: CharType,
    state: CharState,
    prev_state: CharState,
    dir: usize,
    pub attack_delay_ends_at: ElapsedTime,
    pub skill_cast_allowed_at: HashMap<Skills, ElapsedTime>,
    pub cannot_control_until: ElapsedTime,
    pub outlook: CharOutlook,
    pub job_id: JobId,
    pub hp: i32,
    base_attributes: CharAttributes,
    calculated_attribs: CharAttributes,
    attrib_bonuses: CharAttributesBonuses,
    pub statuses: Statuses,
    pub body_handle: DefaultBodyHandle,
    pub collider_handle: DefaultColliderHandle,
}

impl Drop for CharacterStateComponent {
    fn drop(&mut self) {
        log::info!("CharacterStateComponent DROPPED");
    }
}

impl CharacterStateComponent {
    pub fn update_base_attributes(&mut self, dev_configs: &DevConfig) {
        self.base_attributes = Statuses::get_base_attributes(self.job_id, dev_configs);
        self.recalc_attribs_based_on_statuses()
    }

    pub fn set_noncollidable(&self, physics_world: &mut PhysicEngine) {
        if let Some(collider) = physics_world.colliders.get_mut(self.collider_handle) {
            let mut cg = collider.collision_groups().clone();
            cg.modify_membership(self.team.get_collision_group() as usize, false);
            cg.modify_membership(CollisionGroup::NonCollidablePlayer as usize, true);
            collider.set_collision_groups(cg);
        }
        if let Some(body) = physics_world.bodies.get_mut(self.body_handle) {
            body.set_status(BodyStatus::Kinematic);
        }
    }

    pub fn set_collidable(&self, physics_world: &mut PhysicEngine) {
        if let Some(collider) = physics_world.colliders.get_mut(self.collider_handle) {
            let mut cg = collider.collision_groups().clone();
            cg.modify_membership(self.team.get_collision_group() as usize, true);
            cg.modify_membership(CollisionGroup::NonCollidablePlayer as usize, false);
            collider.set_collision_groups(cg);
        }
        if let Some(body) = physics_world.bodies.get_mut(self.body_handle) {
            body.set_status(BodyStatus::Dynamic);
        }
    }

    pub fn new(
        name: String,
        y: f32,
        char_type: CharType,
        outlook: CharOutlook,
        job_id: JobId,
        team: Team,
        dev_configs: &DevConfig,
        physics_component: (DefaultColliderHandle, DefaultBodyHandle),
    ) -> CharacterStateComponent {
        let statuses = Statuses::new();
        let base_attributes = Statuses::get_base_attributes(job_id, dev_configs);
        let calculated_attribs = base_attributes.clone();
        CharacterStateComponent {
            basic_attack: match job_id {
                JobId::GUNSLINGER => BasicAttack::Ranged {
                    bullet_type: WeaponType::SilverBullet,
                },
                JobId::RangedMinion => BasicAttack::Ranged {
                    bullet_type: WeaponType::Arrow,
                },
                JobId::HUNTER => BasicAttack::Ranged {
                    bullet_type: WeaponType::Arrow,
                },
                JobId::Turret => BasicAttack::Ranged {
                    bullet_type: WeaponType::SilverBullet,
                },
                _ => BasicAttack::Melee,
            },
            job_id,
            name,
            pos: v2!(0, 0),
            y,
            team,
            typ: char_type,
            outlook,
            target: None,
            skill_cast_allowed_at: HashMap::new(),
            state: CharState::Idle,
            prev_state: CharState::Idle,
            dir: 0,
            cannot_control_until: ElapsedTime(0.0),
            attack_delay_ends_at: ElapsedTime(0.0),
            hp: calculated_attribs.max_hp,
            base_attributes,
            calculated_attribs,
            attrib_bonuses: CharAttributesBonuses {
                attrs: CharAttributes::zero(),
                durations: BonusDurations::with_invalid_times(),
            },
            statuses,
            body_handle: physics_component.1,
            collider_handle: physics_component.0,
        }
    }

    pub fn base_attributes(&self) -> &CharAttributes {
        &self.base_attributes
    }
    pub fn calculated_attribs(&self) -> &CharAttributes {
        &self.calculated_attribs
    }
    pub fn attrib_bonuses(&self) -> &CharAttributesBonuses {
        &self.attrib_bonuses
    }

    pub fn recalc_attribs_based_on_statuses(&mut self) {
        let modifier_collector = self.statuses.calc_attributes();
        self.calculated_attribs = self.base_attributes.apply(modifier_collector);

        self.attrib_bonuses = self
            .calculated_attribs
            .differences(&self.base_attributes, modifier_collector);
    }

    pub fn update_statuses(
        &mut self,
        self_char_id: CharEntityId,
        system_vars: &mut SystemVariables,
        entities: &specs::Entities,
        updater: &mut LazyUpdate,
        phyisics_world: &mut PhysicEngine,
    ) {
        // TODO: refactor this
        //                 a hack so statuses and self can be mut at the same time
        let mut mut_statuses = std::mem::replace(&mut self.statuses, Statuses::new());
        let bit_indices_of_changed_statuses = mut_statuses.update(
            self_char_id,
            self,
            phyisics_world,
            system_vars,
            entities,
            updater,
        );
        std::mem::replace(&mut self.statuses, mut_statuses);
        if bit_indices_of_changed_statuses > 0 {
            self.statuses
                .remove_statuses(bit_indices_of_changed_statuses);
            self.recalc_attribs_based_on_statuses();
            log::trace!(
                "Status expired. Attributes({:?}): mod: {:?}, attribs: {:?}",
                self_char_id,
                self.attrib_bonuses(),
                self.calculated_attribs()
            );
        }
    }

    pub fn set_pos_dont_use_it(&mut self, pos: WorldCoord) {
        self.pos = pos;
    }

    pub fn set_y(&mut self, y: f32) {
        self.y = y;
    }

    pub fn pos(&self) -> WorldCoord {
        self.pos
    }

    pub fn get_y(&self) -> f32 {
        self.y
    }

    pub fn state_has_changed(&self) -> bool {
        return self.prev_state != self.state;
    }

    pub fn save_prev_state(&mut self) {
        self.prev_state = self.state.clone();
    }

    pub fn can_move(&self, sys_time: ElapsedTime) -> bool {
        let can_move_by_state = match &self.state {
            CharState::CastingSkill(casting_info) => casting_info.can_move,
            CharState::Idle => true,
            CharState::Walking(_pos) => true,
            CharState::Sitting => true,
            CharState::PickingItem => false,
            CharState::StandBy => true,
            CharState::Attacking { .. } => false,
            CharState::ReceivingDamage => true,
            CharState::Freeze => false,
            CharState::Dead => false,
        };
        can_move_by_state
            && self.cannot_control_until.has_already_passed(sys_time)
            && self.statuses.can_move()
    }

    pub fn can_cast(&self, sys_time: ElapsedTime) -> bool {
        let can_cast_by_state = match &self.state {
            CharState::CastingSkill(_) => false,
            CharState::Idle => true,
            CharState::Walking(_pos) => true,
            CharState::Sitting => true,
            CharState::PickingItem => false,
            CharState::StandBy => true,
            CharState::Attacking { .. } => false,
            CharState::ReceivingDamage => false,
            CharState::Freeze => false,
            CharState::Dead => false,
        };
        can_cast_by_state
            && self.cannot_control_until.has_already_passed(sys_time)
            && self.statuses.can_cast()
    }

    pub fn state(&self) -> &CharState {
        &self.state
    }

    pub fn prev_state(&self) -> &CharState {
        &self.prev_state
    }

    pub fn went_from_casting_to_idle(&self) -> bool {
        match self.state {
            CharState::Idle => match self.prev_state {
                CharState::CastingSkill(_) => true,
                _ => false,
            },
            _ => false,
        }
    }

    pub fn dir(&self) -> usize {
        self.dir
    }

    pub fn set_state(&mut self, state: CharState, dir: usize) {
        self.state = state;
        self.dir = dir;
    }

    pub fn set_receiving_damage(&mut self) {
        match &self.state {
            CharState::Idle
            | CharState::Walking(_)
            | CharState::Sitting
            | CharState::PickingItem
            | CharState::StandBy
            | CharState::ReceivingDamage
            | CharState::Freeze
            | CharState::CastingSkill(_) => {
                self.state = CharState::ReceivingDamage;
            }
            CharState::Attacking { .. } | CharState::Dead => {
                // denied
            }
        };
    }

    pub fn set_dir(&mut self, dir: usize) {
        self.dir = dir;
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum ActionPlayMode {
    Repeat,
    PlayThenHold,
    Once,
    Reverse,
    FixFrame(usize),
}

#[derive(Component)]
pub struct SpriteRenderDescriptorComponent {
    pub action_index: usize,
    pub fps_multiplier: f32,
    pub animation_started: ElapsedTime,
    pub forced_duration: Option<ElapsedTime>,
    pub direction: usize,
    /// duration of the current animation
    pub animation_ends_at: ElapsedTime,
}

impl SpriteRenderDescriptorComponent {
    pub fn new() -> SpriteRenderDescriptorComponent {
        SpriteRenderDescriptorComponent {
            action_index: CharActionIndex::Idle as usize,
            animation_started: ElapsedTime(0.0),
            animation_ends_at: ElapsedTime(0.0),
            forced_duration: None,
            direction: 0,
            fps_multiplier: 1.0,
        }
    }
}
