use crate::components::char::CharacterStateComponent;
use crate::components::controller::CharEntityId;
use crate::components::status::status::{
    Status, StatusNature, StatusStackingResult, StatusUpdateResult,
};
use crate::components::{ApplyForceComponent, AttackComponent, AttackType};
use crate::effect::StrEffectType;
use crate::runtime_assets::map::PhysicEngine;
use crate::systems::atk_calc::AttackOutcome;
use crate::systems::render::render_command::RenderCommandCollector;
use crate::systems::render_sys::RenderDesktopClientSystem;
use crate::systems::SystemVariables;
use crate::ElapsedTime;
use specs::LazyUpdate;

#[derive(Clone)]
pub struct AbsorbStatus {
    pub caster_entity_id: CharEntityId,
    pub started: ElapsedTime,
    pub animation_started: ElapsedTime,
    pub until: ElapsedTime,
    pub absorbed_damage: u32,
}

impl AbsorbStatus {
    pub fn new(caster_entity_id: CharEntityId, now: ElapsedTime, duration: f32) -> AbsorbStatus {
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

    fn get_render_size(&self) -> f32 {
        1.0
    }

    fn update(
        &mut self,
        self_char_id: CharEntityId,
        _char_state: &CharacterStateComponent,
        _physics_world: &mut PhysicEngine,
        system_vars: &mut SystemVariables,
        _entities: &specs::Entities,
        _updater: &mut specs::Write<LazyUpdate>,
    ) -> StatusUpdateResult {
        if self.until.has_already_passed(system_vars.time) {
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
                .has_already_passed(system_vars.time)
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

    fn allow_push(&self, _push: &ApplyForceComponent) -> bool {
        false
    }

    fn render(
        &self,
        char_state: &CharacterStateComponent,
        system_vars: &SystemVariables,
        render_commands: &mut RenderCommandCollector,
    ) {
        RenderDesktopClientSystem::render_str(
            StrEffectType::Ramadan,
            self.animation_started,
            &char_state.pos(),
            system_vars,
            render_commands,
        );
    }

    fn get_status_completion_percent(&self, now: ElapsedTime) -> Option<(ElapsedTime, f32)> {
        Some((self.until, now.percentage_between(self.started, self.until)))
    }

    fn stack(&self, _other: Box<dyn Status>) -> StatusStackingResult {
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

    fn typ(&self) -> StatusNature {
        StatusNature::Supportive
    }
}
