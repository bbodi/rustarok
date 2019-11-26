use crate::common::{v2, ElapsedTime};
use crate::components::char::Team;
use crate::components::char::{percentage, CharState, EntityTarget};
use crate::components::skills::skills::Skills;
use crate::components::status::attrib_mod::WalkingSpeedModifierStatus;
use crate::components::status::status::{
    ApplyStatusComponent, StatusEnum, StatusEnumDiscriminants,
};
use crate::tests::setup_ecs_world;
use std::time::Duration;

#[test]
fn finishing_mounting_skill_should_result_in_mounted_state() {
    let mut test_util = setup_ecs_world();

    let char_entity_id = test_util.create_char(v2(10.0, 10.0), Team::Right);
    test_util.cast_skill_on_self(char_entity_id, Skills::Mounting);

    test_util.run_for(Duration::from_secs_f32(0.5));

    test_util.assert_on_character(char_entity_id).is_casting();

    test_util.run_for(Duration::from_secs_f32(0.8));

    test_util
        .assert_on_character(char_entity_id)
        .state(CharState::Idle)
        .movement_speed(percentage(130))
        .has_status(StatusEnumDiscriminants::MountedStatus);
}

#[test]
fn char_should_move_towards_its_target_pos_then_stop() {
    let mut test_util = setup_ecs_world();

    let char_entity_id = test_util.create_char(v2(10.0, 10.0), Team::Right);
    test_util.set_char_target(char_entity_id, EntityTarget::Pos(v2(40.0, 50.0)));

    // it needs one frame for setting its state to Walking
    test_util.run_frames_n_times(1);

    // distance is 50
    // default movement speed is 5 units/second
    // so it takes 10.0 seconds
    test_util.run_for(Duration::from_secs(10));

    // one more frame to go back to Idle from Walk
    test_util.run_frames_n_times(1);

    test_util
        .assert_on_character(char_entity_id)
        .pos(40.0, 50.0)
        .state(CharState::Idle)
        .has_no_active_status()
        .has_max_hp();

    test_util
        .assert_events_in_order()
        .status_change(
            char_entity_id,
            CharState::Idle,
            CharState::Walking(v2(40.0, 50.0)),
        )
        .status_change(
            char_entity_id,
            CharState::Walking(v2(40.0, 50.0)),
            CharState::Idle,
        )
        .no_other_events();
}

#[test]
fn first_char_is_twice_as_fast_as_second() {
    let mut test_util = setup_ecs_world();

    let a_id = test_util.create_char(v2(10.0, 10.0), Team::Right);
    let b_id = test_util.create_char(v2(13.0, 10.0), Team::Right);

    test_util.set_char_target(a_id, EntityTarget::Pos(v2(10.0, 30.0)));
    test_util.set_char_target(b_id, EntityTarget::Pos(v2(13.0, 30.0)));

    test_util.apply_status(ApplyStatusComponent::from_status(
        b_id,
        b_id,
        StatusEnum::MountedStatus {
            speedup: percentage(30),
        },
    ));

    // it needs one frame for setting its state to Walking
    test_util.run_frames_n_times(1);

    // distance is 20
    // default movement speed is 5 units/second
    // so it takes 4 seconds
    test_util.run_for(Duration::from_secs(4));
    // one more frame to go back to Idle from Walk
    test_util.run_frames_n_times(1);

    test_util
        .assert_on_character(a_id)
        .pos(10.0, 30.0)
        .state(CharState::Idle);
    test_util
        .assert_on_character(b_id)
        .pos(13.0, 30.0)
        .state(CharState::Idle);

    test_util
        .assert_events_in_order()
        .status_change(a_id, CharState::Idle, CharState::Walking(v2(10.0, 30.0)))
        .status_change(b_id, CharState::Idle, CharState::Walking(v2(13.0, 30.0)))
        .status_change(b_id, CharState::Walking(v2(13.0, 30.0)), CharState::Idle) // 'b' is faster, it should get there first
        .status_change(a_id, CharState::Walking(v2(10.0, 30.0)), CharState::Idle)
        .no_other_events();
}

#[test]
fn many_chars_with_different_movement_speed() {
    let mut test_util = setup_ecs_world();

    let distance = 100.0;
    let char_ids = (0..100)
        .map(|i| {
            let x = 10 + 3 * i;
            let char_id = test_util.create_char(v2(x as f32, 10.0), Team::Right);
            test_util.set_char_target(char_id, EntityTarget::Pos(v2(x as f32, 10.0 + distance)));
            test_util.apply_status(ApplyStatusComponent::from_status(
                char_id,
                char_id,
                StatusEnum::WalkingSpeedModifierStatus(WalkingSpeedModifierStatus::new(
                    ElapsedTime(0.0),
                    percentage(i),
                    1000.0,
                )),
            ));
            char_id
        })
        .collect::<Vec<_>>();

    // it needs one frame for setting its state to Walking
    test_util.run_frames_n_times(1);

    let travel_time = distance / 5.0;
    test_util.run_for(Duration::from_secs_f32(travel_time));
    // one more frame to go back to Idle from Walk
    test_util.run_frames_n_times(1);

    char_ids.iter().enumerate().for_each(|(i, char_id)| {
        test_util
            .assert_on_character(*char_id)
            .pos(10.0 + (3.0 * i as f32), 10.0 + distance)
            .state(CharState::Idle);
    });

    let mut event_asserter = test_util.assert_events_in_order();
    char_ids.iter().enumerate().for_each(|(i, char_id)| {
        event_asserter.status_change_ref(
            *char_id,
            CharState::Idle,
            CharState::Walking(v2(10.0 + i as f32 * 3.0, 10.0 + distance)),
        );
    });

    // movement speed are increasing, so the last one will get there first
    char_ids.iter().enumerate().rev().for_each(|(i, char_id)| {
        event_asserter.status_change_ref(
            *char_id,
            CharState::Walking(v2(10.0 + i as f32 * 3.0, 10.0 + distance)),
            CharState::Idle,
        );
    });

    event_asserter.no_other_events();
}

#[test]
fn character_cant_go_through_enemy_barricade() {
    let mut test_util = setup_ecs_world();

    //  BBB - barricades
    //   C  - character
    let char_id = test_util.create_char(v2(11.0, 10.0), Team::Right);
    test_util.create_barricade(v2(10.0, 12.0), Team::Left);
    test_util.create_barricade(v2(11.0, 12.0), Team::Left);
    test_util.create_barricade(v2(12.0, 12.0), Team::Left);

    test_util.set_char_target(char_id, EntityTarget::Pos(v2(11.0, 30.0)));

    // it needs one frame for setting its state to Walking
    test_util.run_frames_n_times(1);

    test_util.run_for(Duration::from_secs(2));

    // it is still walking, because blocked by the barricade
    test_util
        .assert_on_character(char_id)
        .pos_y_lesser_than(11.0)
        .state(CharState::Walking(v2(11.0, 30.0)));
}

#[test]
fn character_can_go_through_friendly_barricade() {
    let mut test_util = setup_ecs_world();

    //  BBB - barricades
    //   C  - character
    let char_id = test_util.create_char(v2(11.0, 10.0), Team::Right);
    test_util.create_barricade(v2(10.0, 12.0), Team::Right);
    test_util.create_barricade(v2(11.0, 12.0), Team::Right);
    test_util.create_barricade(v2(12.0, 12.0), Team::Right);

    test_util.set_char_target(char_id, EntityTarget::Pos(v2(11.0, 30.0)));

    // it needs one frame for setting its state to Walking
    test_util.run_frames_n_times(1);

    test_util.run_for(Duration::from_secs(2));

    // it is still walking but was not blocked
    test_util
        .assert_on_character(char_id)
        .pos_y_greater_than(12.0)
        .state(CharState::Walking(v2(11.0, 30.0)));
}
