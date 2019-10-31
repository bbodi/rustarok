use crate::components::char::{ActionPlayMode, CharacterStateComponent};
use crate::components::controller::CharEntityId;
use crate::components::status::status::{
    Status, StatusNature, StatusStackingResult, StatusUpdateResult,
};
use crate::components::{
    ApplyForceComponent, HpModificationRequest, HpModificationResult, HpModificationResultType,
    HpModificationType,
};
use crate::effect::StrEffectType;
use crate::runtime_assets::map::PhysicEngine;
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
    fn dupl(&self) -> Box<dyn Status + Send> {
        Box::new(self.clone())
    }

    fn update(
        &mut self,
        self_char_id: CharEntityId,
        _char_state: &mut CharacterStateComponent,
        _physics_world: &mut PhysicEngine,
        sys_vars: &mut SystemVariables,
        _entities: &specs::Entities,
        _updater: &mut LazyUpdate,
    ) -> StatusUpdateResult {
        if self.until.has_already_passed(sys_vars.time) {
            if self.absorbed_damage > 0 {
                sys_vars.hp_mod_requests.push(HpModificationRequest {
                    src_entity: self.caster_entity_id,
                    dst_entity: self_char_id,
                    typ: HpModificationType::Heal(self.absorbed_damage),
                });
            }
            StatusUpdateResult::RemoveIt
        } else {
            if self
                .animation_started
                .add_seconds(2.0)
                .has_already_passed(sys_vars.time)
            {
                self.animation_started = sys_vars.time.add_seconds(-1.9);
            }
            StatusUpdateResult::KeepIt
        }
    }

    fn hp_mod_is_calculated_but_not_applied_yet(
        &mut self,
        outcome: HpModificationResult,
        _hp_mod_reqs: &mut Vec<HpModificationRequest>,
    ) -> HpModificationResult {
        match outcome.typ {
            HpModificationResultType::Ok(hp_mod_req) => match hp_mod_req {
                HpModificationType::BasicDamage(value, _, _)
                | HpModificationType::SpellDamage(value, _)
                | HpModificationType::Poison(value) => {
                    self.absorbed_damage += value;
                    return outcome.absorbed();
                }
                HpModificationType::Heal(_) => return outcome,
            },
            HpModificationResultType::Blocked | HpModificationResultType::Absorbed => {
                return outcome
            }
        }
    }

    fn allow_push(&self, _push: &ApplyForceComponent) -> bool {
        false
    }

    fn render(
        &self,
        char_state: &CharacterStateComponent,
        sys_vars: &SystemVariables,
        render_commands: &mut RenderCommandCollector,
    ) {
        RenderDesktopClientSystem::render_str(
            StrEffectType::Ramadan,
            self.animation_started,
            &char_state.pos(),
            sys_vars,
            render_commands,
            ActionPlayMode::Repeat,
        );
    }

    fn get_status_completion_percent(&self, now: ElapsedTime) -> Option<(ElapsedTime, f32)> {
        Some((self.until, now.percentage_between(self.started, self.until)))
    }

    fn stack(&self, _other: &Box<dyn Status>) -> StatusStackingResult {
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
