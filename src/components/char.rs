use crate::asset::SpriteResource;
use crate::components::controller::{
    CameraComponent, ControllerComponent, HumanInputComponent, SkillKey, WorldCoords,
};
use crate::components::skills::skill::Skills;
use crate::components::status::status::Statuses;
use crate::consts::{JobId, MonsterId};
use crate::systems::render::render_command::RenderCommandCollectorComponent;
use crate::systems::sound_sys::AudioCommandCollectorComponent;
use crate::systems::{Sex, Sprites, SystemVariables};
use crate::{CharActionIndex, CollisionGroup, ElapsedTime, MonsterActionIndex, PhysicEngine};
use nalgebra::{Point2, Vector2};
use ncollide2d::pipeline::CollisionGroups;
use ncollide2d::shape::ShapeHandle;
use nphysics2d::object::{
    BodyPartHandle, ColliderDesc, DefaultBodyHandle, DefaultColliderHandle, RigidBodyDesc,
};
use specs::prelude::*;
use specs::Entity;
use std::collections::HashMap;

pub fn create_human_player(
    ecs_world: &mut specs::world::World,
    pos2d: Point2<f32>,
    sex: Sex,
    job_id: JobId,
    head_index: usize,
    radius: i32,
    team: Team,
) -> Entity {
    let desktop_client_entity = create_char(
        ecs_world,
        pos2d,
        sex,
        job_id,
        head_index,
        radius,
        team,
        CharType::Player,
        CollisionGroup::Player,
        &[CollisionGroup::NonPlayer],
    );
    let mut human_player = HumanInputComponent::new();
    human_player.assign_skill(SkillKey::Q, Skills::FireWall);
    human_player.assign_skill(SkillKey::W, Skills::AbsorbShield);
    human_player.assign_skill(SkillKey::E, Skills::Heal);
    human_player.assign_skill(SkillKey::R, Skills::BrutalTestSkill);
    human_player.assign_skill(SkillKey::Y, Skills::Mounting);

    ecs_world
        .write_storage()
        .insert(
            desktop_client_entity,
            RenderCommandCollectorComponent::new(),
        )
        .unwrap();
    ecs_world
        .write_storage()
        .insert(desktop_client_entity, AudioCommandCollectorComponent::new())
        .unwrap();
    ecs_world
        .write_storage()
        .insert(desktop_client_entity, human_player)
        .unwrap();
    // camera
    {
        let mut camera_component = CameraComponent::new();
        camera_component.reset_y_and_angle(
            &ecs_world
                .read_resource::<SystemVariables>()
                .matrices
                .projection,
        );
        ecs_world
            .write_storage()
            .insert(desktop_client_entity, camera_component)
            .unwrap();
    }
    return desktop_client_entity;
}

pub fn create_char(
    ecs_world: &mut specs::world::World,
    pos2d: Point2<f32>,
    sex: Sex,
    job_id: JobId,
    head_index: usize,
    radius: i32,
    team: Team,
    typ: CharType,
    collision_group: CollisionGroup,
    blacklist_coll_groups: &[CollisionGroup],
) -> Entity {
    let entity_id = {
        let char_comp = CharacterStateComponent::new(
            typ,
            CharOutlook::Player {
                job_id,
                head_index,
                sex,
            },
            team,
        );
        let mut entity_builder = ecs_world.create_entity().with(char_comp);

        entity_builder = entity_builder.with(SpriteRenderDescriptorComponent {
            action_index: CharActionIndex::Idle as usize,
            animation_started: ElapsedTime(0.0),
            animation_ends_at: ElapsedTime(0.0),
            forced_duration: None,
            direction: 0,
            fps_multiplier: 1.0,
        });
        entity_builder.build()
    };
    let physics_world = &mut ecs_world.write_resource::<PhysicEngine>();
    let physics_component = PhysicsComponent::new(
        physics_world,
        pos2d.coords,
        ComponentRadius(radius),
        entity_id,
        collision_group,
        blacklist_coll_groups,
    );
    ecs_world
        .write_storage()
        .insert(entity_id, physics_component)
        .unwrap();

    // controller
    ecs_world
        .write_storage()
        .insert(entity_id, ControllerComponent::new())
        .unwrap();
    return entity_id;
}

pub fn create_monster(
    ecs_world: &mut specs::world::World,
    pos2d: Point2<f32>,
    monster_id: MonsterId,
    radius: i32,
    team: Team,
    typ: CharType,
    collision_group: CollisionGroup,
    blacklist_coll_groups: &[CollisionGroup],
) -> Entity {
    let entity_id = {
        let mut entity_builder = ecs_world.create_entity().with(CharacterStateComponent::new(
            typ,
            CharOutlook::Monster(monster_id),
            team,
        ));
        entity_builder = entity_builder.with(SpriteRenderDescriptorComponent {
            action_index: CharActionIndex::Idle as usize,
            animation_started: ElapsedTime(0.0),
            animation_ends_at: ElapsedTime(0.0),
            forced_duration: None,
            direction: 0,
            fps_multiplier: 1.0,
        });
        entity_builder.build()
    };
    let mut storage = ecs_world.write_storage();
    let physics_world = &mut ecs_world.write_resource::<PhysicEngine>();
    let physics_component = PhysicsComponent::new(
        physics_world,
        pos2d.coords,
        ComponentRadius(radius),
        entity_id,
        collision_group,
        blacklist_coll_groups,
    );
    storage.insert(entity_id, physics_component).unwrap();
    return entity_id;
}

// radius = ComponentRadius * 0.5f32
#[derive(Eq, PartialEq, Hash)]
pub struct ComponentRadius(pub i32);

impl ComponentRadius {
    pub fn get(&self) -> f32 {
        self.0 as f32 * 0.5
    }
}

#[derive(Component)]
pub struct PhysicsComponent {
    pub radius: ComponentRadius,
    pub body_handle: DefaultBodyHandle,
    pub collider_handle: DefaultColliderHandle,
}

impl PhysicsComponent {
    pub fn new(
        world: &mut PhysicEngine,
        pos: Vector2<f32>,
        radius: ComponentRadius,
        entity_id: Entity,
        collision_group: CollisionGroup,
        blacklist_coll_groups: &[CollisionGroup],
    ) -> PhysicsComponent {
        let capsule = ShapeHandle::new(ncollide2d::shape::Ball::new(radius.get()));
        let body_handle = world.bodies.insert(
            RigidBodyDesc::new()
                .user_data(entity_id)
                .gravity_enabled(false)
                .linear_damping(50.0)
                .set_translation(pos)
                .build(),
        );
        let collider_handle = world.colliders.insert(
            ColliderDesc::new(capsule)
                .collision_groups(
                    CollisionGroups::new()
                        .with_membership(&[collision_group as usize])
                        .with_blacklist(
                            blacklist_coll_groups
                                .iter()
                                .map(|it| *it as usize)
                                .collect::<Vec<_>>()
                                .as_slice(),
                        ),
                )
                .density(radius.0 as f32 * 500.0)
                .user_data(entity_id)
                .build(BodyPartHandle(body_handle, 0)),
        );
        PhysicsComponent {
            radius,
            body_handle,
            collider_handle,
        }
    }
}

#[derive(Clone, Debug)]
pub struct CastingSkillData {
    pub target_area_pos: Option<Vector2<f32>>,
    pub char_to_skill_dir_when_casted: Vector2<f32>,
    pub target_entity: Option<Entity>,
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
        target: Entity,
        damage_occurs_at: ElapsedTime,
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
            (CharState::Attacking { .. }, false) => CharActionIndex::Attacking1 as usize,
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
    OtherEntity(Entity),
    Pos(WorldCoords),
}

const PERCENTAGE_FACTOR: i32 = 1000;
#[derive(Copy, Clone, Debug)]
pub struct Percentage {
    value: i32,
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
        let change =
            num * PERCENTAGE_FACTOR / 100 * self.value / PERCENTAGE_FACTOR / PERCENTAGE_FACTOR;
        return num + change;
    }

    pub fn subtract_me_from(&self, num: i32) -> i32 {
        let change =
            num * PERCENTAGE_FACTOR / 100 * self.value / PERCENTAGE_FACTOR / PERCENTAGE_FACTOR;
        return num - change;
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
        assert_eq!(Percentage(10).subtract_me_from(200), 180);
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
}

pub enum CharOutlook {
    Monster(MonsterId),
    Player {
        job_id: JobId,
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
                job_id,
                head_index: _,
                sex,
            } => {
                let sprites = &sprites.character_sprites;
                (
                    &sprites[&job_id][*sex as usize],
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

#[derive(Clone, Debug)]
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

    pub fn change_armor(
        &mut self,
        modifier: CharAttributeModifier,
        started: ElapsedTime,
        until: ElapsedTime,
    ) {
        if self.durations.armor_bonus_ends_at.is_later_than(until) {
            self.durations.armor_bonus_ends_at = until;
            self.durations.armor_bonus_started_at = started;
        }
        self.armor.push(modifier);
    }

    pub fn change_walking_speed(
        &mut self,
        modifier: CharAttributeModifier,
        started: ElapsedTime,
        until: ElapsedTime,
    ) {
        if self
            .durations
            .walking_speed_bonus_ends_at
            .is_later_than(until)
        {
            self.durations.walking_speed_bonus_ends_at = until;
            self.durations.walking_speed_bonus_started_at = started;
        }
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
}

impl Team {
    pub fn other(&self) -> Team {
        match self {
            Team::Left => Team::Right,
            Team::Right => Team::Left,
        }
    }
}

#[derive(Component)]
pub struct CharacterStateComponent {
    pos: WorldCoords,
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
    pub hp: i32,
    base_attributes: CharAttributes,
    calculated_attribs: CharAttributes,
    attrib_bonuses: CharAttributesBonuses,
    pub statuses: Statuses,
}

impl Drop for CharacterStateComponent {
    fn drop(&mut self) {
        log::info!("CharacterStateComponent DROPPED");
    }
}

impl CharacterStateComponent {
    pub fn new(typ: CharType, outlook: CharOutlook, team: Team) -> CharacterStateComponent {
        let statuses = Statuses::new();
        let base_attributes = Statuses::get_base_attributes(&typ);
        let calculated_attribs = base_attributes.clone();
        CharacterStateComponent {
            pos: v2!(0, 0),
            team,
            typ,
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

    pub fn update_attributes(&mut self) {
        let modifier_collector = self.statuses.calc_attributes();
        self.calculated_attribs = self.base_attributes.apply(modifier_collector);

        self.attrib_bonuses = self
            .calculated_attribs
            .differences(&self.base_attributes, modifier_collector);
    }

    pub fn update_statuses(
        &mut self,
        self_char_id: Entity,
        system_vars: &mut SystemVariables,
        entities: &specs::Entities,
        updater: &mut specs::Write<LazyUpdate>,
    ) {
        let changed =
            self.statuses
                .update(self_char_id, &self.pos(), system_vars, entities, updater);
        if changed {
            self.update_attributes();
            log::trace!(
                "Status expired. Attributes({:?}): mod: {:?}, attribs: {:?}",
                self_char_id,
                self.attrib_bonuses(),
                self.calculated_attribs()
            );
        }
    }

    pub fn set_pos_dont_use_it(&mut self, pos: WorldCoords) {
        self.pos = pos;
    }

    pub fn pos(&self) -> WorldCoords {
        self.pos
    }

    pub fn state_has_changed(&mut self) -> bool {
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
        can_move_by_state && self.cannot_control_until.is_earlier_than(sys_time)
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

#[derive(Clone, Copy)]
pub enum ActionPlayMode {
    Repeat,
    PlayThenHold,
    // FixFrame(12)
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
