use crate::components::status::{Status, StatusUpdateResult, StatusType};
use crate::components::char::{CharAttributes};
use crate::consts::JobId;
use crate::systems::{Sex, Sprites, SystemVariables};
use crate::asset::SpriteResource;
use specs::{Entity, LazyUpdate};
use crate::components::controller::WorldCoords;
use crate::ElapsedTime;
use crate::systems::render::RenderDesktopClientSystem;
use crate::components::skills::skill::Skills;
use crate::components::{AreaAttackComponent, AttackType, StrEffectComponent};
use nalgebra::Isometry2;

pub struct FireBombStatus {
    pub caster_entity_id: Entity,
    pub started: ElapsedTime,
    pub until: ElapsedTime,
}


impl Status for FireBombStatus {
    fn can_target_move(&self) -> bool { true }

    fn typ(&self) -> StatusType { StatusType::Harmful }

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
            system_vars.area_attacks.push(
                AreaAttackComponent {
                    area_shape: Box::new(ncollide2d::shape::Ball::new(2.0)),
                    area_isom: Isometry2::new(*char_pos, 0.0),
                    source_entity_id: self.caster_entity_id,
                    typ: AttackType::Skill(Skills::FireBomb)
                }
            );
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

    fn render(&self, char_pos: &WorldCoords, system_vars: &mut SystemVariables) {
        RenderDesktopClientSystem::render_str("firewall",
                                              self.started,
                                              char_pos,
                                              system_vars);
    }

    fn get_status_completion_percent(&self, now: ElapsedTime) -> Option<f32> {
        Some(now.percentage_between(self.started, self.until))
    }
}