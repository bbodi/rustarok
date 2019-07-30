use crate::asset::SpriteResource;
use crate::components::char::CharAttributeModifierCollector;
use crate::components::controller::WorldCoords;
use crate::components::status::{
    ApplyStatusComponentPayload, ApplyStatusInAreaComponent, Status, StatusType, StatusUpdateResult,
};
use crate::components::{ApplyForceComponent, AreaAttackComponent, AttackType, StrEffectComponent};
use crate::consts::JobId;
use crate::systems::atk_calc::AttackOutcome;
use crate::systems::render::RenderDesktopClientSystem;
use crate::systems::{Sex, Sprites, SystemVariables};
use crate::ElapsedTime;
use nalgebra::{Isometry2, Matrix4};
use specs::{Entity, LazyUpdate};

#[derive(Clone)]
pub struct FireBombStatus {
    pub caster_entity_id: Entity,
    pub started: ElapsedTime,
    pub until: ElapsedTime,
}

impl Status for FireBombStatus {
    fn dupl(&self) -> Box<dyn Status> {
        Box::new(self.clone())
    }

    fn can_target_move(&self) -> bool {
        true
    }

    fn typ(&self) -> StatusType {
        StatusType::Harmful
    }

    fn can_target_cast(&self) -> bool {
        true
    }

    fn get_render_color(&self) -> [f32; 4] {
        [1.0, 1.0, 1.0, 1.0]
    }

    fn get_render_size(&self) -> f32 {
        1.0
    }

    fn calc_attribs(&self, modifiers: &mut CharAttributeModifierCollector) {}

    fn calc_render_sprite<'a>(
        &self,
        job_id: JobId,
        head_index: usize,
        sex: Sex,
        sprites: &'a Sprites,
    ) -> Option<&'a SpriteResource> {
        None
    }

    fn update(
        &mut self,
        self_char_id: Entity,
        char_pos: &WorldCoords,
        system_vars: &mut SystemVariables,
        entities: &specs::Entities,
        updater: &mut specs::Write<LazyUpdate>,
    ) -> StatusUpdateResult {
        if self.until.has_passed(system_vars.time) {
            let area_shape = Box::new(ncollide2d::shape::Ball::new(2.0));
            let area_isom = Isometry2::new(*char_pos, 0.0);
            system_vars.area_attacks.push(AreaAttackComponent {
                area_shape: area_shape.clone(),
                area_isom: area_isom.clone(),
                source_entity_id: self.caster_entity_id,
                typ: AttackType::SpellDamage(200),
            });
            system_vars
                .apply_area_statuses
                .push(ApplyStatusInAreaComponent {
                    source_entity_id: self.caster_entity_id,
                    status: ApplyStatusComponentPayload::from_secondary(Box::new(FireBombStatus {
                        caster_entity_id: self.caster_entity_id,
                        started: system_vars.time,
                        until: system_vars.time.add_seconds(2.0),
                    })),
                    area_shape: area_shape.clone(),
                    area_isom: area_isom.clone(),
                    except: Some(self_char_id),
                });
            let effect_comp = StrEffectComponent {
                effect: "firepillarbomb".to_owned(),
                pos: *char_pos,
                start_time: system_vars.time.add_seconds(-0.5),
                die_at: system_vars.time.add_seconds(1.0),
                duration: ElapsedTime(1.0),
            };
            updater.insert(entities.create(), effect_comp);

            StatusUpdateResult::RemoveIt
        } else {
            StatusUpdateResult::KeepIt
        }
    }

    fn render(
        &self,
        char_pos: &WorldCoords,
        system_vars: &mut SystemVariables,
        view_matrix: &Matrix4<f32>,
    ) {
        RenderDesktopClientSystem::render_str(
            "firewall",
            self.started,
            char_pos,
            system_vars,
            view_matrix,
        );
    }

    fn affect_incoming_damage(&mut self, outcome: AttackOutcome) -> AttackOutcome {
        outcome
    }

    fn allow_push(&mut self, push: &ApplyForceComponent) -> bool {
        true
    }

    fn get_status_completion_percent(&self, now: ElapsedTime) -> Option<(ElapsedTime, f32)> {
        Some((self.until, now.percentage_between(self.started, self.until)))
    }
}
