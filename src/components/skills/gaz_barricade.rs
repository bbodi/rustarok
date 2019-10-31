use crate::common::{v2, Vec2};
use crate::components::char::{CharacterStateComponent, NpcComponent};
use crate::components::controller::CharEntityId;
use crate::components::skills::skills::{SkillDef, SkillManifestation, SkillTargetType, Skills};
use crate::configs::DevConfig;
use crate::systems::render::render_command::RenderCommandCollector;
use crate::systems::spawn_entity_system::SpawnEntitySystem;
use nalgebra::Vector2;
use specs::LazyUpdate;

pub struct GazBarricadeSkill;

pub const GAZ_BARRICADE_SKILL: &'static GazBarricadeSkill = &GazBarricadeSkill;

impl SkillDef for GazBarricadeSkill {
    fn get_icon_path(&self) -> &'static str {
        "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\gn_cartcannon.bmp"
    }

    // TODO 2 barricade ne lehessen egy kockán
    // TODO: teamet a finishből szedje
    // TODO: ezeket a paramétereket pakold egy structba és 1 paramétert adj át (undorito h 90%ban valamelyik apram nincs használva)
    fn finish_cast(
        &self,
        caster_entity_id: CharEntityId,
        _caster_pos: Vec2,
        skill_pos: Option<Vec2>,
        _char_to_skill_dir: &Vec2,
        _target_entity: Option<CharEntityId>,
        ecs_world: &mut specs::world::World,
    ) -> Option<Box<dyn SkillManifestation>> {
        if let Some(caster) = ecs_world
            .read_storage::<CharacterStateComponent>()
            .get(caster_entity_id.0)
        {
            let entities = &ecs_world.entities();
            let updater = &ecs_world.read_resource::<LazyUpdate>();
            let char_entity_id = CharEntityId(entities.create());
            updater.insert(char_entity_id.0, NpcComponent);
            let pos2d = {
                let pos = skill_pos.unwrap();
                Vec2::new((pos.x as i32) as f32, (pos.y as i32) as f32)
            };

            SpawnEntitySystem::create_barricade(
                entities,
                &updater,
                &mut ecs_world.write_resource(),
                &ecs_world.read_resource(),
                caster.team,
                pos2d,
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
