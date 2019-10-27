use nalgebra::Isometry2;
use specs::{Entity, LazyUpdate};

use crate::common::{v2, Vec2};
use crate::components::char::CharacterStateComponent;
use crate::components::controller::CharEntityId;
use crate::components::skills::skills::{
    SkillDef, SkillManifestation, SkillManifestationComponent, SkillTargetType, Skills,
    WorldCollisions,
};
use crate::components::{AreaAttackComponent, HpModificationType};
use crate::configs::DevConfig;
use crate::runtime_assets::map::PhysicEngine;
use crate::systems::render::render_command::{RenderCommandCollector, Trimesh3dType};
use crate::systems::sound_sys::AudioCommandCollectorComponent;
use crate::systems::{AssetResources, SystemVariables};
use crate::ElapsedTime;

pub struct SanctuarySkill;

pub const SANCTUARY_SKILL: &'static SanctuarySkill = &SanctuarySkill;

impl SkillDef for SanctuarySkill {
    fn get_icon_path(&self) -> &'static str {
        "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\wz_meteor.bmp"
    }

    fn finish_cast(
        &self,
        caster_entity_id: CharEntityId,
        caster_pos: Vec2,
        skill_pos: Option<Vec2>,
        char_to_skill_dir: &Vec2,
        target_entity: Option<CharEntityId>,
        ecs_world: &mut specs::world::World,
    ) -> Option<Box<dyn SkillManifestation>> {
        let configs = &ecs_world.read_resource::<DevConfig>().skills.sanctuary;
        Some(Box::new(SanctuarySkillManifest::new(
            caster_entity_id,
            &skill_pos.unwrap(),
            configs.heal,
            configs.heal_freq_seconds,
            ecs_world.read_resource::<SystemVariables>().time,
            configs.duration,
        )))
    }

    fn get_skill_target_type(&self) -> SkillTargetType {
        SkillTargetType::Area
    }

    fn render_target_selection(
        &self,
        is_castable: bool,
        skill_pos: &Vec2,
        char_to_skill_dir: &Vec2,
        render_commands: &mut RenderCommandCollector,
        configs: &DevConfig,
    ) {
        Skills::render_casting_box(
            is_castable,
            &v2(5.0, 5.0),
            skill_pos,
            &v2(0.0, 0.0),
            render_commands,
        );
    }
}

pub struct SanctuarySkillManifest {
    pub caster_entity_id: CharEntityId,
    pub pos: Vec2,
    pub created_at: ElapsedTime,
    pub die_at: ElapsedTime,
    pub next_heal_at: ElapsedTime,
    pub heal_freq: f32,
    pub heal: u32,
}

impl SanctuarySkillManifest {
    pub fn new(
        caster_entity_id: CharEntityId,
        skill_center: &Vec2,
        heal: u32,
        heal_freq: f32,
        system_time: ElapsedTime,
        duration: f32,
    ) -> SanctuarySkillManifest {
        SanctuarySkillManifest {
            caster_entity_id,
            pos: *skill_center,
            created_at: system_time.clone(),
            die_at: system_time.add_seconds(duration),
            next_heal_at: system_time,
            heal,
            heal_freq,
        }
    }
}

impl SkillManifestation for SanctuarySkillManifest {
    fn update(
        &mut self,
        self_entity_id: Entity,
        _all_collisions_in_world: &WorldCollisions,
        sys_vars: &mut SystemVariables,
        _entities: &specs::Entities,
        _char_storage: &mut specs::WriteStorage<CharacterStateComponent>,
        _physics_world: &mut PhysicEngine,
        updater: &mut LazyUpdate,
    ) {
        if self.die_at.has_already_passed(sys_vars.time) {
            updater.remove::<SkillManifestationComponent>(self_entity_id);
        } else {
            if self.next_heal_at.has_not_passed_yet(sys_vars.time) {
                return;
            }
            self.next_heal_at = sys_vars.time.add_seconds(self.heal_freq);
            sys_vars.area_hp_mod_requests.push(AreaAttackComponent {
                area_shape: Box::new(ncollide2d::shape::Cuboid::new(v2(2.5, 2.5))),
                area_isom: Isometry2::new(self.pos, 0.0),
                source_entity_id: self.caster_entity_id,
                typ: HpModificationType::Heal(self.heal),
                except: None,
            });
        }
    }

    fn render(
        &self,
        _now: ElapsedTime,
        _tick: u64,
        _assets: &AssetResources,
        render_commands: &mut RenderCommandCollector,
        _audio_commands: &mut AudioCommandCollectorComponent,
    ) {
        render_commands
            .trimesh3d()
            .pos_2d(&self.pos)
            .add(Trimesh3dType::Sanctuary);
    }
}
