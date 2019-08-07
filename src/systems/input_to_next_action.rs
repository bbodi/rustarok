use crate::components::char::CharacterStateComponent;
use crate::components::controller::{
    CastMode, ControllerComponent, HumanInputComponent, PlayerIntention, SkillKey,
};
use crate::systems::input_sys::InputConsumerSystem;
use crate::systems::SystemFrameDurations;
use sdl2::keyboard::Scancode;
use specs::prelude::*;
use strum::IntoEnumIterator;

pub struct InputToNextActionSystem;

impl<'a> specs::System<'a> for InputToNextActionSystem {
    type SystemData = (
        specs::Entities<'a>,
        specs::WriteStorage<'a, HumanInputComponent>,
        specs::ReadStorage<'a, CharacterStateComponent>,
        specs::WriteStorage<'a, ControllerComponent>,
        specs::WriteExpect<'a, SystemFrameDurations>,
    );

    fn run(
        &mut self,
        (
            entities,
            mut input_storage,
            char_state_storage,
            mut controller_storage,
            mut system_benchmark,
        ): Self::SystemData,
    ) {
        let _stopwatch = system_benchmark.start_measurement("InputToNextActionSystem");
        for (self_id, self_char_comp, input, controller) in (
            &entities,
            &char_state_storage,
            &mut input_storage,
            &mut controller_storage,
        )
            .join()
        {
            let controller: &mut ControllerComponent = controller;
            let input: &mut HumanInputComponent = input;
            let just_pressed_skill_key = SkillKey::iter()
                .filter(|key| input.is_key_just_pressed(key.scancode()))
                .take(1)
                .collect::<Vec<SkillKey>>()
                .pop();
            let just_released_skill_key = SkillKey::iter()
                .filter(|key| input.is_key_just_released(key.scancode()))
                .take(1)
                .collect::<Vec<SkillKey>>()
                .pop();

            if controller.next_action.is_some() {
                // here 'next_action' is the action from the prev frame
                controller.last_action = std::mem::replace(&mut controller.next_action, None);
            }
            let alt_down = input.is_key_down(Scancode::LAlt);
            controller.next_action = if let Some((casting_skill_key, skill)) =
                input.select_skill_target
            {
                match input.cast_mode {
                    CastMode::Normal => {
                        if input.left_mouse_released {
                            log::debug!("Player wants to cast {:?}", skill);
                            input.select_skill_target = None;
                            Some(PlayerIntention::Casting(
                                skill,
                                false,
                                input.mouse_world_pos,
                                input.entity_below_cursor,
                            ))
                        } else if input.right_mouse_pressed {
                            input.select_skill_target = None;
                            None
                        } else if let Some((skill_key, skill)) =
                            just_pressed_skill_key.and_then(|skill_key| {
                                input
                                    .get_skill_for_key(skill_key)
                                    .map(|skill| (skill_key, skill))
                            })
                        {
                            log::debug!("Player select target for casting {:?}", skill);
                            let shhh = InputConsumerSystem::target_selection_or_casting(
                                skill,
                                input.mouse_world_pos,
                                input.entity_below_cursor,
                            );
                            if let Some(s) = shhh {
                                Some(s)
                            } else {
                                input.select_skill_target = Some((skill_key, skill));
                                None
                            }
                        } else {
                            None
                        }
                    }
                    CastMode::OnKeyRelease => {
                        if input.is_key_just_released(casting_skill_key.scancode()) {
                            log::debug!("Player wants to cast {:?}", skill);
                            input.select_skill_target = None;
                            Some(
                                PlayerIntention::Casting(
                                    input.get_skill_for_key(casting_skill_key)
                                        .expect("'is_casting_selection' must be Some only if the casting skill is valid! "),
                                    false,
                                    input.mouse_world_pos,
                                    input.entity_below_cursor,
                                )
                            )
                        } else if input.right_mouse_pressed {
                            input.select_skill_target = None;
                            None
                        } else {
                            None
                        }
                    }
                    CastMode::OnKeyPress => {
                        // not possible to get into this state while OnKeyPress is active
                        None
                    }
                }
            } else if let Some((skill_key, skill)) = just_pressed_skill_key.and_then(|skill_key| {
                input
                    .get_skill_for_key(skill_key)
                    .map(|skill| (skill_key, skill))
            }) {
                match input.cast_mode {
                    CastMode::Normal | CastMode::OnKeyRelease => {
                        if !alt_down {
                            log::debug!(
                                "Player select target for casting {:?} (just_pressed_skill_key)",
                                skill
                            );
                            let shh = InputConsumerSystem::target_selection_or_casting(
                                skill,
                                input.mouse_world_pos,
                                input.entity_below_cursor,
                            );
                            if let Some(s) = shh {
                                Some(s)
                            } else {
                                input.select_skill_target = Some((skill_key, skill));
                                None
                            }
                        } else {
                            None
                        }
                    }
                    CastMode::OnKeyPress => {
                        log::debug!("Player wants to cast {:?}, alt={:?}", skill, alt_down);
                        input.select_skill_target = None;
                        Some(PlayerIntention::Casting(
                            skill,
                            alt_down,
                            input.mouse_world_pos,
                            input.entity_below_cursor,
                        ))
                    }
                }
            } else if let Some((_skill_key, skill)) =
                just_released_skill_key.and_then(|skill_key| {
                    input
                        .get_skill_for_key(skill_key)
                        .map(|skill| (skill_key, skill))
                })
            {
                // can get here only when alt was down and OnKeyRelease
                if alt_down {
                    log::debug!("Player wants to cast {:?}, SELF", skill);
                    Some(PlayerIntention::Casting(
                        skill,
                        true,
                        input.mouse_world_pos,
                        input.entity_below_cursor,
                    ))
                } else {
                    None
                }
            } else if input.right_mouse_pressed {
                Some(PlayerIntention::MoveTowardsMouse(input.mouse_world_pos))
            } else if input.right_mouse_down {
                Some(PlayerIntention::MoveTowardsMouse(input.mouse_world_pos))
            } else if let Some(PlayerIntention::MoveTowardsMouse(pos)) = &controller.last_action {
                // user released the mouse, so it is not a MoveTowardsMouse but a move/attack to command
                if let Some(target_entity_id) = input.entity_below_cursor {
                    if target_entity_id != self_id
                        && char_state_storage
                            .get(target_entity_id)
                            .map(|it| it.team != self_char_comp.team && !it.state().is_dead())
                            .unwrap_or(false)
                    {
                        Some(PlayerIntention::Attack(target_entity_id))
                    } else {
                        Some(PlayerIntention::MoveTo(input.mouse_world_pos))
                    }
                } else {
                    Some(PlayerIntention::MoveTo((*pos).clone()))
                }
            } else {
                None
            };
        }
    }
}
