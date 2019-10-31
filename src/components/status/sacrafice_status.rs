use crate::components::char::Percentage;
use crate::components::char::{ActionPlayMode, CharacterStateComponent};
use crate::components::controller::CharEntityId;
use crate::components::status::status::{
    Status, StatusNature, StatusStackingResult, StatusUpdateResult,
};
use crate::components::{
    HpModificationRequest, HpModificationResult, HpModificationResultType, HpModificationType,
};
use crate::effect::StrEffectType;
use crate::runtime_assets::map::PhysicEngine;
use crate::systems::render::render_command::RenderCommandCollector;
use crate::systems::render_sys::RenderDesktopClientSystem;
use crate::systems::SystemVariables;
use crate::ElapsedTime;
use specs::LazyUpdate;

#[derive(Clone)]
pub struct SacrificeStatus {
    pub sacrifice_caster_id: CharEntityId,
    pub started: ElapsedTime,
    pub until: ElapsedTime,
    pub animation_started: ElapsedTime,
    pub damaged_amount: u32,
    pub sacrifice: Percentage,
}

// TODO
#[allow(dead_code)]
impl SacrificeStatus {
    pub fn new(
        sacrifice_caster_id: CharEntityId,
        sacrifice: Percentage,
        now: ElapsedTime,
        duration: f32,
    ) -> SacrificeStatus {
        SacrificeStatus {
            sacrifice_caster_id,
            started: now,
            animation_started: now.add_seconds(-1.9),
            until: now.add_seconds(duration),
            damaged_amount: 0,
            sacrifice,
        }
    }
}

impl Status for SacrificeStatus {
    fn dupl(&self) -> Box<dyn Status + Send> {
        Box::new(self.clone())
    }

    fn update(
        &mut self,
        _self_char_id: CharEntityId,
        _char_state: &mut CharacterStateComponent,
        _physics_world: &mut PhysicEngine,
        sys_vars: &mut SystemVariables,
        _entities: &specs::Entities,
        _updater: &mut LazyUpdate,
    ) -> StatusUpdateResult {
        if self.until.has_already_passed(sys_vars.time) {
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
        mut outcome: HpModificationResult,
        hp_mod_reqs: &mut Vec<HpModificationRequest>,
    ) -> HpModificationResult {
        match outcome.typ {
            HpModificationResultType::Ok(hp_mod_req) => match hp_mod_req {
                HpModificationType::SpellDamage(value, display_type) => {
                    let absorbed_value = self.sacrifice.of(value as i32) as u32;
                    self.damaged_amount += absorbed_value;
                    // redirect the damage to the sacrifice caster
                    hp_mod_reqs.push(HpModificationRequest {
                        src_entity: outcome.src_entity,
                        dst_entity: self.sacrifice_caster_id,
                        typ: HpModificationType::SpellDamage(absorbed_value, display_type),
                    });
                    // decrease the damage on the original target
                    outcome.typ = HpModificationResultType::Ok(HpModificationType::SpellDamage(
                        value - absorbed_value,
                        display_type,
                    ));
                    outcome
                }
                HpModificationType::BasicDamage(value, display_type, weapon_typ) => {
                    let absorbed_value = self.sacrifice.of(value as i32) as u32;
                    self.damaged_amount += absorbed_value;
                    // redirect the damage to the sacrifice caster
                    hp_mod_reqs.push(HpModificationRequest {
                        src_entity: outcome.src_entity,
                        dst_entity: self.sacrifice_caster_id,
                        typ: HpModificationType::BasicDamage(
                            absorbed_value,
                            display_type,
                            weapon_typ,
                        ),
                    });
                    // decrease the damage on the original target
                    outcome.typ = HpModificationResultType::Ok(HpModificationType::BasicDamage(
                        value - dbg!(absorbed_value),
                        display_type,
                        weapon_typ,
                    ));
                    outcome
                }
                _ => outcome,
            },
            _ => outcome,
        }
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
        StatusStackingResult::Replace
    }

    fn typ(&self) -> StatusNature {
        StatusNature::Supportive
    }
}
