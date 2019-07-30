use crate::components::char::CharacterStateComponent;
use crate::components::skills::skill::{SkillManifestation, Skills};
use crate::components::status::{ApplyStatusComponentPayload, ApplyStatusInAreaComponent};
use crate::systems::render::ONE_SPRITE_PIXEL_SIZE_IN_3D;
use crate::systems::{Collision, SystemVariables};
use crate::video::TEXTURE_0;
use crate::{ElapsedTime, PhysicsWorld};
use nalgebra::{Isometry2, Matrix4, Vector2, Vector3};
use specs::{Entity, LazyUpdate};

pub struct StatusApplierArea<F>
where
    F: FnMut(ElapsedTime) -> ApplyStatusComponentPayload,
{
    pub half_extents: Vector2<f32>,
    pub pos: Vector2<f32>,
    pub name: &'static str,
    pub status_creator: F,
    pub caster_entity_id: Entity,
    pub next_action_at: ElapsedTime,
}

impl<F> StatusApplierArea<F>
where
    F: FnMut(ElapsedTime) -> ApplyStatusComponentPayload,
{
    pub fn new(
        name: &'static str,
        status_creator: F,
        skill_center: &Vector2<f32>,
        size: Vector2<f32>,
        caster_entity_id: Entity,
    ) -> StatusApplierArea<F> {
        let half_extents = v2!(size.x / 2.0, size.y / 2.0);

        StatusApplierArea {
            name,
            status_creator,
            pos: *skill_center,
            half_extents,
            caster_entity_id,
            next_action_at: ElapsedTime(0.0),
        }
    }
}

impl<F> SkillManifestation for StatusApplierArea<F>
where
    F: FnMut(ElapsedTime) -> ApplyStatusComponentPayload,
{
    fn update(
        &mut self,
        self_entity_id: Entity,
        all_collisions_in_world: &Vec<Collision>,
        system_vars: &mut SystemVariables,
        entities: &specs::Entities,
        char_storage: &specs::ReadStorage<CharacterStateComponent>,
        _physics_world: &mut PhysicsWorld,
        updater: &mut specs::Write<LazyUpdate>,
    ) {
        if self.next_action_at.has_passed(system_vars.time) {
            system_vars
                .apply_area_statuses
                .push(ApplyStatusInAreaComponent {
                    source_entity_id: self.caster_entity_id,
                    status: (self.status_creator)(system_vars.time),
                    area_shape: Box::new(ncollide2d::shape::Cuboid::new(self.half_extents)),
                    area_isom: Isometry2::new(self.pos, 0.0),
                    except: None,
                });
            self.next_action_at = system_vars.time.add_seconds(2.0);
        }
    }

    fn render(&self, system_vars: &SystemVariables, view_matrix: &Matrix4<f32>) {
        Skills::render_casting_box2(
            &self.pos,
            &self.half_extents,
            0.0,
            &system_vars,
            view_matrix,
        );
        let shader = system_vars.shaders.sprite_shader.gl_use();
        let texture = &system_vars.texts.custom_texts[self.name];
        shader.set_vec2(
            "size",
            &[
                texture.width as f32 * ONE_SPRITE_PIXEL_SIZE_IN_3D,
                texture.height as f32 * ONE_SPRITE_PIXEL_SIZE_IN_3D,
            ],
        );
        let mut matrix = Matrix4::<f32>::identity();
        matrix.prepend_translation_mut(&v3!(self.pos.x, 3.0, self.pos.y));
        shader.set_mat4("model", &matrix);
        shader.set_vec3("color", &[1.0, 1.0, 1.0]);
        shader.set_vec2("offset", &[0.0, 0.0]);
        shader.set_mat4("projection", &system_vars.matrices.projection);
        shader.set_mat4("view", &view_matrix);
        shader.set_int("model_texture", 0);
        shader.set_f32("alpha", 1.0);
        texture.bind(TEXTURE_0);
        system_vars
            .map_render_data
            .centered_sprite_vertex_array
            .bind()
            .draw();
    }
}
