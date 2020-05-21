use nalgebra::Isometry2;

use crate::audio::sound_sys::AudioCommandCollectorComponent;
use crate::components::char::CharacterStateComponent;
use crate::components::skills::skills::{
    FinishCast, SkillDef, SkillManifestation, SkillManifestationComponent,
    SkillManifestationUpdateParam, SkillTargetType, Skills,
};
use crate::render::opengl_render_sys::Trimesh3dType;
use crate::render::render_command::RenderCommandCollector;
use crate::systems::{AssetResources, SystemVariables};
use crate::GameTime;
use rustarok_common::attack::{AreaAttackComponent, HpModificationType};
use rustarok_common::common::{v2, EngineTime, Local, Vec2};
use rustarok_common::components::char::{EntityId, StaticCharDataComponent};
use rustarok_common::config::CommonConfigs;
use specs::world::WorldExt;
use specs::ReadStorage;

pub struct SanctuarySkill;

pub const SANCTUARY_SKILL: &'static SanctuarySkill = &SanctuarySkill;

impl SkillDef for SanctuarySkill {
    fn get_icon_path(&self) -> &'static str {
        "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\wz_meteor.bmp"
    }

    fn finish_cast(
        &self,
        params: &FinishCast,
        ecs_world: &mut specs::world::World,
    ) -> Option<Box<dyn SkillManifestation>> {
        let configs = &ecs_world.read_resource::<CommonConfigs>().skills.sanctuary;
        Some(Box::new(SanctuarySkillManifest::new(
            params.caster_entity_id,
            &params.skill_pos.unwrap(),
            configs.heal,
            configs.heal_freq_seconds,
            ecs_world.read_resource::<EngineTime>().now(),
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
        _char_to_skill_dir: &Vec2,
        render_commands: &mut RenderCommandCollector,
        _configs: &CommonConfigs,
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
    pub caster_entity_id: EntityId<Local>,
    pub pos: Vec2,
    pub created_at: GameTime<Local>,
    pub die_at: GameTime<Local>,
    pub next_heal_at: GameTime<Local>,
    pub heal_freq: f32,
    pub heal: u32,
}

impl SanctuarySkillManifest {
    pub fn new(
        caster_entity_id: EntityId<Local>,
        skill_center: &Vec2,
        heal: u32,
        heal_freq: f32,
        system_time: GameTime<Local>,
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
    fn update(&mut self, mut params: SkillManifestationUpdateParam) {
        if self.die_at.has_already_passed(params.time().now()) {
            params.remove_component::<SkillManifestationComponent>(params.self_entity_id);
        } else {
            if self.next_heal_at.has_not_passed_yet(params.time().now()) {
                return;
            }
            self.next_heal_at = params.time().now().add_seconds(self.heal_freq);
            params.add_area_hp_mod_request(AreaAttackComponent {
                // TODO2
                //                area_shape: Box::new(ncollide2d::shape::Cuboid::new(v2(2.5, 2.5))),
                //                area_isom: Isometry2::new(self.pos, 0.0),
                source_entity_id: self.caster_entity_id,
                typ: HpModificationType::Heal(self.heal),
                except: None,
            });
        }
    }

    fn render(
        &self,
        _char_entity_storage: &ReadStorage<StaticCharDataComponent>,
        _now: GameTime<Local>,
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
