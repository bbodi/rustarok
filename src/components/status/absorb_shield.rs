use crate::components::char::CharAttributeModifierCollector;
use crate::components::controller::WorldCoords;
use crate::components::status::status::{
    Status, StatusStackingResult, StatusType, StatusUpdateResult,
};
use crate::components::{ApplyForceComponent, AttackComponent, AttackType};
use crate::systems::atk_calc::AttackOutcome;
use crate::systems::render::render_command::RenderCommandCollectorComponent;
use crate::systems::render_sys::RenderDesktopClientSystem;
use crate::systems::SystemVariables;
use crate::ElapsedTime;
use specs::{Entity, LazyUpdate};

#[derive(Clone)]
pub struct AbsorbStatus {
    pub caster_entity_id: Entity,
    pub started: ElapsedTime,
    pub animation_started: ElapsedTime,
    pub until: ElapsedTime,
    pub absorbed_damage: u32,
}

impl AbsorbStatus {
    pub fn new(caster_entity_id: Entity, now: ElapsedTime, duration: f32) -> AbsorbStatus {
        AbsorbStatus {
            caster_entity_id,
            started: now,
            animation_started: now.add_seconds(-1.9),
            until: now.add_seconds(duration),
            absorbed_damage: 0,
        }
    }
}

impl Status for AbsorbStatus {
    fn dupl(&self) -> Box<dyn Status> {
        Box::new(self.clone())
    }

    fn can_target_move(&self) -> bool {
        true
    }

    fn typ(&self) -> StatusType {
        StatusType::Supportive
    }

    fn can_target_cast(&self) -> bool {
        true
    }

    fn get_render_color(&self, _now: ElapsedTime) -> [u8; 4] {
        [255, 255, 255, 255]
    }

    fn get_render_size(&self) -> f32 {
        1.0
    }

    fn calc_attribs(&self, _modifiers: &mut CharAttributeModifierCollector) {}

    fn update(
        &mut self,
        self_char_id: Entity,
        _char_pos: &WorldCoords,
        system_vars: &mut SystemVariables,
        _entities: &specs::Entities,
        _updater: &mut specs::Write<LazyUpdate>,
    ) -> StatusUpdateResult {
        if self.until.is_earlier_than(system_vars.time) {
            if self.absorbed_damage > 0 {
                system_vars.attacks.push(AttackComponent {
                    src_entity: self.caster_entity_id,
                    dst_entity: self_char_id,
                    typ: AttackType::Heal(self.absorbed_damage),
                });
            }
            StatusUpdateResult::RemoveIt
        } else {
            if self
                .animation_started
                .add_seconds(2.0)
                .is_earlier_than(system_vars.time)
            {
                self.animation_started = system_vars.time.add_seconds(-1.9);
            }
            StatusUpdateResult::KeepIt
        }
    }

    fn affect_incoming_damage(&mut self, outcome: AttackOutcome) -> AttackOutcome {
        match outcome {
            AttackOutcome::Damage(value)
            | AttackOutcome::Poison(value)
            | AttackOutcome::Combo {
                sum_damage: value, ..
            }
            | AttackOutcome::Crit(value) => {
                self.absorbed_damage += value;
                AttackOutcome::Absorb
            }
            AttackOutcome::Heal(_) | AttackOutcome::Block | AttackOutcome::Absorb => outcome,
        }
    }

    fn allow_push(&mut self, _push: &ApplyForceComponent) -> bool {
        false
    }

    fn render(
        &self,
        char_pos: &WorldCoords,
        system_vars: &SystemVariables,
        render_commands: &mut RenderCommandCollectorComponent,
    ) {
        RenderDesktopClientSystem::render_str(
            "ramadan",
            self.animation_started,
            char_pos,
            system_vars,
            render_commands,
        );
    }

    fn get_status_completion_percent(&self, now: ElapsedTime) -> Option<(ElapsedTime, f32)> {
        Some((self.until, now.percentage_between(self.started, self.until)))
    }

    fn stack(&mut self, _other: Box<dyn Status>) -> StatusStackingResult {
        // I think it should be overwritten only when the caster_entity_id is the same
        // otherwise other players should get the healed credits for their armors
        //        let other_absorb = unsafe { Statuses::hack_cast::<AbsorbStatus>(&other) };
        //        if other_absorb.until.is_later_than(self.until) {
        //            self.until = other_absorb.until;
        //            self.started = other_absorb.started;
        //            self.caster_entity_id = other_absorb.caster_entity_id;
        //            self.animation_started = other_absorb.animation_started;
        //        }
        StatusStackingResult::AddTheNewStatus
    }
}
