use crate::components::status::{Status, StatusUpdateResult, StatusType};
use crate::components::char::CharAttributes;
use crate::consts::JobId;
use crate::systems::{Sex, Sprites, SystemVariables};
use crate::asset::SpriteResource;
use specs::{Entity, LazyUpdate};
use crate::components::controller::WorldCoords;
use crate::ElapsedTime;
use crate::systems::render::RenderDesktopClientSystem;
use crate::components::{AttackType, AttackComponent, ApplyForceComponent};
use crate::systems::atk_calc::AttackOutcome;
use nalgebra::Matrix4;

#[derive(Clone)]
pub struct AbsorbStatus {
    pub caster_entity_id: Entity,
    pub started: ElapsedTime,
    pub animation_started: ElapsedTime,
    pub until: ElapsedTime,
    pub absorbed_damage: u32,
}


impl AbsorbStatus {
    pub fn new(caster_entity_id: Entity, now: ElapsedTime) -> AbsorbStatus {
        AbsorbStatus {
            caster_entity_id,
            started: now,
            animation_started: now.add_seconds(-1.7),
            until: now.add_seconds(3.0),
            absorbed_damage: 0,
        }
    }
}

impl Status for AbsorbStatus {
    fn dupl(&self) -> Box<dyn Status> { Box::new(self.clone()) }

    fn can_target_move(&self) -> bool { true }

    fn typ(&self) -> StatusType { StatusType::Supportive }

    fn can_target_cast(&self) -> bool { true }

    fn get_render_color(&self) -> [f32; 4] { [1.0, 1.0, 1.0, 1.0] }

    fn get_render_size(&self) -> f32 { 1.0 }

    fn calc_attribs(&self, attributes: &mut CharAttributes) {}

    fn calc_render_sprite<'a>(
        &self,
        job_id:
        JobId,
        head_index: usize,
        sex: Sex,
        sprites: &'a Sprites,
    ) -> Option<&'a SpriteResource> { None }

    fn update(
        &mut self,
        self_char_id: Entity,
        char_pos: &WorldCoords,
        system_vars: &mut SystemVariables,
        entities: &specs::Entities,
        updater: &mut specs::Write<LazyUpdate>,
    ) -> StatusUpdateResult {
        if self.until.has_passed(system_vars.time) {
            system_vars.attacks.push(
                AttackComponent {
                    src_entity: self.caster_entity_id,
                    dst_entity: self_char_id,
                    typ: AttackType::Heal(self.absorbed_damage),
                }
            );
            StatusUpdateResult::RemoveIt
        } else {
            if self.animation_started.add_seconds(2.0).has_passed(system_vars.time) {
                self.animation_started = system_vars.time.add_seconds(-1.7);
            }
            StatusUpdateResult::KeepIt
        }
    }

    fn render(
        &self,
        char_pos: &WorldCoords,
        system_vars: &mut SystemVariables,
        view_matrix: &Matrix4<f32>) {
        RenderDesktopClientSystem::render_str("ramadan",
                                              self.animation_started,
                                              char_pos,
                                              system_vars,
                                              view_matrix);
    }

    fn affect_incoming_damage(&mut self, outcome: AttackOutcome) -> AttackOutcome {
        match outcome {
            AttackOutcome::Damage(value) | AttackOutcome::Poison(value) |
            AttackOutcome::Crit(value) => {
                self.absorbed_damage += value;
                AttackOutcome::Absorb
            }
            _ => { outcome }
        }
    }

    fn allow_push(&mut self, push: &ApplyForceComponent) -> bool { false }

    fn get_status_completion_percent(&self, now: ElapsedTime) -> Option<f32> {
        Some(now.percentage_between(self.started, self.until))
    }
}