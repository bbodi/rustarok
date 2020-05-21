use crate::components::char::ActionPlayMode;
use crate::components::status::status::{StatusUpdateParams, StatusUpdateResult};
use crate::effect::StrEffectType;
use crate::render::render_command::RenderCommandCollector;
use crate::render::render_sys::RenderDesktopClientSystem;
use crate::systems::AssetResources;
use crate::GameTime;
use rustarok_common::attack::{
    DamageDisplayType, HpModificationRequest, HpModificationResult, HpModificationResultType,
    HpModificationType,
};
use rustarok_common::common::{Local, Percentage, Vec2};
use rustarok_common::components::char::EntityId;

#[derive(Clone, Debug)]
pub struct ReflectDamageStatus {
    pub started: GameTime<Local>,
    pub until: GameTime<Local>,
    pub animation_started: GameTime<Local>,
    pub reflected_damage: u32,
    pub reflected_amount: Percentage,
}

// TODO:
#[allow(dead_code)]
impl ReflectDamageStatus {
    pub fn new(
        _self_entity_id: EntityId<Local>,
        reflected_amount: Percentage,
        now: GameTime<Local>,
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

impl ReflectDamageStatus {
    pub fn update(&mut self, params: StatusUpdateParams) -> StatusUpdateResult {
        if self.until.has_already_passed(params.time.now()) {
            StatusUpdateResult::RemoveIt
        } else {
            if self
                .animation_started
                .add_seconds(2.0)
                .has_already_passed(params.time.now())
            {
                self.animation_started = params.time.now().add_seconds(-1.9);
            }
            StatusUpdateResult::KeepIt
        }
    }

    pub fn hp_mod_has_been_applied_on_me(
        &mut self,
        self_id: EntityId<Local>,
        outcome: &HpModificationResult,
        hp_mod_reqs: &mut Vec<HpModificationRequest>,
    ) {
        match outcome.typ {
            HpModificationResultType::Ok(hp_mod_req) => match hp_mod_req {
                HpModificationType::BasicDamage(value, _, weapon_type) => {
                    let reflected_value = self.reflected_amount.of(value as i32) as u32;
                    self.reflected_damage += reflected_value;
                    dbg!(reflected_value);
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

    pub fn render(
        &self,
        char_pos: Vec2,
        now: GameTime<Local>,
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

    pub fn get_status_completion_percent(
        &self,
        now: GameTime<Local>,
    ) -> Option<(GameTime<Local>, f32)> {
        Some((self.until, now.percentage_between(self.started, self.until)))
    }
}
