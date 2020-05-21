use crate::components::status::status::{StatusUpdateParams, StatusUpdateResult};
use crate::GameTime;
use rustarok_common::char_attr::{CharAttributeModifier, CharAttributeModifierCollector};
use rustarok_common::common::{Local, Percentage};

#[derive(Clone, Debug)]
pub struct ArmorModifierStatus {
    pub started: GameTime<Local>,
    pub until: GameTime<Local>,
    pub modifier: Percentage,
}

impl ArmorModifierStatus {
    pub fn new(now: GameTime<Local>, modifier: Percentage) -> ArmorModifierStatus {
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
    pub started: GameTime<Local>,
    pub until: GameTime<Local>,
    pub modifier: Percentage,
}

impl WalkingSpeedModifierStatus {
    pub fn new(
        now: GameTime<Local>,
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
