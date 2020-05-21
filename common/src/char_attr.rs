use crate::common::{percentage, GameTime, Local, Percentage};
use crate::components::char::JobId;
use crate::config::{CommonConfigs, DevConfigStats};
use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct CharAttributes {
    pub max_hp: i32,
    pub attack_damage: u16,
    pub movement_speed: Percentage,
    pub attack_range: Percentage,
    pub attack_speed: Percentage,
    pub armor: Percentage,
    pub healing: Percentage,
    pub hp_regen: Percentage,
    pub mana_regen: Percentage,
}

#[derive(Clone, Debug)]
pub struct CharAttributesBonuses {
    pub attrs: CharAttributes,
    pub durations: BonusDurations,
}

impl CharAttributes {
    pub const TARGET_DUMMY_ATTRIBUTES: CharAttributes = CharAttributes {
        movement_speed: percentage(0),
        attack_range: percentage(0),
        attack_speed: percentage(0),
        attack_damage: 0,
        armor: percentage(0),
        healing: percentage(100),
        hp_regen: percentage(0),
        max_hp: 1_000_000,
        mana_regen: percentage(0),
    };

    pub const HEALING_DUMMY_ATTRIBUTES: CharAttributes = CharAttributes {
        movement_speed: percentage(0),
        attack_range: percentage(0),
        attack_speed: percentage(0),
        attack_damage: 0,
        armor: percentage(0),
        healing: percentage(100),
        hp_regen: percentage(0),
        max_hp: 1_000_000,
        mana_regen: percentage(0),
    };

    pub const BARRICADE_ATTRIBUTES: CharAttributes = CharAttributes {
        movement_speed: percentage(0),
        attack_range: percentage(0),
        attack_speed: percentage(0),
        attack_damage: 0,
        armor: percentage(0),
        healing: percentage(100),
        hp_regen: percentage(0),
        max_hp: 1_000_000,
        mana_regen: percentage(0),
    };

    pub const OTHER_ATTRIBUTES: CharAttributes = CharAttributes {
        movement_speed: percentage(100),
        attack_range: percentage(100),
        attack_speed: percentage(100),
        attack_damage: 76,
        armor: percentage(10),
        healing: percentage(100),
        hp_regen: percentage(100),
        max_hp: 2000,
        mana_regen: percentage(100),
    };

    pub fn get_base_attributes(job_id: JobId, configs: &CommonConfigs) -> &CharAttributes {
        return match job_id {
            JobId::CRUSADER => &configs.stats.player.crusader.attributes,
            JobId::GUNSLINGER => &configs.stats.player.gunslinger.attributes,
            JobId::RANGER => &configs.stats.player.hunter.attributes,
            JobId::RangedMinion => &configs.stats.minion.ranged,
            JobId::HealingDummy => &CharAttributes::HEALING_DUMMY_ATTRIBUTES,
            JobId::TargetDummy => &CharAttributes::TARGET_DUMMY_ATTRIBUTES,
            JobId::MeleeMinion => &configs.stats.minion.melee,
            JobId::Turret => &configs.skills.gaz_turret.turret,
            JobId::Barricade => &configs.skills.gaz_barricade.char_attrs,
            _ => &CharAttributes::OTHER_ATTRIBUTES,
        };
    }

    pub fn zero() -> CharAttributes {
        CharAttributes {
            movement_speed: percentage(0),
            attack_range: percentage(0),
            attack_speed: percentage(0),
            attack_damage: 0,
            armor: percentage(0),
            healing: percentage(0),
            hp_regen: percentage(0),
            max_hp: 0,
            mana_regen: percentage(0),
        }
    }

    pub fn differences(
        &self,
        other: &CharAttributes,
        collector: &CharAttributeModifierCollector,
    ) -> CharAttributesBonuses {
        return CharAttributesBonuses {
            attrs: CharAttributes {
                max_hp: self.max_hp - other.max_hp,
                attack_damage: self.attack_damage - other.attack_damage,
                movement_speed: self.movement_speed.subtract(other.movement_speed),
                attack_range: self.attack_range.subtract(other.attack_range),
                attack_speed: self.attack_speed.subtract(other.attack_speed),
                armor: (self.armor).subtract(other.armor),
                healing: self.healing.subtract(other.healing),
                hp_regen: self.hp_regen.subtract(other.hp_regen),
                mana_regen: self.mana_regen.subtract(other.mana_regen),
            },
            durations: collector.durations.clone(),
        };
    }

    pub fn apply(&self, modifiers: &CharAttributeModifierCollector) -> CharAttributes {
        let mut attr = self.clone();
        for m in &modifiers.max_hp {
            match m {
                CharAttributeModifier::AddPercentage(_p) => {
                    panic!("max_hp += {:?}, you cannot add percentage to a value", m)
                }
                CharAttributeModifier::AddValue(v) => {
                    attr.max_hp += *v as i32;
                }
                CharAttributeModifier::IncreaseByPercentage(p) => {
                    attr.max_hp = p.add_me_to(attr.max_hp);
                }
            }
        }
        for m in &modifiers.attack_damage {
            match m {
                CharAttributeModifier::AddPercentage(_p) => panic!(
                    "attack_damage += {:?}, you cannot add percentage to a value",
                    m
                ),
                CharAttributeModifier::AddValue(v) => {
                    attr.attack_damage += *v as u16;
                }
                CharAttributeModifier::IncreaseByPercentage(p) => {
                    attr.attack_damage = p.add_me_to(attr.attack_damage as i32) as u16;
                }
            }
        }

        for m in &modifiers.movement_speed {
            attr.movement_speed.apply(m);
        }
        for m in &modifiers.attack_range {
            attr.attack_range.apply(m);
        }
        for m in &modifiers.attack_speed {
            attr.attack_speed.apply(m);
        }
        attr.attack_speed.limit(percentage(-300), percentage(500));
        for m in &modifiers.armor {
            attr.armor.apply(m);
        }
        attr.armor.limit(percentage(-100), percentage(100));
        for m in &modifiers.healing {
            attr.healing.apply(m);
        }
        for m in &modifiers.hp_regen {
            attr.hp_regen.apply(m);
        }
        for m in &modifiers.mana_regen {
            attr.mana_regen.apply(m);
        }
        return attr;
    }
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum CharAttributeModifier {
    AddPercentage(Percentage),
    AddValue(f32),
    IncreaseByPercentage(Percentage),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BonusDurations {
    pub max_hp_bonus_ends_at: GameTime<Local>,
    pub walking_speed_bonus_ends_at: GameTime<Local>,
    pub attack_range_bonus_ends_at: GameTime<Local>,
    pub attack_speed_bonus_ends_at: GameTime<Local>,
    pub attack_damage_bonus_ends_at: GameTime<Local>,
    pub armor_bonus_ends_at: GameTime<Local>,
    pub healing_bonus_ends_at: GameTime<Local>,
    pub hp_regen_bonus_ends_at: GameTime<Local>,
    pub mana_regen_bonus_ends_at: GameTime<Local>,

    pub max_hp_bonus_started_at: GameTime<Local>,
    pub walking_speed_bonus_started_at: GameTime<Local>,
    pub attack_range_bonus_started_at: GameTime<Local>,
    pub attack_speed_bonus_started_at: GameTime<Local>,
    pub attack_damage_bonus_started_at: GameTime<Local>,
    pub armor_bonus_started_at: GameTime<Local>,
    pub healing_bonus_started_at: GameTime<Local>,
    pub hp_regen_bonus_started_at: GameTime<Local>,
    pub mana_regen_bonus_started_at: GameTime<Local>,
}

impl BonusDurations {
    pub fn with_invalid_times() -> BonusDurations {
        BonusDurations {
            max_hp_bonus_ends_at: GameTime::from(std::f32::MAX),
            walking_speed_bonus_ends_at: GameTime::from(std::f32::MAX),
            attack_range_bonus_ends_at: GameTime::from(std::f32::MAX),
            attack_speed_bonus_ends_at: GameTime::from(std::f32::MAX),
            attack_damage_bonus_ends_at: GameTime::from(std::f32::MAX),
            armor_bonus_ends_at: GameTime::from(std::f32::MAX),
            healing_bonus_ends_at: GameTime::from(std::f32::MAX),
            hp_regen_bonus_ends_at: GameTime::from(std::f32::MAX),
            mana_regen_bonus_ends_at: GameTime::from(std::f32::MAX),

            max_hp_bonus_started_at: GameTime::from(std::f32::MAX),
            walking_speed_bonus_started_at: GameTime::from(std::f32::MAX),
            attack_range_bonus_started_at: GameTime::from(std::f32::MAX),
            attack_speed_bonus_started_at: GameTime::from(std::f32::MAX),
            attack_damage_bonus_started_at: GameTime::from(std::f32::MAX),
            armor_bonus_started_at: GameTime::from(std::f32::MAX),
            healing_bonus_started_at: GameTime::from(std::f32::MAX),
            hp_regen_bonus_started_at: GameTime::from(std::f32::MAX),
            mana_regen_bonus_started_at: GameTime::from(std::f32::MAX),
        }
    }
}

#[derive(Clone, Debug)]
pub struct CharAttributeModifierCollector {
    max_hp: Vec<CharAttributeModifier>,
    movement_speed: Vec<CharAttributeModifier>,
    attack_range: Vec<CharAttributeModifier>,
    attack_speed: Vec<CharAttributeModifier>,
    attack_damage: Vec<CharAttributeModifier>,
    armor: Vec<CharAttributeModifier>,
    healing: Vec<CharAttributeModifier>,
    hp_regen: Vec<CharAttributeModifier>,
    mana_regen: Vec<CharAttributeModifier>,
    durations: BonusDurations,
}

impl CharAttributeModifierCollector {
    pub fn new() -> CharAttributeModifierCollector {
        CharAttributeModifierCollector {
            max_hp: Vec::with_capacity(8),
            movement_speed: Vec::with_capacity(8),
            attack_range: Vec::with_capacity(8),
            attack_speed: Vec::with_capacity(8),
            attack_damage: Vec::with_capacity(8),
            armor: Vec::with_capacity(8),
            healing: Vec::with_capacity(8),
            hp_regen: Vec::with_capacity(8),
            mana_regen: Vec::with_capacity(8),
            durations: BonusDurations::with_invalid_times(),
        }
    }

    pub fn change_attack_damage(
        &mut self,
        modifier: CharAttributeModifier,
        started: GameTime<Local>,
        until: GameTime<Local>,
    ) {
        CharAttributeModifierCollector::set_durations(
            started,
            until,
            &mut self.durations.attack_damage_bonus_started_at,
            &mut self.durations.attack_damage_bonus_ends_at,
        );
        self.attack_damage.push(modifier);
    }

    pub fn change_attack_speed(
        &mut self,
        modifier: CharAttributeModifier,
        started: GameTime<Local>,
        until: GameTime<Local>,
    ) {
        CharAttributeModifierCollector::set_durations(
            started,
            until,
            &mut self.durations.attack_speed_bonus_started_at,
            &mut self.durations.attack_speed_bonus_ends_at,
        );
        self.attack_speed.push(modifier);
    }

    pub fn change_armor(
        &mut self,
        modifier: CharAttributeModifier,
        started: GameTime<Local>,
        until: GameTime<Local>,
    ) {
        CharAttributeModifierCollector::set_durations(
            started,
            until,
            &mut self.durations.armor_bonus_started_at,
            &mut self.durations.armor_bonus_ends_at,
        );
        self.armor.push(modifier);
    }

    fn set_durations(
        new_started_at: GameTime<Local>,
        new_ends_at: GameTime<Local>,
        current_started_at: &mut GameTime<Local>,
        current_ends_at: &mut GameTime<Local>,
    ) {
        if current_ends_at.has_not_passed_yet(new_ends_at) {
            *current_ends_at = new_ends_at;
            *current_started_at = new_started_at;
        }
    }

    pub fn change_attack_range(
        &mut self,
        modifier: CharAttributeModifier,
        started: GameTime<Local>,
        until: GameTime<Local>,
    ) {
        CharAttributeModifierCollector::set_durations(
            started,
            until,
            &mut self.durations.attack_range_bonus_started_at,
            &mut self.durations.attack_range_bonus_ends_at,
        );
        self.attack_range.push(modifier);
    }

    pub fn change_walking_speed(
        &mut self,
        modifier: CharAttributeModifier,
        started: GameTime<Local>,
        until: GameTime<Local>,
    ) {
        CharAttributeModifierCollector::set_durations(
            started,
            until,
            &mut self.durations.walking_speed_bonus_started_at,
            &mut self.durations.walking_speed_bonus_ends_at,
        );
        self.movement_speed.push(modifier);
    }

    pub fn clear(&mut self) {
        self.max_hp.clear();
        self.movement_speed.clear();
        self.attack_range.clear();
        self.attack_speed.clear();
        self.attack_damage.clear();
        self.armor.clear();
        self.healing.clear();
        self.hp_regen.clear();
        self.mana_regen.clear();
        self.durations = BonusDurations::with_invalid_times();
    }
}
