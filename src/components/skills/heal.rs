use specs::{LazyUpdate, ReadStorage};

use crate::components::skills::skills::{
    FinishCast, SkillDef, SkillManifestation, SkillManifestationComponent,
    SkillManifestationUpdateParam, SkillTargetType,
};

use crate::common::ElapsedTime;
use crate::components::char::CharacterStateComponent;
use crate::components::{HpModificationRequest, HpModificationType, SoundEffectComponent};
use crate::configs::DevConfig;
use crate::systems::render::opengl_render_sys::Trimesh3dType;
use crate::systems::render::render_command::RenderCommandCollector;
use crate::systems::sound_sys::AudioCommandCollectorComponent;
use crate::systems::{AssetResources, CharEntityId, SystemVariables};

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
        let entity = entities.create();
        updater.insert(
            entity,
            SoundEffectComponent {
                target_entity_id,
                sound_id: sys_vars.assets.sounds.heal,
                pos: params.caster_pos,
                start_time: sys_vars.time,
            },
        );
        sys_vars.hp_mod_requests.push(HpModificationRequest {
            src_entity: params.caster_entity_id,
            dst_entity: target_entity_id,
            typ: HpModificationType::Heal(ecs_world.read_resource::<DevConfig>().skills.heal.heal),
        });
        return Some(Box::new(HealSkillManifest::new(
            target_entity_id,
            sys_vars.time,
        )));
    }

    fn get_skill_target_type(&self) -> SkillTargetType {
        SkillTargetType::OnlyAllyAndSelf
    }
}

pub struct HealSkillManifest {
    pub target_entity_id: CharEntityId,
    pub created_at: ElapsedTime,
    pub middle: ElapsedTime,
    pub die_at: ElapsedTime,
}

impl HealSkillManifest {
    pub fn new(target_entity_id: CharEntityId, system_time: ElapsedTime) -> HealSkillManifest {
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
        if self.die_at.has_already_passed(params.now()) {
            params.remove_component::<SkillManifestationComponent>(params.self_entity_id);
        }
    }

    fn render(
        &self,
        char_entity_storage: &ReadStorage<CharacterStateComponent>,
        now: ElapsedTime,
        _tick: u64,
        _assets: &AssetResources,
        render_commands: &mut RenderCommandCollector,
        _audio_commands: &mut AudioCommandCollectorComponent,
    ) {
        if let Some(target_char) = char_entity_storage.get(self.target_entity_id.into()) {
            let first_half = now
                .percentage_between(self.created_at, self.middle)
                .min(1.0);
            let second_half = now
                .percentage_between(self.middle, self.die_at)
                .max(0.0)
                .min(1.0);

            let a = ((first_half - second_half) * 255.0) as u8;

            render_commands
                .trimesh3d()
                .pos_2d(&target_char.pos())
                .color(&[196, 255, 196, a])
                .add(Trimesh3dType::SphericalCylinder);
            //            RenderDesktopClientSystem::render_str(
            //                StrEffectType::Superstar,
            //                self.created_at,
            //                &target_char.pos(),
            //                assets,
            //                now,
            //                render_commands,
            //                ActionPlayMode::Repeat,
            //            );
        }
    }
}
