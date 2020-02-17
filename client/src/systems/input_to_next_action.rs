use crate::client::SimulationTime;
use crate::components::char::CharacterStateComponent;
use crate::components::controller::{
    CastMode, HumanInputComponent, LocalPlayerController, SkillKey,
};
use crate::components::skills::skills::{SkillTargetType, Skills};
use crate::cursor::{CursorFrame, CURSOR_CLICK, CURSOR_NORMAL, CURSOR_STOP, CURSOR_TARGET};
use crate::runtime_assets::map::MapRenderData;
use crate::systems::input_sys::InputConsumerSystem;
use crate::systems::{SystemFrameDurations, SystemVariables};
use crate::LocalTime;
use rustarok_common::common::EngineTime;
use rustarok_common::common::SimulationTick;
use rustarok_common::components::char::{
    LocalCharEntityId, LocalCharStateComp, StaticCharDataComponent, Team,
};
use rustarok_common::components::controller::PlayerIntention;
use rustarok_common::systems::intention_applier::ControllerIntentionToCharTarget;
use sdl2::keyboard::Scancode;
use specs::prelude::*;
use strum::IntoEnumIterator;

// Singleton
pub struct InputToNextActionSystem {
    last_input_tick: SimulationTick,
    prev_intention: Option<PlayerIntention>,
    prev_prev_intention: Option<PlayerIntention>,
}

impl InputToNextActionSystem {
    pub fn new() -> InputToNextActionSystem {
        InputToNextActionSystem {
            last_input_tick: SimulationTick::new(),
            prev_intention: None,
            prev_prev_intention: None,
        }
    }
}

impl InputToNextActionSystem {
    pub fn run(
        &mut self,
        input: &HumanInputComponent,
        static_char_data_storage: ReadStorage<StaticCharDataComponent>,
        auth_char_state_storage: ReadStorage<LocalCharStateComp>,
        local_player: &mut LocalPlayerController,
        system_benchmark: &mut SystemFrameDurations,
        time: &EngineTime,
        sim_time: &SimulationTime,
        sim_tick: SimulationTick,
        map_render_data: &MapRenderData,
    ) {
        let _stopwatch = system_benchmark.start_measurement("InputToNextActionSystem");
        let current_frame_intention = {
            if local_player.controller.controlled_entity.is_none() {
                return;
            }
            let controlled_entity_id = local_player.controller.controlled_entity.unwrap();

            let self_char_team = static_char_data_storage
                .get(controlled_entity_id.into())
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

            local_player.calc_entities_below_cursor(
                self_char_team,
                input.last_mouse_x,
                input.last_mouse_y,
            );

            local_player.cell_below_cursor_walkable = map_render_data.gat.is_walkable(
                input.mouse_world_pos.x.max(0.0) as usize,
                input.mouse_world_pos.y.abs() as usize,
            );
            let (cursor_frame, cursor_color) = InputToNextActionSystem::determine_cursor(
                time.now(),
                &local_player,
                controlled_entity_id,
                &auth_char_state_storage,
                &static_char_data_storage,
                self_char_team,
            );
            local_player.cursor_anim_descr.action_index = cursor_frame.1;
            local_player.cursor_color = cursor_color;

            let alt_down = input.alt_down;
            let (current_frame_intention, new_select_skill_target) =
                InputToNextActionSystem::determine_intention(
                    &auth_char_state_storage,
                    &input,
                    &local_player,
                    just_pressed_skill_key,
                    just_released_skill_key,
                    alt_down,
                );
            local_player.select_skill_target = new_select_skill_target;

            current_frame_intention
        };

        if sim_tick.as_u64() % 3 == 0 && sim_tick > self.last_input_tick {
            let controller = &mut local_player.controller;
            self.last_input_tick = sim_tick;
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
            };
        } else {
            let controller = &mut local_player.controller;
            controller.intention = None;
        }
        let controller = &local_player.controller;
        if controller.intention.is_some() {
            // here 'intention' is the action from the prev frame
            local_player.last_intention = controller.intention.clone();
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

//impl<'a> System<'a> for InputToNextActionSystem {
//    type SystemData = (
//        ReadExpect<'a, HumanInputComponent>,
//        ReadStorage<'a, StaticCharDataComponent>,
//        ReadStorage<'a, LocalCharStateComp>,
//        WriteExpect<'a, LocalPlayerController>,
//        WriteExpect<'a, SystemFrameDurations>,
//        ReadExpect<'a, SystemVariables>,
//        ReadExpect<'a, EngineTime>,
//        ReadExpect<'a, SimulationTime>,
//        ReadExpect<'a, SimulationTick>,
//        ReadExpect<'a, MapRenderData>,
//    );
//
//    fn run(
//        &mut self,
//        (
//            input,
//            static_char_data_storage,
//            auth_char_state_storage,
//            mut local_player,
//            mut system_benchmark,
//            time,
//            sim_time,
//            sim_tick,
//            map_render_data,
//        ): Self::SystemData,
//    ) {
//    }
//}

pub struct ClientIntentionToCharTargetSystem;
impl<'a> System<'a> for ClientIntentionToCharTargetSystem {
    type SystemData = (
        WriteStorage<'a, LocalCharStateComp>,
        ReadExpect<'a, LocalPlayerController>,
    );

    fn run(&mut self, (mut auth_char_state_storage, local_player): Self::SystemData) {
        ControllerIntentionToCharTarget::controller_intention_to_char_target(
            &local_player.controller,
            &mut auth_char_state_storage,
        )
    }
}

impl InputToNextActionSystem {
    pub fn determine_cursor(
        now: LocalTime,
        local_player: &LocalPlayerController,
        controlled_entity: LocalCharEntityId,
        auth_char_state_storage: &ReadStorage<LocalCharStateComp>,
        static_char_data_storage: &ReadStorage<StaticCharDataComponent>,
        self_team: Team,
    ) -> (CursorFrame, [u8; 3]) {
        return if let Some((skill_key, skill)) = local_player.select_skill_target {
            let is_castable = auth_char_state_storage
                .get(controlled_entity.into())
                .unwrap()
                .skill_cast_allowed_at[skill_key as usize]
                .has_already_passed(now);
            if !is_castable {
                (CURSOR_STOP, [255, 255, 255])
            } else if skill.get_definition().get_skill_target_type() != SkillTargetType::Area {
                (CURSOR_TARGET, [255, 255, 255])
            } else {
                (CURSOR_CLICK, [255, 255, 255])
            }
        } else if let Some(entity_below_cursor) =
            local_player.entities_below_cursor.get_enemy_or_friend()
        {
            let ent_is_dead_or_friend = {
                if let Some(auth_state) = auth_char_state_storage.get(entity_below_cursor.into()) {
                    let char_state = static_char_data_storage
                        .get(entity_below_cursor.into())
                        .unwrap();
                    !auth_state.state().is_alive() || char_state.team.is_ally_to(self_team)
                } else {
                    false
                }
            };
            if entity_below_cursor == controlled_entity || ent_is_dead_or_friend {
                // self or dead
                (CURSOR_NORMAL, [51, 117, 230])
            } else {
                (CURSOR_NORMAL, [255, 0, 0])
            }
        } else if !local_player.cell_below_cursor_walkable {
            (CURSOR_STOP, [255, 255, 255])
        } else {
            (CURSOR_NORMAL, [255, 255, 255])
        };
    }
}

impl InputToNextActionSystem {
    fn determine_intention(
        auth_char_state_storage: &ReadStorage<LocalCharStateComp>,
        input: &HumanInputComponent,
        local_player: &LocalPlayerController,
        just_pressed_skill_key: Option<SkillKey>,
        just_released_skill_key: Option<SkillKey>,
        alt_down: bool,
    ) -> (Option<PlayerIntention>, Option<(SkillKey, Skills)>) {
        return if let Some((casting_skill_key, skill)) = local_player.select_skill_target {
            if skill == Skills::AttackMove {
                if input.left_mouse_pressed {
                    (
                        Some(PlayerIntention::AttackTowards(input.mouse_world_pos)),
                        None,
                    )
                } else if input.right_mouse_pressed || input.is_key_just_pressed(Scancode::Escape) {
                    (None, None)
                } else {
                    (None, local_player.select_skill_target)
                }
            } else {
                match input.cast_mode {
                    CastMode::Normal => {
                        if input.left_mouse_released {
                            log::debug!("Player wants to cast {:?}", skill);
                            // TODO2
                            //                                Some(PlayerIntention::Casting(
                            //                                    skill,
                            //                                    false,
                            //                                    input.mouse_world_pos,
                            //                                ))
                            (None, None)
                        } else if input.right_mouse_pressed
                            || input.is_key_just_pressed(Scancode::Escape)
                        {
                            (None, None)
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
                                (Some(s), local_player.select_skill_target)
                            } else {
                                let new_target = if !input.is_console_open {
                                    Some((skill_key, skill))
                                } else {
                                    local_player.select_skill_target
                                };
                                (None, new_target)
                            }
                        } else {
                            (None, local_player.select_skill_target)
                        }
                    }
                    CastMode::OnKeyRelease => {
                        if input.is_key_just_released(casting_skill_key.scancode()) {
                            log::debug!("Player wants to cast {:?}", skill);
                            // TODO2
                            //                                Some(
                            //                                    PlayerIntention::Casting(
                            //                                        input.get_skill_for_key(casting_skill_key)
                            //                                            .expect("'is_casting_selection' must be Some only if the casting skill is valid! "),
                            //                                        false,
                            //                                        input.mouse_world_pos,
                            //                                    )
                            //                                )
                            (None, None)
                        } else if input.right_mouse_pressed
                            || input.is_key_just_pressed(Scancode::Escape)
                        {
                            (None, None)
                        } else {
                            (None, local_player.select_skill_target)
                        }
                    }
                    CastMode::OnKeyPress => {
                        // not possible to get into this state while OnKeyPress is active
                        (None, local_player.select_skill_target)
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
                            (Some(s), local_player.select_skill_target)
                        } else {
                            let new_target = if !input.is_console_open {
                                Some((skill_key, skill))
                            } else {
                                local_player.select_skill_target
                            };
                            (None, new_target)
                        }
                    } else {
                        (None, local_player.select_skill_target)
                    }
                }
                CastMode::OnKeyPress => {
                    log::debug!("Player wants to cast {:?}, alt={:?}", skill, alt_down);
                    // TODO2
                    //                        Some(PlayerIntention::Casting(
                    //                            skill,
                    //                            alt_down,
                    //                            input.mouse_world_pos,
                    //                        ))
                    (None, None)
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
                (None, local_player.select_skill_target)
            } else {
                (None, local_player.select_skill_target)
            }
        } else if input.right_mouse_pressed || input.right_mouse_down {
            (
                Some(PlayerIntention::MoveTowardsMouse(input.mouse_world_pos)),
                local_player.select_skill_target,
            )
        } else if input.right_mouse_released {
            if let Some(target_entity_id) = local_player.entities_below_cursor.get_enemy() {
                if auth_char_state_storage
                    .get(target_entity_id.into())
                    .map(|it| !it.state().is_dead())
                    .unwrap_or(false)
                {
                    (
                        Some(PlayerIntention::Attack(target_entity_id)),
                        local_player.select_skill_target,
                    )
                } else {
                    (
                        Some(PlayerIntention::MoveTo(input.mouse_world_pos)),
                        local_player.select_skill_target,
                    )
                }
            } else {
                (
                    Some(PlayerIntention::MoveTo((input.mouse_world_pos).clone())),
                    local_player.select_skill_target,
                )
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
            (None, local_player.select_skill_target)
        };
    }
}
