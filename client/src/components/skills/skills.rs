use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use nalgebra::Vector2;
use nphysics2d::object::DefaultColliderHandle;
use serde::Deserialize;
use serde::Serialize;
use specs::prelude::*;
use strum_macros::EnumIter;

use crate::components::char::{ActionPlayMode, CastingSkillData, CharacterStateComponent, Team};
use crate::components::skills::absorb_shield::ABSORB_SHIELD_SKILL;
use crate::components::skills::brutal_test_skill::BRUTAL_TEST_SKILL;
use crate::components::skills::cure::CURE_SKILL;
use crate::components::skills::fire_bomb::FIRE_BOMB_SKILL;
use crate::components::skills::firewall::FIRE_WALL_SKILL;
use crate::components::skills::heal::HEAL_SKILL;
use crate::components::skills::lightning::LIGHTNING_SKILL;
use crate::components::skills::mounting::MOUNTING_SKILL;
use crate::components::skills::poison::POISON_SKILL;
use crate::components::skills::wiz_pyroblast::WIZ_PYRO_BLAST_SKILL;
use rustarok_common::common::{v2_to_v3, DeltaTime, Vec2};

use crate::audio::sound_sys::AudioCommandCollectorComponent;
use crate::components::skills::assa_blade_dash::ASSA_BLADE_DASH_SKILL;
use crate::components::skills::assa_phase_prism::ASSA_PHASE_PRISM_SKILL;
use crate::components::skills::falcon_attack::FALCON_ATTACK_SKILL;
use crate::components::skills::falcon_carry::FALCON_CARRY_SKILL;
use crate::components::skills::gaz_barricade::GAZ_BARRICADE_SKILL;
use crate::components::skills::gaz_exo_skel::EXO_SKELETON_SKILL;
use crate::components::skills::gaz_turret::{
    GAZ_DESTROY_TURRET_SKILL, GAZ_TURRET_SKILL, GAZ_TURRET_TARGET_SKILL,
};
use crate::components::skills::gaz_xplod_charge::GAZ_XPLODIUM_CHARGE_SKILL;
use crate::components::skills::sanctuary::SANCTUARY_SKILL;
use crate::components::status::status::{ApplyStatusComponent, ApplyStatusInAreaComponent};
use crate::components::{ApplyForceComponent, AreaAttackComponent, HpModificationRequest};
use crate::configs::DevConfig;
use crate::effect::StrEffectType;
use crate::render::render_command::RenderCommandCollector;
use crate::render::render_sys::RenderDesktopClientSystem;
use crate::systems::{AssetResources, CharEntityId, Collision, SystemVariables};
use crate::{ElapsedTime, PhysicEngine};

pub type WorldCollisions = HashMap<(DefaultColliderHandle, DefaultColliderHandle), Collision>;

pub struct SkillManifestationUpdateParam<'a, 'longer> {
    pub self_entity_id: Entity,
    pub all_collisions_in_world: &'longer WorldCollisions,
    sys_vars: &'longer mut SystemVariables,
    entities: &'a Entities<'a>,
    pub char_storage: &'longer mut WriteStorage<'a, CharacterStateComponent>,
    pub physics_world: &'longer mut PhysicEngine,
    updater: &'longer mut LazyUpdate,
}

impl<'a, 'longer> SkillManifestationUpdateParam<'a, 'longer> {
    pub fn new(
        self_entity_id: Entity,
        all_collisions_in_world: &'longer WorldCollisions,
        sys_vars: &'longer mut SystemVariables,
        entities: &'a Entities,
        char_storage: &'longer mut WriteStorage<'a, CharacterStateComponent>,
        physics_world: &'longer mut PhysicEngine,
        updater: &'longer mut LazyUpdate,
    ) -> SkillManifestationUpdateParam<'a, 'longer> {
        SkillManifestationUpdateParam {
            self_entity_id,
            all_collisions_in_world,
            sys_vars,
            entities,
            char_storage,
            physics_world,
            updater,
        }
    }

    pub fn remove_component<C>(&self, entitiy_id: Entity)
    where
        C: Component + Send + Sync,
    {
        self.updater.remove::<C>(entitiy_id);
    }

    pub fn insert_comp<C>(&self, entitiy_id: Entity, comp: C)
    where
        C: Component + Send + Sync,
    {
        self.updater.insert(entitiy_id, comp);
    }

    pub fn create_entity_with_comp<C>(&self, comp: C)
    where
        C: Component + Send + Sync,
    {
        self.insert_comp(self.entities.create(), comp);
    }

    pub fn now(&self) -> ElapsedTime {
        self.sys_vars.time
    }

    pub fn tick(&self) -> u64 {
        self.sys_vars.tick
    }

    pub fn dt(&self) -> DeltaTime {
        self.sys_vars.dt
    }

    pub fn assets(&self) -> &AssetResources {
        &self.sys_vars.assets
    }

    pub fn apply_status(&mut self, apply_status_comp: ApplyStatusComponent) {
        self.sys_vars.apply_statuses.push(apply_status_comp);
    }

    pub fn apply_area_status(&mut self, apply_status_comp: ApplyStatusInAreaComponent) {
        self.sys_vars.apply_area_statuses.push(apply_status_comp);
    }

    pub fn add_hp_mod_request(&mut self, hp_mod_req: HpModificationRequest) {
        self.sys_vars.hp_mod_requests.push(hp_mod_req);
    }

    pub fn add_area_hp_mod_request(&mut self, hp_mod_req: AreaAttackComponent) {
        self.sys_vars.area_hp_mod_requests.push(hp_mod_req);
    }

    pub fn apply_force(&mut self, force: ApplyForceComponent) {
        self.sys_vars.pushes.push(force);
    }
}

pub trait SkillManifestation {
    fn update(&mut self, params: SkillManifestationUpdateParam);

    fn render(
        &self,
        char_entity_storage: &ReadStorage<CharacterStateComponent>,
        now: ElapsedTime,
        tick: u64,
        assets: &AssetResources,
        render_commands: &mut RenderCommandCollector,
        audio_command_collector: &mut AudioCommandCollectorComponent,
    );
}

#[storage(HashMapStorage)]
#[derive(Component)]
pub struct SkillManifestationComponent {
    pub self_entity_id: Entity,
    pub skill: Arc<Mutex<Box<dyn SkillManifestation>>>,
}

impl SkillManifestationComponent {
    pub fn new(
        self_entity_id: Entity,
        skill: Box<dyn SkillManifestation>,
    ) -> SkillManifestationComponent {
        SkillManifestationComponent {
            self_entity_id,
            skill: Arc::new(Mutex::new(skill)),
        }
    }

    pub fn update(&mut self, params: SkillManifestationUpdateParam) {
        let mut skill = self.skill.lock().unwrap();
        skill.update(params);
    }

    pub fn render(
        &self,
        char_entity_storage: &ReadStorage<CharacterStateComponent>,
        now: ElapsedTime,
        tick: u64,
        assets: &AssetResources,
        render_commands: &mut RenderCommandCollector,
        audio_commands: &mut AudioCommandCollectorComponent,
    ) {
        let skill = self.skill.lock().unwrap();
        skill.render(
            char_entity_storage,
            now,
            tick,
            assets,
            render_commands,
            audio_commands,
        );
    }
}

unsafe impl Sync for SkillManifestationComponent {}

unsafe impl Send for SkillManifestationComponent {}

pub struct FinishCast {
    pub skill: Skills,
    pub caster_entity_id: CharEntityId,
    pub caster_pos: Vec2,
    pub caster_team: Team,
    pub skill_pos: Option<Vec2>,
    pub char_to_skill_dir: Vec2,
    pub target_entity: Option<CharEntityId>,
}

pub trait SkillDef {
    fn get_icon_path(&self) -> &'static str;
    fn finish_cast(
        &self,
        params: &FinishCast,
        ecs_world: &mut World,
    ) -> Option<Box<dyn SkillManifestation>>;

    fn get_skill_target_type(&self) -> SkillTargetType;
    fn render_casting(
        &self,
        char_pos: &Vec2,
        casting_state: &CastingSkillData,
        sys_vars: &SystemVariables,
        dev_configs: &DevConfig,
        render_commands: &mut RenderCommandCollector,
        char_storage: &ReadStorage<CharacterStateComponent>,
    ) {
        RenderDesktopClientSystem::render_str(
            StrEffectType::Moonstar,
            casting_state.cast_started,
            char_pos,
            &sys_vars.assets,
            sys_vars.time,
            render_commands,
            ActionPlayMode::Repeat,
        );
        if let Some(target_area_pos) = casting_state.target_area_pos {
            self.render_target_selection(
                true,
                &target_area_pos,
                &casting_state.char_to_skill_dir_when_casted,
                render_commands,
                dev_configs,
            );
        } else if let Some(target_entity) = casting_state.target_entity {
            if let Some(target_char) = char_storage.get(target_entity.into()) {
                render_commands
                    .horizontal_texture_3d()
                    .rotation_rad(sys_vars.time.0 % 6.28)
                    .pos(&target_char.pos())
                    .add(sys_vars.assets.sprites.magic_target)
            }
        }
    }
    fn render_target_selection(
        &self,
        _is_castable: bool,
        _skill_pos: &Vec2,
        _char_to_skill_dir: &Vec2,
        _render_commands: &mut RenderCommandCollector,
        _configs: &DevConfig,
    ) {
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash, EnumIter, Serialize, Deserialize)]
pub enum Skills {
    AttackMove,
    FireWall,
    BrutalTestSkill,
    Lightning,
    Heal,
    Mounting,
    Poison,
    Cure,
    FireBomb,
    AbsorbShield,
    WizPyroBlast,
    AssaBladeDash,
    AssaPhasePrism,
    GazXplodiumCharge,
    GazTurret,
    GazBarricade,
    GazDestroyTurret,
    GazTurretTarget,
    FalconCarry,
    FalconAttack,
    Sanctuary,
    ExoSkeleton,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SkillCastingAttributes {
    pub casting_time: ElapsedTime,
    pub cast_delay: ElapsedTime,
    pub casting_range: f32,
    // in case of Directional skills
    pub width: Option<f32>,
}

pub struct AttackMoveSkill;

pub const ATTACK_MOVE_SKILL: &'static AttackMoveSkill = &AttackMoveSkill;

impl SkillDef for AttackMoveSkill {
    fn get_icon_path(&self) -> &'static str {
        ""
    }

    fn finish_cast(
        &self,
        _params: &FinishCast,
        _ecs_world: &mut World,
    ) -> Option<Box<dyn SkillManifestation>> {
        None
    }

    fn get_skill_target_type(&self) -> SkillTargetType {
        SkillTargetType::Area
    }
}

impl Skills {
    pub fn get_definition(&self) -> &'static dyn SkillDef {
        match self {
            Skills::WizPyroBlast => WIZ_PYRO_BLAST_SKILL,
            Skills::FireWall => FIRE_WALL_SKILL,
            Skills::Heal => HEAL_SKILL,
            Skills::BrutalTestSkill => BRUTAL_TEST_SKILL,
            Skills::Lightning => LIGHTNING_SKILL,
            Skills::Mounting => MOUNTING_SKILL,
            Skills::Poison => POISON_SKILL,
            Skills::Cure => CURE_SKILL,
            Skills::FireBomb => FIRE_BOMB_SKILL,
            Skills::AbsorbShield => ABSORB_SHIELD_SKILL,
            Skills::AssaBladeDash => ASSA_BLADE_DASH_SKILL,
            Skills::AssaPhasePrism => ASSA_PHASE_PRISM_SKILL,
            Skills::GazXplodiumCharge => GAZ_XPLODIUM_CHARGE_SKILL,
            Skills::GazTurret => GAZ_TURRET_SKILL,
            Skills::GazDestroyTurret => GAZ_DESTROY_TURRET_SKILL,
            Skills::GazTurretTarget => GAZ_TURRET_TARGET_SKILL,
            Skills::FalconCarry => FALCON_CARRY_SKILL,
            Skills::FalconAttack => FALCON_ATTACK_SKILL,
            Skills::Sanctuary => SANCTUARY_SKILL,
            Skills::ExoSkeleton => EXO_SKELETON_SKILL,
            Skills::AttackMove => ATTACK_MOVE_SKILL,
            Skills::GazBarricade => GAZ_BARRICADE_SKILL,
        }
    }

    pub fn get_cast_attributes<'a>(
        &'a self,
        configs: &'a DevConfig,
        char_state: &CharacterStateComponent,
    ) -> &'a SkillCastingAttributes {
        match self {
            Skills::WizPyroBlast => &configs.skills.wiz_pyroblast.attributes,
            Skills::FireWall => &configs.skills.firewall.attributes,
            Skills::Heal => &configs.skills.heal.attributes,
            Skills::BrutalTestSkill => &configs.skills.brutal_test_skill.attributes,
            Skills::Lightning => &configs.skills.lightning.attributes,
            Skills::Mounting => {
                if char_state.statuses.is_mounted() {
                    &configs.skills.unmounting
                } else {
                    &configs.skills.mounting
                }
            }
            Skills::Poison => &configs.skills.poison.attributes,
            Skills::Cure => &configs.skills.cure,
            Skills::FireBomb => &configs.skills.firebomb.attributes,
            Skills::AbsorbShield => &configs.skills.absorb_shield.attributes,
            Skills::AssaBladeDash => &configs.skills.assa_blade_dash.attributes,
            Skills::AssaPhasePrism => &configs.skills.assa_phase_prism.attributes,
            Skills::GazXplodiumCharge => &configs.skills.gaz_xplodium_charge.attributes,
            Skills::GazTurret => &configs.skills.gaz_turret.attributes,
            Skills::GazDestroyTurret => &configs.skills.gaz_destroy_turret,
            Skills::GazTurretTarget => &SkillCastingAttributes {
                casting_time: ElapsedTime(0.0),
                cast_delay: ElapsedTime(0.0),
                casting_range: 999_999_999.0,
                width: None,
            },
            Skills::FalconCarry => &configs.skills.falcon_carry.attributes,
            Skills::FalconAttack => &configs.skills.falcon_attack.attributes,
            Skills::Sanctuary => &configs.skills.sanctuary.attributes,
            Skills::ExoSkeleton => &configs.skills.exoskeleton.attributes,
            Skills::AttackMove => &SkillCastingAttributes {
                casting_time: ElapsedTime(0.0),
                cast_delay: ElapsedTime(0.0),
                casting_range: 200_000_000.0,
                width: None,
            },
            Skills::GazBarricade => &configs.skills.gaz_barricade.attributes,
        }
    }

    pub fn limit_vector_into_range(char_pos: &Vec2, mouse_pos: &Vec2, range: f32) -> (Vec2, Vec2) {
        let dir2d = mouse_pos - char_pos;
        let dir_vector = dir2d.normalize();
        let pos = char_pos + dir_vector * dir2d.magnitude().min(range);
        return (pos, dir_vector);
    }

    pub fn render_casting_box(
        is_castable: bool,
        casting_area_size: &Vec2,
        skill_pos: &Vec2,
        char_to_skill_dir: &Vec2,
        render_commands: &mut RenderCommandCollector,
    ) {
        let angle = char_to_skill_dir.angle(&Vector2::y());
        let angle = if char_to_skill_dir.x > 0.0 {
            angle
        } else {
            -angle
        };
        let skill_pos = v2_to_v3(skill_pos);

        render_commands
            .rectangle_3d()
            .pos(&skill_pos)
            .rotation_rad(angle)
            .color(
                &(if is_castable {
                    [0, 255, 0, 255]
                } else {
                    [179, 179, 179, 255]
                }),
            )
            .size(casting_area_size.x, casting_area_size.y)
            .add()
    }

    pub fn is_casting_allowed_based_on_target(
        skill_target_type: SkillTargetType,
        skill_casting_range: f32,
        caster_id: CharEntityId,
        target_entity: Option<CharEntityId>,
        target_distance: f32,
    ) -> bool {
        match skill_target_type {
            SkillTargetType::Area => true,
            SkillTargetType::Directional => true,
            SkillTargetType::NoTarget => true,
            SkillTargetType::AnyEntity => {
                target_entity.is_some() && skill_casting_range >= target_distance
            }
            SkillTargetType::OnlyAllyButNoSelf => {
                target_entity.map(|it| it != caster_id).unwrap_or(false)
                    && skill_casting_range >= target_distance
            }
            SkillTargetType::OnlyAllyAndSelf => {
                target_entity.is_some() && skill_casting_range >= target_distance
            }
            SkillTargetType::OnlyEnemy => {
                target_entity.is_some() && skill_casting_range >= target_distance
            }
        }
    }
}

#[derive(Eq, PartialEq)]
#[allow(dead_code)]
pub enum SkillTargetType {
    /// casts immediately
    NoTarget,
    Area,
    Directional,
    AnyEntity,
    OnlyAllyButNoSelf,
    OnlyAllyAndSelf,
    OnlyEnemy,
}
