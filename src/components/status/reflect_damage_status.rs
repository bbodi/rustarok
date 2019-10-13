use crate::components::char::Percentage;
use crate::components::char::{ActionPlayMode, CharacterStateComponent};
use crate::components::controller::CharEntityId;
use crate::components::status::status::{
    Status, StatusNature, StatusStackingResult, StatusUpdateResult,
};
use crate::components::{
    DamageDisplayType, HpModificationRequest, HpModificationResult, HpModificationResultType,
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
pub struct ReflectDamageStatus {
    pub started: ElapsedTime,
    pub until: ElapsedTime,
    pub animation_started: ElapsedTime,
    pub reflected_damage: u32,
    pub reflected_amount: Percentage,
}

impl ReflectDamageStatus {
    pub fn new(
        self_entity_id: CharEntityId,
        reflected_amount: Percentage,
        now: ElapsedTime,
        duration: f32,
    ) -> ReflectDamageStatus {
        ReflectDamageStatus {
            started: now,
            animation_started: now.add_seconds(-1.9),
            until: now.add_seconds(duration),
            reflected_damage: 0,
            reflected_amount,
        }
    }
}

impl Status for ReflectDamageStatus {
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

    fn hp_mod_has_been_applied_on_enemy(
        &mut self,
        self_id: CharEntityId,
        outcome: &HpModificationResult,
        hp_mod_reqs: &mut Vec<HpModificationRequest>,
    ) {
        match outcome.typ {
            HpModificationResultType::Ok(hp_mod_req) => match hp_mod_req {
                HpModificationType::BasicDamage(value, _, weapon_type) => {
                    let reflected_value = self.reflected_amount.of(value as i32) as u32;
                    self.reflected_damage += reflected_value;
                    hp_mod_reqs.push(HpModificationRequest {
                        src_entity: self_id,
                        dst_entity: outcome.src_entity,
                        typ: HpModificationType::BasicDamage(
                            reflected_value,
                            DamageDisplayType::SingleNumber,
                            weapon_type,
                        ),
                    })
                }
                _ => {}
            },
            _ => {}
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
