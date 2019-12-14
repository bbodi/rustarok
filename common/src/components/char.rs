use crate::common::Vec2;
use specs::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub enum CharState {
    Idle,
    Walking(Vec2),
    //    StandBy,
    //    Attacking {
    //        target: CharEntityId,
    //        damage_occurs_at: ElapsedTime,
    //        basic_attack: BasicAttackType,
    //    },
    //    ReceivingDamage,
    //    Dead,
    //    CastingSkill(CastingSkillData),
}

unsafe impl Sync for CharState {}

unsafe impl Send for CharState {}

#[derive(Component)]
pub struct AuthorizedCharStateComponent {
    pos: Vec2,
    state: CharState,
}
