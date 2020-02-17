use crate::char_attr::CharAttributes;
use crate::common::{v2, EngineTime, LocalTime, Percentage, Vec2};
use crate::components::char::LocalCharEntityId;
use serde::Deserialize;
use serde::Serialize;

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum WeaponType {
    Sword,
    Arrow,
    SilverBullet,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[allow(variant_size_differences)]
pub enum BasicAttackType {
    MeleeSimple,
    #[allow(dead_code)]
    MeleeCombo {
        combo_count: u8,
        base_dmg_percentage_for_each_combo: Percentage,
    },
    Ranged {
        bullet_type: WeaponType,
    },
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum DamageDisplayType {
    SingleNumber,
    Combo(u8),
}

impl BasicAttackType {
    pub fn finish_attack(
        &self,
        calculated_attribs: &CharAttributes,
        caster_entity_id: LocalCharEntityId,
        caster_pos: Vec2,
        target_pos: Vec2,
        target_entity_id: LocalCharEntityId,
        hp_mod_requests: &mut Vec<HpModificationRequest>,
        time: &EngineTime,
    ) -> Option<Box<u32>> {
        match self {
            BasicAttackType::MeleeSimple => {
                hp_mod_requests.push(HpModificationRequest {
                    src_entity: caster_entity_id,
                    dst_entity: target_entity_id,
                    typ: HpModificationType::BasicDamage(
                        calculated_attribs.attack_damage as u32,
                        DamageDisplayType::SingleNumber,
                        WeaponType::Sword,
                    ),
                });
                None
            }
            BasicAttackType::MeleeCombo {
                combo_count,
                base_dmg_percentage_for_each_combo,
            } => {
                let p = base_dmg_percentage_for_each_combo;
                let dmg =
                    (p.of(calculated_attribs.attack_damage as i32) * *combo_count as i32) as u32;
                hp_mod_requests.push(HpModificationRequest {
                    src_entity: caster_entity_id,
                    dst_entity: target_entity_id,
                    typ: HpModificationType::BasicDamage(
                        dmg,
                        DamageDisplayType::Combo(*combo_count),
                        WeaponType::Sword,
                    ),
                });
                None
            }
            // TODO2 ranged
            BasicAttackType::Ranged { bullet_type } => {
                //                Some(Box::new(BasicRangeAttackBullet::new(
                //                    caster_pos,
                //                    caster_entity_id,
                //                    target_entity_id,
                //                    target_pos,
                //                    time.now(),
                //                    *bullet_type,
                //                    time.simulation_frame,
                //                )))
                None
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum HpModificationType {
    BasicDamage(u32, DamageDisplayType, WeaponType),
    SpellDamage(u32, DamageDisplayType),
    Heal(u32),
    Poison(u32),
}

#[derive(Debug)]
pub struct HpModificationRequest {
    pub src_entity: LocalCharEntityId,
    pub dst_entity: LocalCharEntityId,
    pub typ: HpModificationType,
}

impl HpModificationRequest {
    pub fn allow(self, dmg: u32) -> HpModificationResult {
        HpModificationResult {
            src_entity: self.src_entity,
            dst_entity: self.dst_entity,
            typ: HpModificationResultType::Ok(match self.typ {
                HpModificationType::BasicDamage(_, display_type, weapon_type) => {
                    HpModificationType::BasicDamage(dmg, display_type, weapon_type)
                }
                HpModificationType::SpellDamage(_, display_type) => {
                    HpModificationType::SpellDamage(dmg, display_type)
                }
                HpModificationType::Heal(_) => HpModificationType::Heal(dmg),
                HpModificationType::Poison(_) => HpModificationType::Poison(dmg),
            }),
        }
    }

    pub fn blocked(self) -> HpModificationResult {
        HpModificationResult {
            src_entity: self.src_entity,
            dst_entity: self.dst_entity,
            typ: HpModificationResultType::Blocked,
        }
    }
}

#[derive(Debug)]
pub struct HpModificationResult {
    pub src_entity: LocalCharEntityId,
    pub dst_entity: LocalCharEntityId,
    pub typ: HpModificationResultType,
}

impl HpModificationResult {
    pub fn absorbed(self) -> HpModificationResult {
        HpModificationResult {
            src_entity: self.src_entity,
            dst_entity: self.dst_entity,
            typ: HpModificationResultType::Absorbed,
        }
    }
}

#[derive(Debug)]
pub enum HpModificationResultType {
    Ok(HpModificationType),
    Blocked,
    Absorbed,
}

// TODO: be static types for Cuboid area attack components, Circle, etc
pub struct AreaAttackComponent {
    // TODO2
    //    pub area_shape: Box<dyn ncollide2d::shape::Shape<f32>>,
    //    pub area_isom: Isometry2<f32>,
    pub source_entity_id: LocalCharEntityId,
    pub typ: HpModificationType,
    pub except: Option<LocalCharEntityId>,
}

#[derive(Debug)]
pub struct ApplyForceComponent {
    pub src_entity: LocalCharEntityId,
    pub dst_entity: LocalCharEntityId,
    pub force: Vec2,
    pub duration: f32,
}

struct BasicRangeAttackBullet {
    start_pos: Vec2,
    target_pos: Vec2,
    current_pos: Vec2,
    caster_id: LocalCharEntityId,
    target_id: LocalCharEntityId,
    started_at: LocalTime,
    ends_at: LocalTime,
    weapon_type: WeaponType,
    started_tick: u64,
}

impl BasicRangeAttackBullet {
    fn new(
        start_pos: Vec2,
        caster_id: LocalCharEntityId,
        target_id: LocalCharEntityId,
        target_pos: Vec2,
        now: LocalTime,
        bullet_type: WeaponType,
        now_tick: u64,
    ) -> BasicRangeAttackBullet {
        BasicRangeAttackBullet {
            start_pos,
            current_pos: v2(0.0, 0.0),
            target_pos: v2(0.0, 0.0),
            caster_id,
            target_id,
            started_at: now,
            ends_at: now.add_seconds(((target_pos - start_pos).magnitude() * 0.05).min(0.3)),
            weapon_type: bullet_type,
            started_tick: now_tick,
        }
    }
}
