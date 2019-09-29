use crate::components::char::{
    CharOutlook, CharPhysicsEntityBuilder, CharStateComponentBuilder, CharacterEntityBuilder,
    CharacterStateComponent, NpcComponent,
};
use crate::components::controller::{CharEntityId, WorldCoord};
use crate::components::skills::skills::{SkillDef, SkillManifestation, SkillTargetType, Skills};
use crate::configs::DevConfig;
use crate::consts::{JobId, MonsterId};
use crate::runtime_assets::map::PhysicEngine;
use crate::systems::render::render_command::RenderCommandCollector;
use crate::systems::SystemVariables;
use nalgebra::Vector2;
use nphysics2d::object::BodyStatus;
use specs::LazyUpdate;

pub struct GazBarricadeSkill;

pub const GAZ_BARRICADE_SKILL: &'static GazBarricadeSkill = &GazBarricadeSkill;

impl SkillDef for GazBarricadeSkill {
    fn get_icon_path(&self) -> &'static str {
        "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\gn_cartcannon.bmp"
    }

    fn finish_cast(
        &self,
        caster_entity_id: CharEntityId,
        caster_pos: WorldCoord,
        skill_pos: Option<Vector2<f32>>,
        char_to_skill_dir: &Vector2<f32>,
        target_entity: Option<CharEntityId>,
        ecs_world: &mut specs::world::World,
    ) -> Option<Box<dyn SkillManifestation>> {
        if let Some(caster) = ecs_world
            .read_storage::<CharacterStateComponent>()
            .get(caster_entity_id.0)
        {
            let entities = &ecs_world.entities();
            let updater = &ecs_world.read_resource::<LazyUpdate>();
            let system_vars = ecs_world.read_resource::<SystemVariables>();
            let char_entity_id = CharEntityId(entities.create());
            updater.insert(char_entity_id.0, NpcComponent);
            let pos2d = {
                let pos = skill_pos.unwrap();
                Vector2::new((pos.x as i32) as f32, (pos.y as i32) as f32)
            };
            CharacterEntityBuilder::new(char_entity_id, "barricade")
                .insert_sprite_render_descr_component(updater)
                .physics(
                    CharPhysicsEntityBuilder::new(pos2d)
                        .collision_group(caster.team.get_barricade_collision_group())
                        .rectangle(1.0, 1.0)
                        .body_status(BodyStatus::Static),
                    &mut ecs_world.write_resource::<PhysicEngine>(),
                )
                .char_state(
                    CharStateComponentBuilder::new()
                        .outlook(CharOutlook::Monster(MonsterId::Barricade))
                        .job_id(JobId::Barricade)
                        .team(caster.team),
                    updater,
                    &ecs_world.read_resource::<DevConfig>(),
                );
        }
        None
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
