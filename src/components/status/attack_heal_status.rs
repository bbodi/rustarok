use crate::common::Vec2;
use crate::components::char::ActionPlayMode;
use crate::components::char::Percentage;
use crate::components::status::status::{StatusUpdateParams, StatusUpdateResult};
use crate::components::{
    HpModificationRequest, HpModificationResult, HpModificationResultType, HpModificationType,
};
use crate::effect::StrEffectType;
use crate::systems::render::render_command::RenderCommandCollector;
use crate::systems::render_sys::RenderDesktopClientSystem;
use crate::systems::{AssetResources, CharEntityId};
use crate::ElapsedTime;

#[derive(Clone, Debug)]
pub struct AttackHealStatus {
    pub started: ElapsedTime,
    pub until: ElapsedTime,
    pub animation_started: ElapsedTime,
    pub healed_amount: u32,
    pub heal: Percentage,
}

// TODO:
#[allow(dead_code)]
impl AttackHealStatus {
    pub fn new(heal: Percentage, now: ElapsedTime, duration: f32) -> AttackHealStatus {
        AttackHealStatus {
            started: now,
            animation_started: now.add_seconds(-1.9),
            until: now.add_seconds(duration),
            healed_amount: 0,
            heal,
        }
    }
}

impl AttackHealStatus {
    pub fn update(&mut self, params: StatusUpdateParams) -> StatusUpdateResult {
        if self.until.has_already_passed(params.sys_vars.time) {
            StatusUpdateResult::RemoveIt
        } else {
            if self
                .animation_started
                .add_seconds(2.0)
                .has_already_passed(params.sys_vars.time)
            {
                self.animation_started = params.sys_vars.time.add_seconds(-1.9);
            }
            StatusUpdateResult::KeepIt
        }
    }

    pub fn hp_mod_has_been_applied_on_enemy(
        &mut self,
        self_id: CharEntityId,
        outcome: &HpModificationResult,
        hp_mod_reqs: &mut Vec<HpModificationRequest>,
    ) {
        match outcome.typ {
            HpModificationResultType::Ok(hp_mod_req) => match hp_mod_req {
                HpModificationType::BasicDamage(value, _, _weapon_type) => {
                    let healed_amount = self.heal.of(value as i32) as u32;
                    self.healed_amount += healed_amount;
                    hp_mod_reqs.push(HpModificationRequest {
                        src_entity: self_id,
                        dst_entity: self_id,
                        typ: HpModificationType::Heal(healed_amount),
                    })
                }
                _ => {}
            },
            _ => {}
        }
    }

    pub fn render(
        &self,
        char_pos: Vec2,
        now: ElapsedTime,
        assets: &AssetResources,
        render_commands: &mut RenderCommandCollector,
    ) {
        RenderDesktopClientSystem::render_str(
            StrEffectType::Ramadan,
            self.animation_started,
            &char_pos,
            assets,
            now,
            render_commands,
            ActionPlayMode::Repeat,
        );
    }

    pub fn get_status_completion_percent(&self, now: ElapsedTime) -> Option<(ElapsedTime, f32)> {
        Some((self.until, now.percentage_between(self.started, self.until)))
    }
}
