use crate::components::char::{CharAttributeModifier, CharAttributeModifierCollector, Percentage};
use crate::components::status::status::{StatusUpdateParams, StatusUpdateResult};
use crate::ElapsedTime;

#[derive(Clone, Debug)]
pub struct ArmorModifierStatus {
    pub started: ElapsedTime,
    pub until: ElapsedTime,
    pub modifier: Percentage,
}

impl ArmorModifierStatus {
    pub fn new(now: ElapsedTime, modifier: Percentage) -> ArmorModifierStatus {
        ArmorModifierStatus {
            started: now,
            until: now.add_seconds(10.0),
            modifier,
        }
    }
}

impl ArmorModifierStatus {
    pub fn calc_attribs(&self, modifiers: &mut CharAttributeModifierCollector) {
        modifiers.change_armor(
            CharAttributeModifier::AddPercentage(self.modifier),
            self.started,
            self.until,
        );
    }

    pub fn update(&mut self, params: StatusUpdateParams) -> StatusUpdateResult {
        if self.until.has_already_passed(params.time.now()) {
            StatusUpdateResult::RemoveIt
        } else {
            StatusUpdateResult::KeepIt
        }
    }
}

#[derive(Clone, Debug)]
pub struct WalkingSpeedModifierStatus {
    pub started: ElapsedTime,
    pub until: ElapsedTime,
    pub modifier: Percentage,
}

impl WalkingSpeedModifierStatus {
    pub fn new(
        now: ElapsedTime,
        modifier: Percentage,
        duration: f32,
    ) -> WalkingSpeedModifierStatus {
        WalkingSpeedModifierStatus {
            started: now,
            until: now.add_seconds(duration),
            modifier,
        }
    }
}

impl WalkingSpeedModifierStatus {
    pub fn calc_attribs(&self, modifiers: &mut CharAttributeModifierCollector) {
        modifiers.change_walking_speed(
            CharAttributeModifier::AddPercentage(self.modifier),
            self.started,
            self.until,
        );
    }

    pub fn update(&mut self, params: StatusUpdateParams) -> StatusUpdateResult {
        if self.until.has_already_passed(params.time.now()) {
            StatusUpdateResult::RemoveIt
        } else {
            StatusUpdateResult::KeepIt
        }
    }
}
