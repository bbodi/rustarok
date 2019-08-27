use crate::components::char::PhysicsComponent;
use crate::components::controller::{ConsoleComponent, HumanInputComponent};
use crate::components::status::absorb_shield::AbsorbStatus;
use crate::components::status::status::{
    ApplyStatusComponent, ApplyStatusComponentPayload, MainStatuses,
};
use crate::systems::render::opengl_render_sys::{NORMAL_FONT_H, NORMAL_FONT_W};
use crate::systems::render::render_command::{Font, RenderCommandCollectorComponent, UiLayer2d};
use crate::systems::SystemVariables;
use crate::video::{VIDEO_HEIGHT, VIDEO_WIDTH};
use crate::PhysicEngine;
use nalgebra::Isometry2;
use sdl2::keyboard::Scancode;
use specs::prelude::*;
use std::collections::HashMap;

pub struct ConsoleSystem<'a> {
    command_defs: &'a HashMap<String, CommandDefinition>,
}

pub struct SetPosCommand;

impl AutocompletionGenerator for SetPosCommand {
    fn create_autocompletion_list(&self) -> Vec<String> {
        vec!["sharp".to_owned(), "béla".to_owned(), "Józsi".to_owned()]
    }
}

impl From<SetPosCommand> for CommandDefinition {
    fn from(cmd: SetPosCommand) -> Self {
        CommandDefinition {
            name: "set_pos".to_string(),
            arguments: vec![
                ("x", CommandParamType::Int, true),
                ("y", CommandParamType::Int, true),
                ("[username]", CommandParamType::String, false),
            ],
            action: Box::new(|self_entity_id, args, ecs_world| {
                let x = args.as_int(0).unwrap();
                let y = args.as_int(1).unwrap();
                let username = args.as_str(2);

                let entity_id = if let Some(username) = username {
                    ConsoleSystem::get_user_id_by_name(ecs_world, username)
                } else {
                    Some(self_entity_id)
                };

                let body_handle = entity_id.and_then(|it| {
                    ecs_world
                        .read_storage::<PhysicsComponent>()
                        .get(it)
                        .map(|it| it.body_handle)
                });

                if let Some(body_handle) = body_handle {
                    let physics_world = &mut ecs_world.write_resource::<PhysicEngine>();
                    if let Some(body) = physics_world.bodies.rigid_body_mut(body_handle) {
                        body.set_position(Isometry2::translation(x as f32, y as f32));
                        Ok(())
                    } else {
                        Err("No rigid body was found for this user".to_owned())
                    }
                } else {
                    Err("The user was not found".to_owned())
                }
            }),
        }
    }
}

impl<'a> ConsoleSystem<'a> {
    pub fn new(command_defs: &'a HashMap<String, CommandDefinition>) -> ConsoleSystem {
        ConsoleSystem { command_defs }
    }

    fn get_byte_pos(text: &str, index: usize) -> usize {
        text.char_indices()
            .nth(index as usize)
            .unwrap_or((text.len(), '0'))
            .0
    }

    fn get_user_id_by_name(ecs_world: &specs::World, username: &str) -> Option<Entity> {
        let mut user_entity_id: Option<Entity> = None;
        for (entity_id, human) in (
            &ecs_world.entities(),
            &ecs_world.read_storage::<HumanInputComponent>(),
        )
            .join()
        {
            if human.username == username {
                user_entity_id = Some(entity_id);
                break;
            }
        }
        return user_entity_id;
    }

    pub fn init_commands() -> HashMap<String, CommandDefinition> {
        let mut command_defs: HashMap<String, CommandDefinition> = HashMap::new();
        command_defs.insert("set_pos".to_owned(), SetPosCommand.into());

        command_defs.insert(
            "add_status".to_owned(),
            CommandDefinition {
                name: "add_status".to_string(),
                arguments: vec![
                    ("status_name", CommandParamType::String, true),
                    ("time(ms)", CommandParamType::Int, true),
                    ("[username]", CommandParamType::String, false),
                ],
                action: Box::new(|self_entity_id, args, ecs_world| {
                    let status_name = args.as_str(0).unwrap();
                    let time = args.as_int(1).unwrap();

                    let username = args.as_str(2);
                    let entity_id = if let Some(username) = username {
                        ConsoleSystem::get_user_id_by_name(ecs_world, username)
                    } else {
                        Some(self_entity_id)
                    };

                    if let Some(entity_id) = entity_id {
                        let mut system_vars = ecs_world.write_resource::<SystemVariables>();
                        let now = system_vars.time;
                        system_vars.apply_statuses.push(ApplyStatusComponent {
                            source_entity_id: self_entity_id,
                            target_entity_id: entity_id,
                            status: match status_name {
                                "absorb" => ApplyStatusComponentPayload::from_secondary(Box::new(
                                    AbsorbStatus::new(self_entity_id, now),
                                )),
                                _ => ApplyStatusComponentPayload::from_main_status(
                                    MainStatuses::Poison,
                                ),
                            },
                        });
                        Ok(())
                    } else {
                        Err("The user was not found".to_owned())
                    }
                }),
            },
        );

        return command_defs;
    }
}

trait AutocompletionGenerator {
    fn create_autocompletion_list(&self) -> Vec<String>;
}

#[derive(Copy, Clone)]
pub enum CommandParamType {
    String, // Box<dyn AutocompletionGenerator>
    Int,
    Float,
}

pub struct CommandArguments {
    args: Vec<(String, (usize, usize))>, // argument, start and end pos
}

impl CommandArguments {
    fn new(text: &str) -> CommandArguments {
        let mut args = Vec::with_capacity(3);
        let mut qoute_started = false;
        let mut current_str = String::with_capacity(12);
        let mut text_started = false;
        let mut start_pos = 0;
        for (i, ch) in text.chars().enumerate() {
            let push = if ch == '"' && qoute_started {
                true
            } else if ch == '"' && !qoute_started {
                qoute_started = true;
                text_started = true;
                start_pos = i;
                false
            } else if !ch.is_whitespace() && !text_started {
                text_started = true;
                current_str.push(ch);
                start_pos = i;
                false
            } else if ch.is_whitespace() && !text_started {
                // skip whitespaces between arguments
                false
            } else if ch.is_whitespace() && text_started && !qoute_started {
                true
            } else {
                current_str.push(ch);
                false
            };
            if push {
                args.push((current_str, (start_pos, i)));
                current_str = String::with_capacity(12);
                text_started = false;
                qoute_started = false;
            }
        }
        if !current_str.is_empty() {
            // push the last param
            let len = current_str.chars().count();
            args.push((current_str, (start_pos, start_pos + len)));
        }
        CommandArguments { args }
    }

    pub fn is_cursor_inside_arg(&self, index: usize, cursor_x: usize) -> bool {
        if let Some((arg, (start, end))) = self.args.get(index + 1) {
            cursor_x >= *start && cursor_x <= *end
        } else {
            // there is no entry for this arg, check if we are after the prev arg
            let end_of_prev_arg = self.args.get(index).map(|it| (it.1).1);
            if let Some(end_of_prev_arg) = end_of_prev_arg {
                end_of_prev_arg < cursor_x
            } else {
                false
            }
        }
    }

    pub fn get_command_name(&self) -> Option<&String> {
        self.args.get(0).map(|it| &it.0)
    }
    // first argument is the command name!
    pub fn as_int(&self, index: usize) -> Option<i32> {
        self.args.get(index + 1).map(|it| it.0.parse().unwrap())
    }

    pub fn as_str(&self, index: usize) -> Option<&str> {
        self.args.get(index + 1).map(|it| it.0.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        assert_eq!(
            "first_arg",
            CommandArguments::new("skip first_arg").as_str(0).unwrap()
        );
        assert_eq!(
            "first_arg",
            CommandArguments::new("skip \"first_arg\"")
                .as_str(0)
                .unwrap()
        );
        assert_eq!(
            "first_arg with    spaces",
            CommandArguments::new("skip \"first_arg with    spaces\"")
                .as_str(0)
                .unwrap()
        );
        assert_eq!(
            "first_arg",
            CommandArguments::new("skip \"first_arg").as_str(0).unwrap()
        );

        assert_eq!(
            "first_arg",
            CommandArguments::new("  skip first_arg").as_str(0).unwrap()
        );

        assert_eq!(
            "3",
            CommandArguments::new("skip 1 2   3").as_str(2).unwrap()
        );
    }
}

pub struct CommandDefinition {
    pub name: String,
    pub arguments: Vec<(&'static str, CommandParamType, bool)>, // name, type, mandatory
    pub action: CommandCallback,
}

pub type CommandCallback =
    Box<dyn Fn(Entity, &CommandArguments, &mut specs::World) -> Result<(), String>>;

enum ConsoleWordType {
    Normal,
    Error,
}

struct ConsoleWords {
    text: String,
    typ: ConsoleWordType,
}

impl ConsoleWords {
    fn new(text: &str, typ: ConsoleWordType) -> ConsoleWords {
        ConsoleWords {
            text: text.to_owned(),
            typ,
        }
    }
}

impl ConsoleEntry {
    pub fn new() -> ConsoleEntry {
        ConsoleEntry {
            words: Vec::with_capacity(12),
        }
    }

    pub fn add_error(mut self, text: &str) -> ConsoleEntry {
        self.words
            .push(ConsoleWords::new(text, ConsoleWordType::Error));
        self
    }

    pub fn add_normal(mut self, text: &str) -> ConsoleEntry {
        self.words
            .push(ConsoleWords::new(text, ConsoleWordType::Normal));
        self
    }
}

pub struct ConsoleEntry {
    words: Vec<ConsoleWords>,
}

impl<'a, 'b> specs::System<'a> for ConsoleSystem<'b> {
    type SystemData = (
        specs::ReadStorage<'a, HumanInputComponent>,
        specs::WriteStorage<'a, ConsoleComponent>,
        specs::WriteStorage<'a, RenderCommandCollectorComponent>,
        specs::ReadExpect<'a, SystemVariables>,
    );

    fn run(
        &mut self,
        (
            input_storage,
            mut console_storage,
            mut render_collector_storage,
            system_vars,
        ): Self::SystemData,
    ) {
        for (input, render_commands, console) in (
            &input_storage,
            &mut render_collector_storage,
            &mut console_storage,
        )
            .join()
        {
            let now = system_vars.time;
            let console_color = system_vars.dev_configs.console.color;
            let console_height = (VIDEO_HEIGHT / 3) as i32;
            let repeat_time = 0.1;
            if !input.is_console_open {
                if console.y_pos > 0 {
                    console.y_pos -= 3;
                }
            } else {
                if console.y_pos < console_height {
                    console.y_pos += 3;
                }
                if console.cursor_change.is_earlier_than(system_vars.time) {
                    console.cursor_shown = !console.cursor_shown;
                    console.cursor_change = system_vars.time.add_seconds(0.5);
                }

                if input.is_key_just_pressed(Scancode::Up) {
                    if console.history_pos < console.command_history.len() {
                        console.history_pos += 1;
                    }
                    let idx = console.command_history.len() - console.history_pos;
                    console.input = console.command_history[idx].clone();
                    console.cursor_x = console.input.chars().count();
                } else if input.is_key_just_pressed(Scancode::Down) {
                    if console.history_pos > 1 {
                        console.history_pos -= 1;
                        let idx = console.command_history.len() - console.history_pos;
                        console.input = console.command_history[idx].clone();
                        console.cursor_x = console.input.chars().count();
                    } else {
                        console.history_pos = 0;
                        console.input = String::with_capacity(32);
                        console.cursor_x = 0;
                    }
                } else if input.is_key_down(Scancode::Left)
                    && console.cursor_x > 0
                    && console.key_repeat_allowed_at.is_earlier_than(now)
                {
                    ConsoleSystem::handle_left_cursor(input, console);
                    console.key_repeat_allowed_at = now.add_seconds(repeat_time);
                } else if input.is_key_down(Scancode::Right)
                    && console.cursor_x < console.input.chars().count()
                    && console.key_repeat_allowed_at.is_earlier_than(now)
                {
                    ConsoleSystem::handle_right_cursor(input, console);
                    console.key_repeat_allowed_at = now.add_seconds(repeat_time);
                } else if input.is_key_down(Scancode::Home) {
                    console.cursor_x = 0;
                } else if input.is_key_down(Scancode::End) {
                    console.cursor_x = console.input.chars().count();
                } else if input.is_key_down(Scancode::Backspace)
                    && console.cursor_x > 0
                    && console.key_repeat_allowed_at.is_earlier_than(now)
                {
                    if console.cursor_x as usize >= console.input.chars().count() {
                        console.input.pop();
                    } else {
                        let idx = ConsoleSystem::get_byte_pos(&console.input, console.cursor_x - 1);
                        console.input.remove(idx);
                    }
                    console.cursor_x -= 1;
                    console.key_repeat_allowed_at = now.add_seconds(repeat_time);
                } else if input.is_key_down(Scancode::Delete)
                    && console.cursor_x < console.input.chars().count()
                    && console.key_repeat_allowed_at.is_earlier_than(now)
                {
                    if console.cursor_x as usize >= console.input.chars().count() - 1 {
                        console.input.pop();
                    } else {
                        let idx = ConsoleSystem::get_byte_pos(&console.input, console.cursor_x);
                        console.input.remove(idx);
                    }
                    console.key_repeat_allowed_at = now.add_seconds(repeat_time);
                } else if !input.text.is_empty() {
                    let idx = ConsoleSystem::get_byte_pos(&console.input, console.cursor_x);
                    console.input.insert_str(idx, &input.text);
                    console.cursor_x += 1;
                } else if input.is_key_just_released(Scancode::Return) {
                    // PRESS ENTER
                    // validate input
                    let input = std::mem::replace(&mut console.input, String::with_capacity(32));
                    console.print(&input);
                    let args = CommandArguments::new(&input);
                    console.command_history.push(input);
                    console.cursor_x = 0;
                    console.history_pos = 0;
                    if let Some(command_def) = self
                        .command_defs
                        .get(args.get_command_name().unwrap_or(&"".to_owned()))
                    {
                        let mandatory_arg_count =
                            command_def.arguments.iter().take_while(|it| it.2).count();
                        let actual_arg_count = args.args.len() - 1;
                        if actual_arg_count < mandatory_arg_count
                            || actual_arg_count > command_def.arguments.len()
                        {
                            console.error(&format!(
                                "Illegal number of parameters (expected at least {}, at most {}, provided {})",
                                mandatory_arg_count,
                                command_def.arguments.len(),
                                actual_arg_count
                            ));
                            continue;
                        }

                        let ok = command_def.arguments.iter().enumerate().all(
                            |(i, (param_name, arg_type, mandatory))| {
                                let ok = match arg_type {
                                    CommandParamType::Float => args
                                        .as_str(i)
                                        .map(|it| it.parse::<f32>().is_ok())
                                        .unwrap_or(false),
                                    CommandParamType::Int => args
                                        .as_str(i)
                                        .map(|it| it.parse::<i32>().is_ok())
                                        .unwrap_or(false),
                                    CommandParamType::String => true,
                                };
                                if !ok {
                                    console.error(&format!(
                                        "{}, the {}. parameter ('{}') must be {}",
                                        param_name,
                                        i,
                                        args.as_str(i).unwrap_or(""),
                                        match *arg_type {
                                            CommandParamType::Float => "float",
                                            CommandParamType::Int => "int",
                                            CommandParamType::String => "string",
                                        }
                                    ));
                                }
                                ok
                            },
                        );
                        if ok {
                            console.command_to_execute = Some(args);
                        } else {
                            continue;
                        }
                    } else {
                        console.error("Unknown command")
                    }
                }
            }

            // Draw

            if console.y_pos > 0 {
                // background
                render_commands
                    .prepare_for_2d()
                    .screen_pos(0, 0)
                    .size2(VIDEO_WIDTH as i32, console.y_pos)
                    .color(&console_color)
                    .add_rectangle_command(UiLayer2d::Console);
                // cursor
                if console.cursor_shown {
                    render_commands
                        .prepare_for_2d()
                        .screen_pos(
                            console.cursor_x as i32 * NORMAL_FONT_W - NORMAL_FONT_W / 2,
                            console.y_pos - NORMAL_FONT_H - 3,
                        )
                        .color(&[1.0, 1.0, 1.0, console_color[3]])
                        .add_text_command("|", Font::Normal, UiLayer2d::ConsoleTexts)
                }

                let row_count = console_height / NORMAL_FONT_H;
                let input_row_y = console.y_pos - NORMAL_FONT_H - 3;
                for (i, row) in console
                    .rows
                    .iter()
                    .rev()
                    .take(row_count as usize)
                    .enumerate()
                {
                    let mut x = 0;
                    for words in &row.words {
                        render_commands
                            .prepare_for_2d()
                            .screen_pos(
                                x * NORMAL_FONT_W,
                                input_row_y - (1 + i as i32) * NORMAL_FONT_H,
                            )
                            .color(&match words.typ {
                                ConsoleWordType::Normal => [0.8, 0.8, 0.8, console_color[3]],
                                ConsoleWordType::Error => [1.0, 0.0, 0.0, console_color[3]],
                            })
                            .add_text_command(&words.text, Font::Normal, UiLayer2d::ConsoleTexts);
                        x += words.text.chars().count() as i32;
                    }
                }

                // input prompt
                if !console.input.is_empty() {
                    render_commands
                        .prepare_for_2d()
                        .screen_pos(0, input_row_y)
                        .color(&[1.0, 1.0, 1.0, console_color[3]])
                        .add_text_command(&console.input, Font::Normal, UiLayer2d::ConsoleTexts)
                }

                let current_args = CommandArguments::new(&console.input);
                if let Some(command_def) = self
                    .command_defs
                    .get(current_args.get_command_name().unwrap_or(&"".to_owned()))
                {
                    // draw help prompt above the cursor
                    let help_text_len: usize = command_def
                        .arguments
                        .iter()
                        .map(|it| it.0.chars().count())
                        .sum::<usize>()
                        + command_def.arguments.len() // spaces
                        - 1;
                    let start_x = ((console.cursor_x as i32 - help_text_len as i32 / 2)
                        * NORMAL_FONT_W)
                        .max(0);
                    // background
                    render_commands
                        .prepare_for_2d()
                        .screen_pos(start_x, console.y_pos - NORMAL_FONT_H * 2 - 3)
                        .size2(help_text_len as i32 * NORMAL_FONT_W, NORMAL_FONT_H)
                        .color(&[55.0 / 255.0, 57.0 / 255.0, 57.0 / 255.0, console_color[3]])
                        .add_rectangle_command(UiLayer2d::ConsoleAutocompletion);
                    // text
                    let mut x: usize = 0;
                    command_def
                        .arguments
                        .iter()
                        .map(|it| it.0.to_owned())
                        .enumerate()
                        .for_each(|(i, param_name)| {
                            let is_cursor_inside_arg =
                                current_args.is_cursor_inside_arg(i, console.cursor_x);
                            let color = if is_cursor_inside_arg {
                                [1.0, 1.0, 1.0, console_color[3]] // active argument
                            } else {
                                [85.0 / 255.0, 87.0 / 255.0, 87.0 / 255.0, console_color[3]]
                            };
                            render_commands
                                .prepare_for_2d()
                                .screen_pos(
                                    start_x + x as i32,
                                    console.y_pos - NORMAL_FONT_H * 2 - 3,
                                )
                                .color(&color)
                                .add_text_command(
                                    &param_name,
                                    Font::Normal,
                                    UiLayer2d::ConsoleAutocompletion,
                                );
                            x += (param_name.chars().count() + 1) * NORMAL_FONT_W as usize;
                        });
                }
            }
        }
    }
}

impl<'a> ConsoleSystem<'a> {
    fn handle_right_cursor(input: &HumanInputComponent, console: &mut ConsoleComponent) {
        if input.is_key_down(Scancode::LCtrl) || input.is_key_down(Scancode::RCtrl) {
            // find first non-alpha character
            let next_char_is_space = console
                .input
                .chars()
                .nth(console.cursor_x as usize)
                .unwrap()
                .is_whitespace();
            let predicate = |ch: char| -> bool {
                if next_char_is_space {
                    !ch.is_whitespace()
                } else {
                    ch.is_whitespace()
                }
            };
            let idx = ConsoleSystem::get_byte_pos(&console.input, console.cursor_x);
            if let Some(ix) = console.input[idx..].chars().position(predicate) {
                console.cursor_x += ix;
            } else {
                // not found, jump to the end
                console.cursor_x = console.input.chars().count();
            }
        } else {
            console.cursor_x = (console.cursor_x + 1).min(console.input.chars().count());
        }
    }

    fn handle_left_cursor(input: &HumanInputComponent, console: &mut ConsoleComponent) {
        if input.is_key_down(Scancode::LCtrl) || input.is_key_down(Scancode::RCtrl) {
            // find first non-alpha character
            let prev_char_is_space = console
                .input
                .chars()
                .nth(console.cursor_x as usize - 1)
                .unwrap()
                .is_whitespace();
            let predicate = |ch: char| -> bool {
                if prev_char_is_space {
                    !ch.is_whitespace()
                } else {
                    ch.is_whitespace()
                }
            };

            let idx = ConsoleSystem::get_byte_pos(&console.input, console.cursor_x);
            if let Some(ix) = console.input[0..idx].chars().rev().position(predicate) {
                console.cursor_x -= ix;
            } else {
                // not found, jump to the beginning
                console.cursor_x = 0;
            }
        } else {
            console.cursor_x = (console.cursor_x - 1).max(0);
        }
    }
}
