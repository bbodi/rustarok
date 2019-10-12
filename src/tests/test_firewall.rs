use crate::components::char::{CharState, EntityTarget, Team};
use crate::components::skills::skills::Skills;
use crate::tests::setup_ecs_world;
use nalgebra::Vector2;
use std::time::Duration;

#[test]
fn enemy_firewall_damages() {
    let mut test_util = setup_ecs_world();

    let char_id = test_util.create_char(v2!(13, 10), Team::Left);
    let enemy_id = test_util.create_char(v2!(10, 10), Team::Right);
    test_util.run_frames_n_times(1);

    test_util.cast_skill_on_pos(char_id, Skills::FireWall, v2!(10, 10));

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

    let char_id = test_util.create_char(v2!(10, 10), Team::Right);
    test_util.cast_skill_on_pos(char_id, Skills::FireWall, v2!(10, 10));

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

 reflection: reflecting x% of the incoming damage
sacrafice: A játékos helyett B kapja a sebzést
Heal: Az ütés x%át healelje magán a támadó

#[test]
fn character_cant_move_after_touched_firewall() {}
