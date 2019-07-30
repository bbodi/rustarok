use crate::components::char::CharacterStateComponent;
use crate::components::skills::skill::{SkillManifestation, SkillManifestationComponent};
use crate::components::{AreaAttackComponent, AttackType, StrEffectComponent};
use crate::systems::{Collision, SystemVariables};
use crate::video::draw_circle_inefficiently;
use crate::{ElapsedTime, PhysicsWorld};
use nalgebra::{Isometry2, Matrix4, Vector2};
use nphysics2d::object::ColliderHandle;
use specs::{Entity, LazyUpdate};
use std::collections::HashMap;

pub struct LightningSkill;

impl LightningSkill {
    pub fn render_target_selection(
        skill_pos: &Vector2<f32>,
        char_to_skill_dir: &Vector2<f32>,
        system_vars: &SystemVariables,
        view_matrix: &Matrix4<f32>,
    ) {
        for i in 0..3 {
            draw_circle_inefficiently(
                &system_vars.shaders.trimesh_shader,
                &system_vars.matrices.projection,
                view_matrix,
                &(skill_pos + char_to_skill_dir * i as f32 * 2.2),
                0.0,
                1.0,
                &[0.0, 1.0, 0.0, 1.0],
            );
        }
    }
}

pub struct LightningManifest {
    pub caster_entity_id: Entity,
    pub effect_id: Entity,
    pub pos: Vector2<f32>,
    pub dir_vector: Vector2<f32>,
    pub created_at: ElapsedTime,
    pub next_action_at: ElapsedTime,
    pub next_damage_at: ElapsedTime,
    pub last_skill_pos: Vector2<f32>,
    pub action_count: u8,
}

impl LightningManifest {
    pub fn new(
        caster_entity_id: Entity,
        skill_center: &Vector2<f32>,
        dir_vector: &Vector2<f32>,
        now: ElapsedTime,
        entities: &specs::Entities,
    ) -> LightningManifest {
        LightningManifest {
            caster_entity_id,
            effect_id: entities.create(),
            pos: *skill_center,
            created_at: now,
            next_action_at: now,
            next_damage_at: now,
            last_skill_pos: *skill_center,
            action_count: 0,
            dir_vector: *dir_vector,
        }
    }
}

impl SkillManifestation for LightningManifest {
    fn update(
        &mut self,
        self_entity_id: Entity,
        _all_collisions_in_world: &HashMap<(ColliderHandle, ColliderHandle), Collision>,
        system_vars: &mut SystemVariables,
        entities: &specs::Entities,
        char_storage: &specs::ReadStorage<CharacterStateComponent>,
        physics_world: &mut PhysicsWorld,
        updater: &mut specs::Write<LazyUpdate>,
    ) {
        if self
            .created_at
            .add_seconds(12.0)
            .has_passed(system_vars.time)
        {
            updater.remove::<SkillManifestationComponent>(self_entity_id);
            updater.remove::<StrEffectComponent>(self.effect_id);
        } else {
            if self.next_action_at.has_passed(system_vars.time) {
                updater.remove::<StrEffectComponent>(self.effect_id);
                let effect_comp = match self.action_count {
                    0 => StrEffectComponent {
                        effect: "StrEffect::Lightning".to_owned(),
                        pos: self.pos,
                        start_time: system_vars.time.add_seconds(-0.5),
                        die_at: system_vars.time.add_seconds(1.0),
                        duration: ElapsedTime(1.0),
                    },
                    1 => {
                        let pos = self.pos + self.dir_vector * 2.2;
                        StrEffectComponent {
                            effect: "StrEffect::Lightning".to_owned(),
                            pos,
                            start_time: system_vars.time.add_seconds(-0.5),
                            die_at: system_vars.time.add_seconds(1.0),
                            duration: ElapsedTime(1.0),
                        }
                    }
                    2 => {
                        let pos = self.pos + self.dir_vector * 2.0 * 2.2;
                        StrEffectComponent {
                            effect: "StrEffect::Lightning".to_owned(),
                            pos,
                            start_time: system_vars.time.add_seconds(-0.5),
                            die_at: system_vars.time.add_seconds(1.0),
                            duration: ElapsedTime(1.0),
                        }
                    }
                    3 => {
                        let pos = self.pos + self.dir_vector * 2.0 * 2.2;
                        StrEffectComponent {
                            effect: "StrEffect::Lightning".to_owned(),
                            pos,
                            start_time: system_vars.time.add_seconds(-0.5),
                            die_at: system_vars.time.add_seconds(1.0),
                            duration: ElapsedTime(1.0),
                        }
                    }
                    4 => {
                        let pos = self.pos + self.dir_vector * 2.2;
                        StrEffectComponent {
                            effect: "StrEffect::Lightning".to_owned(),
                            pos,
                            start_time: system_vars.time.add_seconds(-0.5),
                            die_at: system_vars.time.add_seconds(1.0),
                            duration: ElapsedTime(1.0),
                        }
                    }
                    5 => StrEffectComponent {
                        effect: "StrEffect::Lightning".to_owned(),
                        pos: self.pos,
                        start_time: system_vars.time.add_seconds(-0.5),
                        die_at: system_vars.time.add_seconds(1.0),
                        duration: ElapsedTime(1.0),
                    },
                    _ => {
                        return;
                    }
                };
                self.last_skill_pos = effect_comp.pos.clone();
                updater.insert(self.effect_id, effect_comp);
                self.action_count += 1;
                self.next_action_at = system_vars.time.add_seconds(1.5);
                self.next_damage_at = system_vars.time.add_seconds(1.0);
            }
            if self.next_damage_at.has_passed(system_vars.time) {
                system_vars.area_attacks.push(AreaAttackComponent {
                    area_shape: Box::new(ncollide2d::shape::Ball::new(1.0)),
                    area_isom: Isometry2::new(self.last_skill_pos, 0.0),
                    source_entity_id: self.caster_entity_id,
                    typ: AttackType::SpellDamage(120),
                });
                self.next_damage_at = self.next_damage_at.add_seconds(0.6);
            }
        }
    }

    fn render(&self, system_vars: &SystemVariables, view_matrix: &Matrix4<f32>) {
        for i in self.action_count..3 {
            draw_circle_inefficiently(
                &system_vars.shaders.trimesh_shader,
                &system_vars.matrices.projection,
                view_matrix,
                &(self.pos + self.dir_vector * i as f32 * 2.2),
                0.0,
                1.0,
                &[0.0, 1.0, 0.0, 1.0],
            );
        }
        // backwards
        if self.action_count >= 4 {
            for i in self.action_count..6 {
                draw_circle_inefficiently(
                    &system_vars.shaders.trimesh_shader,
                    &system_vars.matrices.projection,
                    view_matrix,
                    &(self.pos + self.dir_vector * (5 - i) as f32 * 2.2),
                    0.0,
                    1.0,
                    &[0.0, 1.0, 0.0, 1.0],
                );
            }
        }
    }
}
