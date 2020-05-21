use crate::components::char::{percentage, CharState, Team};
use crate::components::status::attack_heal_status::AttackHealStatus;
use crate::components::status::reflect_damage_status::ReflectDamageStatus;
use crate::components::status::sacrafice_status::SacrificeStatus;
use crate::components::status::status::{ApplyStatusComponent, StatusEnum};
use crate::tests::setup_ecs_world;
use rustarok_common::common::{v2, GameTime};
use rustarok_common::components::char::EntityTarget;
use std::time::Duration;

#[test]
fn basic_attack() {
    let mut test_util = setup_ecs_world();

    let attacker_id = test_util.create_char(v2(10.0, 10.0), Team::Left);
    let attacked_id = test_util.create_char(v2(10.0, 10.0), Team::Right);
    test_util.set_char_target(attacker_id, EntityTarget::OtherEntity(attacked_id));

    test_util.run_for(Duration::from_secs_f32(0.5));

    // clicks away to stop attacking
    test_util.set_char_target(attacker_id, EntityTarget::Pos(v2(20.0, 10.0)));

    test_util.run_for(Duration::from_secs_f32(0.5));

    test_util
        .assert_on_character(attacked_id)
        .has_less_than_max_hp()
        .state(CharState::Idle);

    test_util
        .assert_on_character(attacker_id)
        .has_max_hp()
        .state(CharState::Walking(v2(20.0, 10.0)));
}

#[test]
fn reflection() {
    let mut test_util = setup_ecs_world();

    let attacker_id = test_util.create_char(v2(10.0, 10.0), Team::Left);
    let attacked_id = test_util.create_char(v2(10.0, 10.0), Team::Right);
    test_util.apply_status(ApplyStatusComponent::from_status(
        attacked_id,
        attacked_id,
        StatusEnum::ReflectDamageStatus(ReflectDamageStatus::new(
            attacked_id,
            percentage(10),
            GameTime::from(0.0),
            10.0,
        )),
    ));
    test_util.set_char_target(attacker_id, EntityTarget::OtherEntity(attacked_id));

    test_util.run_for(Duration::from_secs_f32(0.5));

    // clicks away to stop attacking
    test_util.set_char_target(attacker_id, EntityTarget::Pos(v2(20.0, 10.0)));

    test_util.run_for(Duration::from_secs_f32(0.5));

    test_util
        .assert_on_character(attacked_id)
        .has_less_than_max_hp()
        .state(CharState::Idle);

    test_util
        .assert_on_character(attacker_id)
        .has_less_than_max_hp()
        .state(CharState::Walking(v2(20.0, 10.0)));

    // 10% of the damage is reflected back
    test_util
        .assert_events_in_order()
        .basic_damage_eq(attacked_id, attacker_id, 12);
}

#[test]
fn vampiric_attack() {
    let mut test_util = setup_ecs_world();

    let attacker_id = test_util.create_char(v2(10.0, 10.0), Team::Left);
    let attacked_id = test_util.create_char(v2(10.0, 10.0), Team::Right);
    test_util.apply_status(ApplyStatusComponent::from_status(
        attacker_id,
        attacker_id,
        StatusEnum::AttackHealStatus(AttackHealStatus::new(
            percentage(10),
            GameTime::from(0.0),
            10.0,
        )),
    ));
    test_util.set_char_target(attacker_id, EntityTarget::OtherEntity(attacked_id));

    test_util.run_for(Duration::from_secs_f32(0.5));

    // clicks away to stop attacking
    test_util.set_char_target(attacker_id, EntityTarget::Pos(v2(20.0, 10.0)));

    test_util.run_for(Duration::from_secs_f32(0.5));

    test_util
        .assert_on_character(attacked_id)
        .has_less_than_max_hp()
        .state(CharState::Idle);

    test_util
        .assert_on_character(attacker_id)
        .has_max_hp()
        .state(CharState::Walking(v2(20.0, 10.0)));

    // 10% of the damage is healed on the attacked
    test_util
        .assert_events_in_order()
        .basic_damage_eq(attacker_id, attacked_id, 120)
        .heal_eq(attacker_id, attacker_id, 12);
}

#[test]
fn sacrifice() {
    let mut test_util = setup_ecs_world();

    let attacker_id = test_util.create_char(v2(10.0, 10.0), Team::Left);
    let attacked_id = test_util.create_char(v2(10.0, 10.0), Team::Right);
    let sacrifice_id = test_util.create_char(v2(10.0, 10.0), Team::Right);
    test_util.apply_status(ApplyStatusComponent::from_status(
        attacked_id,
        attacked_id,
        StatusEnum::SacrificeStatus(SacrificeStatus::new(
            sacrifice_id,
            percentage(10),
            GameTime::from(0.0),
            10.0,
        )),
    ));
    test_util.set_char_target(attacker_id, EntityTarget::OtherEntity(attacked_id));

    test_util.run_for(Duration::from_secs_f32(0.5));

    // clicks away to stop attacking
    test_util.set_char_target(attacker_id, EntityTarget::Pos(v2(20.0, 10.0)));

    test_util.run_for(Duration::from_secs_f32(0.5));

    test_util
        .assert_on_character(attacked_id)
        .has_less_than_max_hp()
        .state(CharState::Idle);

    test_util
        .assert_on_character(sacrifice_id)
        .has_less_than_max_hp()
        .state(CharState::Idle);

    // 10% of the damage is redirected to the 'sacrifice' char
    test_util
        .assert_events_in_order()
        .basic_damage_eq(
            attacker_id,
            attacked_id,
            percentage(10).subtract_me_from(120) as u32,
        )
        .basic_damage_eq(attacker_id, sacrifice_id, percentage(10).of(120) as u32);
}

#[test]
fn sacrifice_100_percent() {
    let mut test_util = setup_ecs_world();

    let attacker_id = test_util.create_char(v2(10.0, 10.0), Team::Left);
    let attacked_id = test_util.create_char(v2(10.0, 10.0), Team::Right);
    let sacrifice_id = test_util.create_char(v2(10.0, 10.0), Team::Right);
    test_util.apply_status(ApplyStatusComponent::from_status(
        attacked_id,
        attacked_id,
        StatusEnum::SacrificeStatus(SacrificeStatus::new(
            sacrifice_id,
            percentage(100),
            GameTime::from(0.0),
            10.0,
        )),
    ));
    test_util.set_char_target(attacker_id, EntityTarget::OtherEntity(attacked_id));

    test_util.run_for(Duration::from_secs_f32(0.5));

    // clicks away to stop attacking
    test_util.set_char_target(attacker_id, EntityTarget::Pos(v2(20.0, 10.0)));

    test_util.run_for(Duration::from_secs_f32(0.5));

    test_util
        .assert_on_character(attacked_id)
        .has_max_hp()
        .state(CharState::Idle);

    test_util
        .assert_on_character(sacrifice_id)
        .has_less_than_max_hp()
        .state(CharState::Idle);

    // 10% of the damage is redirected to the 'sacrifice' char
    test_util
        .assert_events_in_order()
        .basic_damage_eq(attacker_id, attacked_id, 0)
        .basic_damage_eq(attacker_id, sacrifice_id, 120);
}
