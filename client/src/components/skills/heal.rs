use specs::{LazyUpdate, ReadStorage};

use crate::components::skills::skills::{
    FinishCast, SkillDef, SkillManifestation, SkillManifestationComponent,
    SkillManifestationUpdateParam, SkillTargetType,
};

use crate::audio::sound_sys::AudioCommandCollectorComponent;
use crate::components::char::CharacterStateComponent;
use crate::components::SoundEffectComponent;
use crate::render::opengl_render_sys::Trimesh3dType;
use crate::render::render_command::RenderCommandCollector;
use crate::systems::{AssetResources, SystemVariables};
use rustarok_common::attack::{HpModificationRequest, HpModificationType};
use rustarok_common::common::{EngineTime, LocalTime};
use rustarok_common::components::char::{LocalCharEntityId, StaticCharDataComponent};
use rustarok_common::config::CommonConfigs;
use specs::world::WorldExt;

pub struct HealSkill;

pub const HEAL_SKILL: &'static HealSkill = &HealSkill;

impl SkillDef for HealSkill {
    fn get_icon_path(&self) -> &'static str {
        "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\al_heal.bmp"
    }

    fn finish_cast(
        &self,
        params: &FinishCast,
        ecs_world: &mut specs::world::World,
    ) -> Option<Box<dyn SkillManifestation>> {
        let target_entity_id = params.target_entity.unwrap();
        let entities = &ecs_world.entities();
        let updater = ecs_world.read_resource::<LazyUpdate>();
        let mut sys_vars = ecs_world.write_resource::<SystemVariables>();
        let now = ecs_world.read_resource::<EngineTime>().now();
        let entity = entities.create();
        updater.insert(
            entity,
            SoundEffectComponent {
                target_entity_id,
                sound_id: sys_vars.assets.sounds.heal,
                pos: params.caster_pos,
                start_time: now,
            },
        );
        // TODO: helper(&mut ecs_world).add_hp_mod()...
        ecs_world
            .write_resource::<Vec<HpModificationRequest>>()
            .push(HpModificationRequest {
                src_entity: params.caster_entity_id,
                dst_entity: target_entity_id,
                typ: HpModificationType::Heal(
                    ecs_world.read_resource::<CommonConfigs>().skills.heal.heal,
                ),
            });
        return Some(Box::new(HealSkillManifest::new(target_entity_id, now)));
    }

    fn get_skill_target_type(&self) -> SkillTargetType {
        SkillTargetType::OnlyAllyAndSelf
    }
}

pub struct HealSkillManifest {
    pub target_entity_id: LocalCharEntityId,
    pub created_at: LocalTime,
    pub middle: LocalTime,
    pub die_at: LocalTime,
}

impl HealSkillManifest {
    pub fn new(target_entity_id: LocalCharEntityId, system_time: LocalTime) -> HealSkillManifest {
        HealSkillManifest {
            target_entity_id,
            created_at: system_time,
            middle: system_time.add_seconds(1.0),
            die_at: system_time.add_seconds(2.0),
        }
    }
}

impl SkillManifestation for HealSkillManifest {
    fn update(&mut self, params: SkillManifestationUpdateParam) {
        if self.die_at.has_already_passed(params.time().now()) {
            params.remove_component::<SkillManifestationComponent>(params.self_entity_id);
        }
    }

    fn render(
        &self,
        char_entity_storage: &ReadStorage<StaticCharDataComponent>,
        now: LocalTime,
        _assets: &AssetResources,
        render_commands: &mut RenderCommandCollector,
        _audio_commands: &mut AudioCommandCollectorComponent,
    ) {
        // TODO2
        //        if let Some(target_char) = char_entity_storage.get(self.target_entity_id.into()) {
        //            let first_half = now
        //                .percentage_between(self.created_at, self.middle)
        //                .min(1.0);
        //            let second_half = now
        //                .percentage_between(self.middle, self.die_at)
        //                .max(0.0)
        //                .min(1.0);
        //
        //            let a = ((first_half - second_half) * 255.0) as u8;
        //
        //            render_commands
        //                .trimesh3d()
        //                .pos_2d(&target_char.pos())
        //                .color(&[196, 255, 196, a])
        //                .add(Trimesh3dType::SphericalCylinder);
        //        }
    }
}
