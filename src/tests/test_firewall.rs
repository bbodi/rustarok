use crate::common::{v2, ElapsedTime};
use crate::components::char::{percentage, CharState, EntityTarget, Team};
use crate::components::skills::skills::Skills;
use crate::components::status::attack_heal_status::AttackHealStatus;
use crate::components::status::reflect_damage_status::ReflectDamageStatus;
use crate::components::status::sacrafice_status::SacrificeStatus;
use crate::components::status::status::ApplyStatusComponent;
use crate::tests::setup_ecs_world;
use std::time::Duration;

#[test]
fn enemy_firewall_damages() {
    let mut test_util = setup_ecs_world();

    let char_id = test_util.create_char(v2(13.0, 10.0), Team::Left);
    let enemy_id = test_util.create_char(v2(10.0, 10.0), Team::Right);
    test_util.run_frames_n_times(1);

    test_util.cast_skill_on_pos(char_id, Skills::FireWall, v2(10.0, 10.0));

    test_util.run_for(Duration::from_secs(3));

    test_util
        .assert_on_character(enemy_id)
        .has_less_than_max_hp()
        .state(CharState::Idle);

    test_util
        .assert_events_in_order()
        .state_went_into_casting(char_id)
        .spell_damage(char_id, enemy_id);
}

#[test]
fn own_firewall_does_not_harm() {
    let mut test_util = setup_ecs_world();

    let char_id = test_util.create_char(v2(10.0, 10.0), Team::Right);
    test_util.cast_skill_on_pos(char_id, Skills::FireWall, v2(10.0, 10.0));

    // it needs one frame for setting its state to Walking
    test_util.run_frames_n_times(1);

    // it is still walking but was not blocked
    test_util
        .assert_on_character(char_id)
        .has_max_hp()
        .state(CharState::Idle);

    test_util
        .assert_events_in_order()
        .state_went_into_casting(char_id)
        .no_other_events();
}

#[test]
fn character_cant_move_after_touched_firewall() {}
