use crate::components::char::CharacterStateComponent;
use crate::components::controller::{
    CastMode, HumanInputComponent, LocalPlayerControllerComponent, SkillKey,
};
use crate::components::skills::skills::{SkillTargetType, Skills};
use crate::cursor::{CursorFrame, CURSOR_CLICK, CURSOR_NORMAL, CURSOR_STOP, CURSOR_TARGET};
use crate::runtime_assets::map::MapRenderData;
use crate::systems::input_sys::InputConsumerSystem;
use crate::systems::{SystemFrameDurations, SystemVariables};
use crate::ElapsedTime;
use rustarok_common::common::EngineTime;
use rustarok_common::components::char::{AuthorizedCharStateComponent, Team};
use rustarok_common::components::controller::{ControllerComponent, PlayerIntention};
use sdl2::keyboard::Scancode;
use specs::prelude::*;
use strum::IntoEnumIterator;

// Singleton
pub struct InputToNextActionSystem {
    prev_intention: Option<PlayerIntention>,
    prev_prev_intention: Option<PlayerIntention>,
}

impl InputToNextActionSystem {
    pub fn new() -> InputToNextActionSystem {
        InputToNextActionSystem {
            prev_intention: None,
            prev_prev_intention: None,
        }
    }
}

impl<'a> System<'a> for InputToNextActionSystem {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, HumanInputComponent>,
        ReadStorage<'a, CharacterStateComponent>,
        ReadStorage<'a, AuthorizedCharStateComponent>,
        WriteStorage<'a, ControllerComponent>,
        WriteStorage<'a, LocalPlayerControllerComponent>,
        WriteExpect<'a, SystemFrameDurations>,
        ReadExpect<'a, SystemVariables>,
        ReadExpect<'a, EngineTime>,
        ReadExpect<'a, MapRenderData>,
    );

    fn run(
        &mut self,
        (
            entities,
            input_storage,
            char_state_storage,
            auth_char_state_storage,
            mut controller_storage,
            mut desktop_storage,
            mut system_benchmark,
            sys_vars,
            time,
            map_render_data,
        ): Self::SystemData,
    ) {
        let _stopwatch = system_benchmark.start_measurement("InputToNextActionSystem");
        for (_self_id, input, controller, mut desktop) in (
            &entities,
            &input_storage,
            &mut controller_storage,
            &mut desktop_storage,
        )
            .join()
        {
            // TODO: it can happen that this unwrap panics if browser clients disconnects
            let self_char_team = char_state_storage
                .get(controller.controlled_entity.into())
                .unwrap()
                .team;

            // TODO: optimize it
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

            desktop.calc_entities_below_cursor(
                self_char_team,
                input.last_mouse_x,
                input.last_mouse_y,
            );

            desktop.cell_below_cursor_walkable = map_render_data.gat.is_walkable(
                input.mouse_world_pos.x.max(0.0) as usize,
                input.mouse_world_pos.y.abs() as usize,
            );
            let (cursor_frame, cursor_color) = InputToNextActionSystem::determine_cursor(
                time.now(),
                controller,
                desktop,
                &char_state_storage,
                &auth_char_state_storage,
                self_char_team,
            );
            desktop.cursor_anim_descr.action_index = cursor_frame.1;
            desktop.cursor_color = cursor_color;

            let alt_down = input.alt_down;
            let current_frame_intention = InputToNextActionSystem::determine_intention(
                &auth_char_state_storage,
                input,
                &mut desktop,
                just_pressed_skill_key,
                just_released_skill_key,
                alt_down,
            );

            if time.tick % 3 == 0 {
                controller.intention = match (
                    &self.prev_prev_intention,
                    &self.prev_intention,
                    &current_frame_intention,
                ) {
                    (
                        Some(PlayerIntention::MoveTowardsMouse(_)),
                        Some(PlayerIntention::MoveTowardsMouse(_)),
                        Some(PlayerIntention::MoveTowardsMouse(pos)),
                    ) => Some(PlayerIntention::MoveTowardsMouse(*pos)),
                    _ => current_frame_intention
                        .as_ref()
                        .or(self
                            .prev_intention
                            .as_ref()
                            .or(self.prev_prev_intention.as_ref()))
                        .map(|it| it.clone()),
                }
            } else {
                controller.intention = None;
            }
            if controller.intention.is_some() {
                // here 'intention' is the action from the prev frame
                desktop.last_intention = controller.intention.clone();
            }
            self.prev_prev_intention =
                std::mem::replace(&mut self.prev_intention, current_frame_intention);

            // in console mode, only moving around is allowed
            if input.is_console_open {
                if let Some(next_action) = &controller.intention {
                    match next_action {
                        PlayerIntention::MoveTo(_) => {}
                        PlayerIntention::MoveTowardsMouse(_) => {}
                        PlayerIntention::Attack(_) => {}
                        PlayerIntention::AttackTowards(_) => {} // TODO2
                                                                //                        PlayerIntention::Casting(_, _, _) => {
                                                                //                            log::debug!("...but the console is open");
                                                                //                            controller.next_action = None;
                                                                //                        }
                    }
                }
            }
        }
    }
}

impl InputToNextActionSystem {
    pub fn determine_cursor(
        now: ElapsedTime,
        controller: &ControllerComponent,
        desktop: &LocalPlayerControllerComponent,
        char_state_storage: &ReadStorage<CharacterStateComponent>,
        auth_char_state_storage: &ReadStorage<AuthorizedCharStateComponent>,
        self_team: Team,
    ) -> (CursorFrame, [u8; 3]) {
        return if let Some((_skill_key, skill)) = desktop.select_skill_target {
            let is_castable = char_state_storage
                .get(controller.controlled_entity.into())
                .unwrap()
                .skill_cast_allowed_at
                .get(&skill)
                .unwrap_or(&ElapsedTime(0.0))
                .has_already_passed(now);
            if !is_castable {
                (CURSOR_STOP, [255, 255, 255])
            } else if skill.get_definition().get_skill_target_type() != SkillTargetType::Area {
                (CURSOR_TARGET, [255, 255, 255])
            } else {
                (CURSOR_CLICK, [255, 255, 255])
            }
        } else if let Some(entity_below_cursor) =
            desktop.entities_below_cursor.get_enemy_or_friend()
        {
            let ent_is_dead_or_friend = {
                if let Some(auth_state) = auth_char_state_storage.get(entity_below_cursor.into()) {
                    let char_state = char_state_storage.get(entity_below_cursor.into()).unwrap();
                    !auth_state.state().is_alive() || char_state.team.is_ally_to(self_team)
                } else {
                    false
                }
            };
            if entity_below_cursor == controller.controlled_entity || ent_is_dead_or_friend {
                // self or dead
                (CURSOR_NORMAL, [51, 117, 230])
            } else {
                (CURSOR_NORMAL, [255, 0, 0])
            }
        } else if !desktop.cell_below_cursor_walkable {
            (CURSOR_STOP, [255, 255, 255])
        } else {
            (CURSOR_NORMAL, [255, 255, 255])
        };
    }
}

impl InputToNextActionSystem {
    fn determine_intention(
        auth_char_state_storage: &ReadStorage<AuthorizedCharStateComponent>,
        input: &HumanInputComponent,
        desktop: &mut LocalPlayerControllerComponent,
        just_pressed_skill_key: Option<SkillKey>,
        just_released_skill_key: Option<SkillKey>,
        alt_down: bool,
    ) -> Option<PlayerIntention> {
        return if let Some((casting_skill_key, skill)) = desktop.select_skill_target {
            if skill == Skills::AttackMove {
                if input.left_mouse_pressed {
                    desktop.select_skill_target = None;
                    Some(PlayerIntention::AttackTowards(input.mouse_world_pos))
                } else if input.right_mouse_pressed || input.is_key_just_pressed(Scancode::Escape) {
                    desktop.select_skill_target = None;
                    None
                } else {
                    None
                }
            } else {
                match input.cast_mode {
                    CastMode::Normal => {
                        if input.left_mouse_released {
                            log::debug!("Player wants to cast {:?}", skill);
                            desktop.select_skill_target = None;
                            // TODO2
                            //                                Some(PlayerIntention::Casting(
                            //                                    skill,
                            //                                    false,
                            //                                    input.mouse_world_pos,
                            //                                ))
                            None
                        } else if input.right_mouse_pressed
                            || input.is_key_just_pressed(Scancode::Escape)
                        {
                            desktop.select_skill_target = None;
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
                            );
                            if let Some(s) = shhh {
                                Some(s)
                            } else {
                                if !input.is_console_open {
                                    desktop.select_skill_target = Some((skill_key, skill));
                                }
                                None
                            }
                        } else {
                            None
                        }
                    }
                    CastMode::OnKeyRelease => {
                        if input.is_key_just_released(casting_skill_key.scancode()) {
                            log::debug!("Player wants to cast {:?}", skill);
                            // TODO2
                            desktop.select_skill_target = None;
                            //                                Some(
                            //                                    PlayerIntention::Casting(
                            //                                        input.get_skill_for_key(casting_skill_key)
                            //                                            .expect("'is_casting_selection' must be Some only if the casting skill is valid! "),
                            //                                        false,
                            //                                        input.mouse_world_pos,
                            //                                    )
                            //                                )
                            None
                        } else if input.right_mouse_pressed
                            || input.is_key_just_pressed(Scancode::Escape)
                        {
                            desktop.select_skill_target = None;
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
                        );
                        if let Some(s) = shh {
                            Some(s)
                        } else {
                            if !input.is_console_open {
                                desktop.select_skill_target = Some((skill_key, skill));
                            }
                            None
                        }
                    } else {
                        None
                    }
                }
                CastMode::OnKeyPress => {
                    log::debug!("Player wants to cast {:?}, alt={:?}", skill, alt_down);
                    desktop.select_skill_target = None;
                    // TODO2
                    //                        Some(PlayerIntention::Casting(
                    //                            skill,
                    //                            alt_down,
                    //                            input.mouse_world_pos,
                    //                        ))
                    None
                }
            }
        } else if let Some((_skill_key, skill)) = just_released_skill_key.and_then(|skill_key| {
            input
                .get_skill_for_key(skill_key)
                .map(|skill| (skill_key, skill))
        }) {
            // can get here only when alt was down and OnKeyRelease
            if alt_down {
                log::debug!("Player wants to cast {:?}, SELF", skill);
                // TODO2
                //                    Some(PlayerIntention::Casting(skill, true, input.mouse_world_pos))
                None
            } else {
                None
            }
        } else if input.right_mouse_pressed || input.right_mouse_down {
            Some(PlayerIntention::MoveTowardsMouse(input.mouse_world_pos))
        } else if input.right_mouse_released {
            if let Some(target_entity_id) = desktop.entities_below_cursor.get_enemy() {
                if auth_char_state_storage
                    .get(target_entity_id.into())
                    .map(|it| !it.state().is_dead())
                    .unwrap_or(false)
                {
                    Some(PlayerIntention::Attack(target_entity_id))
                } else {
                    Some(PlayerIntention::MoveTo(input.mouse_world_pos))
                }
            } else {
                Some(PlayerIntention::MoveTo((input.mouse_world_pos).clone()))
            }
        // TODO2
        //            } else if let Some(PlayerIntention::Casting(..)) = &controller.last_action {
        //                // Casting might have been rejected because for example the char was attacked at the time, but
        //                // we want to cast it as soon as the rejection reason ceases AND there is no other intention
        //                if controller.repeat_next_action {
        //                    controller.last_action.clone()
        //                } else {
        //                    None
        //                }
        } else {
            None
        };
    }
}
