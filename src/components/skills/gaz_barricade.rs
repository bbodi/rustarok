use crate::components::controller::WorldCoord;
use crate::components::skills::skills::{
    FinishCast, FinishSimpleSkillCastComponent, SkillDef, SkillTargetType, Skills,
};
use crate::configs::DevConfig;
use crate::systems::render::render_command::RenderCommandCollector;
use crate::systems::spawn_entity_system::{SpawnEntityComponent, SpawnEntityType};
use crate::systems::SystemVariables;
use nalgebra::Vector2;
use specs::{Entities, LazyUpdate};

pub struct GazBarricadeSkill;

pub const GAZ_BARRICADE_SKILL: &'static GazBarricadeSkill = &GazBarricadeSkill;

impl SkillDef for GazBarricadeSkill {
    fn get_icon_path(&self) -> &'static str {
        "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\gn_cartcannon.bmp"
    }

    // TODO 2 barricade ne lehessen egy kockán
    fn finish_cast(&self, finish_cast_data: FinishCast, entities: &Entities, updater: &LazyUpdate) {
        updater.insert(
            entities.create(),
            FinishSimpleSkillCastComponent::new(
                finish_cast_data,
                GazBarricadeSkill::do_finish_cast,
            ),
        )
    }

    fn get_skill_target_type(&self) -> SkillTargetType {
        SkillTargetType::Area
    }

    fn render_target_selection(
        &self,
        is_castable: bool,
        skill_pos: &WorldCoord,
        char_to_skill_dir: &Vector2<f32>,
        render_commands: &mut RenderCommandCollector,
        configs: &DevConfig,
    ) {
        let pos2d = { Vector2::new((skill_pos.x as i32) as f32, (skill_pos.y as i32) as f32) };
        Skills::render_casting_box(
            is_castable,
            &Vector2::new(1.0, 1.0),
            &pos2d,
            &Vector2::zeros(),
            render_commands,
        );
    }
}

impl GazBarricadeSkill {
    fn do_finish_cast(
        finish_cast: &FinishCast,
        entities: &Entities,
        updater: &LazyUpdate,
        dev_configs: &DevConfig,
        sys_vars: &mut SystemVariables,
    ) {
        let pos2d = {
            let pos = finish_cast.skill_pos.unwrap();
            Vector2::new((pos.x as i32) as f32, (pos.y as i32) as f32)
        };
        updater.insert(
            entities.create(),
            SpawnEntityComponent::new(SpawnEntityType::Barricade {
                pos: pos2d,
                team: finish_cast.caster_team,
            }),
        );
    }
}
