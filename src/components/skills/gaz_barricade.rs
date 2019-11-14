use crate::common::{v2, Vec2};
use crate::components::char::{CharacterStateComponent, NpcComponent};
use crate::components::controller::CharEntityId;
use crate::components::skills::skills::{
    FinishCast, SkillDef, SkillManifestation, SkillTargetType, Skills,
};
use crate::configs::DevConfig;
use crate::consts::JobId;
use crate::systems::render::render_command::RenderCommandCollector;
use crate::systems::spawn_entity_system::SpawnEntitySystem;
use nalgebra::Vector2;
use specs::prelude::*;
use specs::LazyUpdate;

pub struct GazBarricadeSkill;

pub const GAZ_BARRICADE_SKILL: &'static GazBarricadeSkill = &GazBarricadeSkill;

impl SkillDef for GazBarricadeSkill {
    fn get_icon_path(&self) -> &'static str {
        "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\gn_cartcannon.bmp"
    }

    // TODO if the skill is rejected due to occupied tile, sp should not be lowered
    fn finish_cast(
        &self,
        params: &FinishCast,
        ecs_world: &mut World,
    ) -> Option<Box<dyn SkillManifestation>> {
        let entities = &ecs_world.entities();
        let updater = &ecs_world.read_resource::<LazyUpdate>();
        let char_entity_id = CharEntityId(entities.create());
        updater.insert(char_entity_id.0, NpcComponent);
        let tile_pos = {
            let pos = params.skill_pos.unwrap();
            Vec2::new((pos.x as i32) as f32, (pos.y as i32) as f32)
        };

        for char_state in (&mut ecs_world.read_storage::<CharacterStateComponent>()).join() {
            if char_state.job_id != JobId::Barricade {
                continue;
            }
            if char_state.pos().x as i32 == tile_pos.x as i32
                && char_state.pos().y as i32 == tile_pos.y as i32
            {
                // tile is already occupied
                return None;
            }
        }

        SpawnEntitySystem::create_barricade(
            entities,
            &updater,
            &mut ecs_world.write_resource(),
            &ecs_world.read_resource(),
            params.caster_team,
            tile_pos,
        );
        return None;
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
        _configs: &DevConfig,
    ) {
        let pos2d = { Vec2::new((skill_pos.x as i32) as f32, (skill_pos.y as i32) as f32) };
        Skills::render_casting_box(
            is_castable,
            &v2(1.0, 1.0),
            &pos2d,
            &Vector2::zeros(),
            render_commands,
        );
    }
}
