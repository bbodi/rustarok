use crate::audio::sound_sys::AudioCommandCollectorComponent;
use crate::components::char::CharacterStateComponent;
use crate::components::skills::skills::{SkillManifestation, SkillManifestationUpdateParam};
use crate::components::status::status::{ApplyStatusComponent, StatusEnum};
use crate::render::render_command::RenderCommandCollector;
use crate::systems::AssetResources;
use crate::{GameTime, Local, PhysicEngine};
use nalgebra::Vector2;
use nphysics2d::object::DefaultColliderHandle;
use rustarok_common::common::{v2, Vec2};
use rustarok_common::components::char::{EntityId, StaticCharDataComponent};
use specs::ReadStorage;

pub struct StatusApplierArea<F>
where
    F: FnMut(GameTime<Local>) -> StatusEnum,
{
    pub collider_handle: DefaultColliderHandle,
    pub extents: Vector2<u16>,
    pub pos: Vec2,
    pub name: String,
    pub status_creator: F,
    pub caster_entity_id: EntityId<Local>,
    pub next_action_at: GameTime<Local>,
}

impl<F> StatusApplierArea<F>
where
    F: FnMut(GameTime<Local>) -> StatusEnum,
{
    pub fn new(
        name: String,
        status_creator: F,
        skill_center: &Vec2,
        size: Vector2<u16>,
        caster_entity_id: EntityId<Local>,
        physics_world: &mut PhysicEngine,
    ) -> StatusApplierArea<F> {
        let (collider_handle, _body_handle) = physics_world.add_cuboid_skill_area(
            *skill_center,
            0.0,
            v2(size.x as f32, size.y as f32),
        );
        StatusApplierArea {
            collider_handle,
            name,
            status_creator,
            pos: *skill_center,
            extents: size,
            caster_entity_id,
            next_action_at: GameTime::from(0.0),
        }
    }
}

impl<F> SkillManifestation for StatusApplierArea<F>
where
    F: FnMut(GameTime<Local>) -> StatusEnum,
{
    fn update(&mut self, mut params: SkillManifestationUpdateParam) {
        if self.next_action_at.has_already_passed(params.time().now()) {
            let self_collider_handle = self.collider_handle;
            let my_collisions = params
                .all_collisions_in_world
                .iter()
                .filter(|(_key, coll)| coll.other_coll_handle == self_collider_handle);
            for (_key, coll) in my_collisions {
                let char_collider = params
                    .physics_world
                    .colliders
                    .get(coll.character_coll_handle)
                    .unwrap();
                let char_entity_id = *char_collider
                    .user_data()
                    .map(|v| v.downcast_ref().unwrap())
                    .unwrap();
                params.apply_status(ApplyStatusComponent {
                    source_entity_id: self.caster_entity_id,
                    target_entity_id: char_entity_id,
                    status: (self.status_creator)(params.time().now()),
                });
                self.next_action_at = params.time().now().add_seconds(2.0);
            }
        }
    }

    fn render(
        &self,
        _char_entity_storage: &ReadStorage<StaticCharDataComponent>,
        now: GameTime<Local>,
        assets: &AssetResources,
        render_commands: &mut RenderCommandCollector,
        _audio_commands: &mut AudioCommandCollectorComponent,
    ) {
        render_commands
            .rectangle_3d()
            .pos_2d(&self.pos)
            .color(&[0, 255, 0, 255])
            .size(self.extents.x as f32, self.extents.y as f32)
            .add();

        render_commands
            .sprite_3d()
            .pos_2d(&self.pos)
            .y(3.0)
            .color(&if self.next_action_at.has_already_passed(now) {
                [0, 255, 0, 255]
            } else {
                [77, 77, 77, 255]
            })
            .add(assets.texts.custom_texts[&self.name]);
    }
}
