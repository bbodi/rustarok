use crate::components::char::CharacterStateComponent;
use crate::components::controller::HumanInputComponent;
use crate::render::opengl_render_sys::{NORMAL_FONT_H, NORMAL_FONT_W};
use crate::render::render_command::{Font, RenderCommandCollector, UiLayer2d};
use crate::systems::console_commands::{
    cmd_add_falcon, cmd_add_status, cmd_bind_key, cmd_clear, cmd_clone_char, cmd_control_char,
    cmd_disable_collision, cmd_enable_collision, cmd_follow_char, cmd_get_pos, cmd_goto, cmd_heal,
    cmd_inspect, cmd_kill_all, cmd_list_entities, cmd_list_players, cmd_list_statuses,
    cmd_reload_configs, cmd_remove_falcon, cmd_resurrect, cmd_set_config, cmd_set_damping,
    cmd_set_fullscreen, cmd_set_job, cmd_set_mass, cmd_set_outlook, cmd_set_pos,
    cmd_set_resolution, cmd_set_team, cmd_spawn_area, cmd_spawn_entity, cmd_toggle_console,
};
use crate::systems::SystemVariables;
use crate::video::Video;
use crate::GameTime;
use rustarok_common::common::{EngineTime, Local};
use rustarok_common::components::char::{CharType, EntityId, StaticCharDataComponent};
use rustarok_common::config::CommonConfigs;
use rustarok_common::console::{CommandArguments, CommandElement};
use rustarok_common::packets::to_server::ToServerPacket::ConsoleCommand;
use sdl2::keyboard::Scancode;
use specs::prelude::*;
use std::collections::HashMap;

// add a slider for a vairable?

#[derive(Eq, PartialEq)]
enum AutocompletionType {
    CommandName,
    Param,
    CommandHistory,
}

pub struct ConsoleComponent {
    command_history: Vec<String>,
    rows: Vec<ConsoleEntry>,
    history_pos: usize,
    autocompletion_open: Option<AutocompletionType>,
    autocompletion_index: usize,
    full_autocompletion_list: Vec<String>,
    filtered_autocompletion_list: Vec<String>,
    cursor_x: usize,
    cursor_inside_quotes: bool,
    cursor_parameter_index: usize,
    input: String,
    args: CommandArguments,
    y_pos: i32,
    cursor_shown: bool,
    cursor_change: GameTime<Local>,
    key_repeat_allowed_at: GameTime<Local>,
    pub command_to_execute: Option<CommandArguments>,
}

impl ConsoleComponent {
    pub fn new() -> ConsoleComponent {
        ConsoleComponent {
            args: CommandArguments::new(""),
            autocompletion_index: 0,
            cursor_inside_quotes: false,
            autocompletion_open: None,
            cursor_parameter_index: 0,
            full_autocompletion_list: vec![],
            filtered_autocompletion_list: vec![],
            history_pos: 0,
            command_history: vec![],
            rows: vec![],
            cursor_x: 0,
            input: "".to_string(),
            y_pos: 0,
            cursor_shown: false,
            cursor_change: GameTime::from(0.0),
            key_repeat_allowed_at: GameTime::from(0.0),
            command_to_execute: None,
        }
    }

    pub fn clear(&mut self) {
        self.rows.clear();
    }

    pub fn set_cursor_x(&mut self, new_x: usize) {
        self.cursor_x = new_x;
        self.cursor_or_input_has_changed();
    }

    pub fn set_input_and_cursor_x(&mut self, new_x: usize, new_input: String) {
        self.input = new_input;
        self.cursor_x = new_x;
        self.cursor_or_input_has_changed();
    }

    pub fn set_input(&mut self, new_input: String) {
        self.input = new_input;
        self.cursor_or_input_has_changed();
    }

    pub fn cursor_or_input_has_changed(&mut self) {
        // check if cursor is inside quotes
        self.cursor_inside_quotes = false;
        for ch in self.input.chars().take(self.cursor_x) {
            if ch == '"' && self.cursor_inside_quotes {
                self.cursor_inside_quotes = false;
            } else if ch == '"' && !self.cursor_inside_quotes {
                self.cursor_inside_quotes = true;
            }
        }

        // check if cursor inside parameters
        self.args = CommandArguments::new(&self.input);
        let old_index = self.cursor_parameter_index;
        if let Some(index) = (0..10).find(|i| self.args.is_cursor_inside_arg(*i, self.cursor_x)) {
            self.cursor_parameter_index = index + 1;
        } else {
            self.cursor_parameter_index = 0;
        }
        if old_index != self.cursor_parameter_index {
            self.close_autocompletion();
        } else {
            self.filter_autocompletion_list();
        }
    }

    fn close_autocompletion(&mut self) {
        self.autocompletion_open = None;
        self.autocompletion_index = 0;
        self.full_autocompletion_list.clear();
        self.filtered_autocompletion_list.clear();
    }

    pub fn filter_autocompletion_list(&mut self) {
        if self.autocompletion_open.is_some() {
            let param = self.args.args.get(self.cursor_parameter_index);
            let current_word = param
                .map(|param| {
                    let filtering_chars = if param.start_pos > self.cursor_x {
                        self.cursor_x
                    } else {
                        self.cursor_x - param.start_pos
                    };
                    param.text.chars().take(filtering_chars).collect()
                })
                .unwrap_or("".to_owned());
            self.filtered_autocompletion_list = if current_word.is_empty() {
                self.full_autocompletion_list.clone()
            } else {
                let mut filtered_and_sorted: Vec<_> = self
                    .full_autocompletion_list
                    .iter()
                    .map(|text| {
                        let matc = sublime_fuzzy::best_match(&current_word, text);
                        (matc, text)
                    })
                    .filter(|(matc, _text)| matc.is_some())
                    .collect();
                filtered_and_sorted.sort_by(|(matc1, _), (matc2, _)| {
                    matc2
                        .as_ref()
                        .unwrap()
                        .score()
                        .cmp(&matc1.as_ref().unwrap().score())
                });
                filtered_and_sorted
                    .drain(..)
                    .map(|(_matc, text)| text.clone())
                    .collect()
            };
            if self.filtered_autocompletion_list.is_empty() {
                self.close_autocompletion();
            } else {
                self.autocompletion_index = self
                    .autocompletion_index
                    .min(self.filtered_autocompletion_list.len() - 1)
            }
        }
    }

    pub fn print(&mut self, text: &str) {
        self.rows
            .push(ConsoleEntry::new().add(text, ConsoleWordType::Normal));
    }

    pub fn add_entry(&mut self, entry: ConsoleEntry) {
        self.rows.push(entry);
    }

    pub fn error(&mut self, text: &str) {
        self.rows
            .push(ConsoleEntry::new().add(text, ConsoleWordType::Error));
    }

    fn handle_backspace(
        &mut self,
        input: &HumanInputComponent,
        now: GameTime<Local>,
        repeat_time: f32,
    ) {
        let (new_input, new_x) = if input.ctrl_down || input.ctrl_down {
            // find first non-alpha character
            let prev_char_is_space = self
                .input
                .chars()
                .nth(self.cursor_x - 1)
                .unwrap()
                .is_whitespace();
            let predicate = |ch: char| -> bool {
                if prev_char_is_space {
                    !ch.is_whitespace()
                } else {
                    ch.is_whitespace()
                }
            };

            let idx = ConsoleComponent::get_byte_pos(&self.input, self.cursor_x);
            let (new_input, new_x) =
                if let Some(ix) = self.input[0..idx].chars().rev().position(predicate) {
                    (
                        (self
                            .input
                            .chars()
                            .take(self.cursor_x - ix)
                            .collect::<String>()
                            + &self.input.chars().skip(self.cursor_x).collect::<String>()),
                        self.cursor_x - ix,
                    )
                } else {
                    // not found, remove everything
                    ("".to_owned(), 0)
                };
            (new_input, new_x)
        } else {
            if self.cursor_x >= self.input.chars().count() {
                self.input.pop();
            } else {
                let idx = ConsoleComponent::get_byte_pos(&self.input, self.cursor_x - 1);
                self.input.remove(idx);
            }
            (self.input.clone(), self.cursor_x - 1)
        };
        self.set_input_and_cursor_x(new_x, new_input);
        self.key_repeat_allowed_at = now.add_seconds(repeat_time);
    }

    fn handle_delete_key(
        &mut self,
        input: &HumanInputComponent,
        now: GameTime<Local>,
        repeat_time: f32,
    ) {
        let new_input = if input.ctrl_down || input.ctrl_down {
            // find first non-alpha character
            let next_char_is_space = self
                .input
                .chars()
                .nth(self.cursor_x)
                .unwrap()
                .is_whitespace();
            let predicate = |ch: char| -> bool {
                if next_char_is_space {
                    !ch.is_whitespace()
                } else {
                    ch.is_whitespace()
                }
            };
            let idx = ConsoleComponent::get_byte_pos(&self.input, self.cursor_x);
            if let Some(ix) = self.input[idx..].chars().position(predicate) {
                self.input.chars().take(self.cursor_x).collect::<String>()
                    + &self
                        .input
                        .chars()
                        .skip(self.cursor_x + ix)
                        .collect::<String>()
            } else {
                // not found, remove everything after the cursor
                self.input.chars().take(self.cursor_x).collect::<String>()
            }
        } else {
            if self.cursor_x >= self.input.chars().count() - 1 {
                self.input.pop();
            } else {
                let idx = ConsoleComponent::get_byte_pos(&self.input, self.cursor_x);
                self.input.remove(idx);
            }
            self.input.clone()
        };
        self.set_input(new_input);
        self.key_repeat_allowed_at = now.add_seconds(repeat_time);
    }

    fn get_byte_pos(text: &str, index: usize) -> usize {
        text.char_indices()
            .nth(index)
            .unwrap_or((text.len(), '0'))
            .0
    }

    fn handle_right_cursor(&mut self, input: &HumanInputComponent) {
        if input.ctrl_down || input.ctrl_down {
            // find first non-alpha character
            let next_char_is_space = self
                .input
                .chars()
                .nth(self.cursor_x)
                .unwrap()
                .is_whitespace();
            let predicate = |ch: char| -> bool {
                if next_char_is_space {
                    !ch.is_whitespace()
                } else {
                    ch.is_whitespace()
                }
            };
            let idx = ConsoleComponent::get_byte_pos(&self.input, self.cursor_x);
            if let Some(ix) = self.input[idx..].chars().position(predicate) {
                self.cursor_x += ix;
            } else {
                // not found, jump to the end
                self.cursor_x = self.input.chars().count();
            }
        } else {
            self.cursor_x = (self.cursor_x + 1).min(self.input.chars().count());
        }
        self.set_cursor_x(self.cursor_x);
    }

    fn handle_left_cursor(&mut self, input: &HumanInputComponent) {
        if input.ctrl_down || input.ctrl_down {
            // find first non-alpha character
            let prev_char_is_space = self
                .input
                .chars()
                .nth(self.cursor_x - 1)
                .unwrap()
                .is_whitespace();
            let predicate = |ch: char| -> bool {
                if prev_char_is_space {
                    !ch.is_whitespace()
                } else {
                    ch.is_whitespace()
                }
            };

            let idx = ConsoleComponent::get_byte_pos(&self.input, self.cursor_x);
            if let Some(ix) = self.input[0..idx].chars().rev().position(predicate) {
                self.cursor_x -= ix;
            } else {
                // not found, jump to the beginning
                self.cursor_x = 0;
            }
        } else {
            self.cursor_x = (self.cursor_x - 1).max(0);
        }
        self.set_cursor_x(self.cursor_x);
    }

    fn insert_str_to_prompt(
        &mut self,
        text: &str,
        static_char_storage: &ReadStorage<StaticCharDataComponent>,
        command_defs: &HashMap<String, CommandDefinition>,
    ) {
        let idx = ConsoleComponent::get_byte_pos(&self.input, self.cursor_x);
        self.input.insert_str(idx, text);
        self.set_input_and_cursor_x(self.cursor_x + 1, self.input.clone());
        if self.autocompletion_open.is_none() {
            let autocompletion_type = if self.cursor_parameter_index == 0 {
                AutocompletionType::CommandName
            } else {
                AutocompletionType::Param
            };
            self.open_autocompletion(autocompletion_type, static_char_storage, command_defs);
        }
    }

    fn autocompletion_selected(
        &mut self,
        close_autocompletion: bool,
        autocompletion_by_pressing_enter: bool,
        command_defs: &HashMap<String, CommandDefinition>,
    ) {
        let mut arg = CommandArguments::new(&self.input);
        let selected_text = &self.filtered_autocompletion_list[self.autocompletion_index];
        let is_parameter_completion = Some(AutocompletionType::Param) == self.autocompletion_open;
        let (selected_text, quoted) = if is_parameter_completion && selected_text.contains(" ") {
            (format!("\"{}\"", selected_text), true)
        } else {
            (selected_text.clone(), false)
        };
        let end_pos = if self.cursor_parameter_index == 0 {
            selected_text.chars().count() + 1
        } else {
            arg.args[self.cursor_parameter_index - 1].end_pos + 2 + selected_text.chars().count()
        };
        let new_input = if self.autocompletion_open == Some(AutocompletionType::CommandHistory) {
            selected_text
        } else {
            if arg.args.len() < self.cursor_parameter_index + 1 {
                arg.args.push(CommandElement {
                    text: selected_text,
                    start_pos: 0,
                    end_pos,
                    qouted: quoted,
                });
            } else {
                arg.args[self.cursor_parameter_index].text = selected_text;
            }
            let mut new_input = arg
                .args
                .iter()
                .map(|it| it.text.as_str())
                .collect::<Vec<&str>>()
                .join(" ");
            if end_pos > new_input.len() {
                new_input += " ";
            }
            new_input
        };
        if close_autocompletion {
            self.set_input_and_cursor_x(end_pos.min(new_input.chars().count()), new_input);
            self.close_autocompletion();
            let command_def = command_defs.get(arg.get_command_name().unwrap_or(&"".to_owned()));
            if let Some(command_def) = command_def {
                let has_no_param = command_def.arguments.is_empty();
                let last_param_was_completed =
                    command_def.arguments.len() <= self.cursor_parameter_index;
                if (has_no_param || last_param_was_completed) && autocompletion_by_pressing_enter {
                    // execute the command immediately if it does not have any parameters,
                    // or the last parameter was autocompleted
                    // and autocompletion was done by pressing enter
                    self.input_added(false, command_defs);
                }
            }
        } else {
            self.set_input(new_input);
        }
    }

    fn input_added(
        &mut self,
        keep_input_prompt: bool,
        command_defs: &HashMap<String, CommandDefinition>,
    ) {
        let input = self.input.trim().to_owned();
        let args = CommandArguments::new(&input);
        let command_def = command_defs.get(args.get_command_name().unwrap_or(&"".to_owned()));
        self.add_entry(ConsoleSystem::create_console_entry(&args, command_def));
        self.command_history.push(input);
        if !keep_input_prompt {
            self.set_input_and_cursor_x(0, String::with_capacity(32));
        }
        self.history_pos = 0;
        // validate input
        if let Some(command_def) = command_def {
            let mandatory_arg_count = command_def.arguments.iter().take_while(|it| it.2).count();
            let actual_arg_count = args.args.len() - 1;
            if actual_arg_count < mandatory_arg_count
                || actual_arg_count > command_def.arguments.len()
            {
                self.error(&format!(
                    "Illegal number of parameters (expected at least {}, at most {}, provided {})",
                    mandatory_arg_count,
                    command_def.arguments.len(),
                    actual_arg_count
                ));
                return;
            }

            let ok = command_def.arguments.iter().enumerate().all(
                |(i, (param_name, arg_type, mandatory))| {
                    let ok = match arg_type {
                        CommandParamType::Float => args
                            .as_str(i)
                            .map(|it| it.parse::<f32>().is_ok())
                            .unwrap_or(!*mandatory),
                        CommandParamType::Int => args
                            .as_str(i)
                            .map(|it| it.parse::<i32>().is_ok())
                            .unwrap_or(!*mandatory),
                        CommandParamType::String => true,
                    };
                    if !ok {
                        self.error(&format!(
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
                self.command_to_execute = Some(args);
            }
        } else {
            self.error("Unknown command")
        }
    }

    fn open_autocompletion(
        &mut self,
        autocompletion: AutocompletionType,
        static_char_storage: &ReadStorage<StaticCharDataComponent>,
        command_defs: &HashMap<String, CommandDefinition>,
    ) {
        match autocompletion {
            AutocompletionType::CommandName => {
                // list commands
                self.full_autocompletion_list = command_defs
                    .keys()
                    .map(|it| it.to_owned())
                    .collect::<Vec<String>>();
                self.filtered_autocompletion_list = self.full_autocompletion_list.clone();
            }
            AutocompletionType::Param => {
                let command_def = command_defs.get(
                    CommandArguments::new(&self.input)
                        .get_command_name()
                        .unwrap_or(&"".to_owned()),
                );

                if let Some(list) = command_def.and_then(|it| {
                    if it.arguments.len() < self.cursor_parameter_index {
                        None
                    } else {
                        it.autocompletion.get_autocompletion_list(
                            self.cursor_parameter_index - 1,
                            static_char_storage,
                        )
                    }
                }) {
                    self.full_autocompletion_list = list;
                    self.filtered_autocompletion_list = self.full_autocompletion_list.clone();
                }
            }
            AutocompletionType::CommandHistory => {
                self.full_autocompletion_list = self.command_history.clone();
                self.filtered_autocompletion_list = self.full_autocompletion_list.clone();
            }
        }
        self.autocompletion_open = if self.full_autocompletion_list.is_empty() {
            None
        } else {
            Some(autocompletion)
        };
        self.filter_autocompletion_list();
    }
}

impl ConsoleSystem {
    fn create_console_entry(
        args: &CommandArguments,
        command_def: Option<&CommandDefinition>,
    ) -> ConsoleEntry {
        let mut entry = ConsoleEntry::new().add("> ", ConsoleWordType::Normal);
        let name = args.get_command_name().unwrap_or("");
        if !name.is_empty() {
            entry = entry.add(
                name,
                if command_def.is_some() {
                    ConsoleWordType::CommandName
                } else {
                    ConsoleWordType::Error
                },
            );
        }
        let param_str = &args
            .args
            .iter()
            .skip(1)
            .map(|it| it.text.as_str())
            .collect::<Vec<&str>>()
            .join(" ");
        if !param_str.is_empty() {
            entry = entry.add(" ", ConsoleWordType::Normal);
            entry = entry.add(param_str, ConsoleWordType::Param);
        }
        return entry;
    }

    fn render_console_entry(
        render_commands: &mut RenderCommandCollector,
        console_color: &[u8; 4],
        input_row_y: i32,
        row_index: i32,
        row: &ConsoleEntry,
    ) {
        let mut x = 0;
        for words in &row.words {
            render_commands
                .text_2d()
                .screen_pos(
                    3 + x * NORMAL_FONT_W,
                    input_row_y - row_index * NORMAL_FONT_H,
                )
                .color(&match words.typ {
                    ConsoleWordType::Normal => [204, 204, 204, console_color[3]],
                    ConsoleWordType::Error => [255, 0, 0, console_color[3]],
                    ConsoleWordType::CommandName => [128, 255, 128, console_color[3]],
                    ConsoleWordType::Param => [128, 128, 255, console_color[3]],
                })
                .font(Font::Normal)
                .layer(UiLayer2d::ConsoleTexts)
                .add(&words.text);
            x += words.text.chars().count() as i32;
        }
    }

    pub fn get_char_id_by_name(ecs_world: &World, username: &str) -> Option<EntityId<Local>> {
        for (entity_id, char_state) in (
            &ecs_world.entities(),
            &ecs_world.read_storage::<StaticCharDataComponent>(),
        )
            .join()
        {
            if char_state.name == username {
                return Some(EntityId::from(entity_id));
            }
        }
        return None;
    }

    pub fn init_commands(
        _effect_names: Vec<String>,
        _map_names: Vec<String>,
        resolutions: Vec<String>,
    ) -> HashMap<String, CommandDefinition> {
        let mut command_defs: HashMap<String, CommandDefinition> = HashMap::new();
        ConsoleSystem::add_command(&mut command_defs, cmd_set_pos());
        ConsoleSystem::add_command(&mut command_defs, cmd_get_pos());
        ConsoleSystem::add_command(&mut command_defs, cmd_add_status());
        ConsoleSystem::add_command(&mut command_defs, cmd_list_statuses());
        ConsoleSystem::add_command(&mut command_defs, cmd_list_players());
        ConsoleSystem::add_command(&mut command_defs, cmd_set_resolution(resolutions));
        ConsoleSystem::add_command(&mut command_defs, cmd_set_fullscreen());
        ConsoleSystem::add_command(&mut command_defs, cmd_list_entities());
        //        ConsoleSystem::add_command(&mut command_defs, cmd_spawn_effect(effect_names));
        ConsoleSystem::add_command(&mut command_defs, cmd_spawn_area());
        ConsoleSystem::add_command(&mut command_defs, cmd_spawn_entity());
        ConsoleSystem::add_command(&mut command_defs, cmd_reload_configs());
        ConsoleSystem::add_command(&mut command_defs, cmd_heal());
        ConsoleSystem::add_command(&mut command_defs, cmd_kill_all());
        ConsoleSystem::add_command(&mut command_defs, cmd_goto());
        ConsoleSystem::add_command(&mut command_defs, cmd_follow_char());
        ConsoleSystem::add_command(&mut command_defs, cmd_control_char());
        ConsoleSystem::add_command(&mut command_defs, cmd_set_outlook());
        ConsoleSystem::add_command(&mut command_defs, cmd_resurrect());
        ConsoleSystem::add_command(&mut command_defs, cmd_set_team());
        ConsoleSystem::add_command(&mut command_defs, cmd_set_damping());
        ConsoleSystem::add_command(&mut command_defs, cmd_set_mass());
        ConsoleSystem::add_command(&mut command_defs, cmd_clear());
        ConsoleSystem::add_command(&mut command_defs, cmd_add_falcon());
        ConsoleSystem::add_command(&mut command_defs, cmd_remove_falcon());
        ConsoleSystem::add_command(&mut command_defs, cmd_set_job());
        ConsoleSystem::add_command(&mut command_defs, cmd_enable_collision());
        ConsoleSystem::add_command(&mut command_defs, cmd_disable_collision());
        ConsoleSystem::add_command(&mut command_defs, cmd_clone_char());
        ConsoleSystem::add_command(&mut command_defs, cmd_bind_key());
        ConsoleSystem::add_command(&mut command_defs, cmd_toggle_console());
        ConsoleSystem::add_command(&mut command_defs, cmd_inspect());
        ConsoleSystem::add_command(&mut command_defs, cmd_set_config());

        return command_defs;
    }

    fn add_command(defs: &mut HashMap<String, CommandDefinition>, command_def: CommandDefinition) {
        defs.insert(command_def.name.to_owned(), command_def);
    }
}

trait AutocompletionGenerator {
    fn create_autocompletion_list(&self) -> Vec<String>;
}

#[derive(Copy, Clone)]
pub enum CommandParamType {
    String,
    Int,
    Float,
}

pub trait AutocompletionProvider {
    fn get_autocompletion_list(
        &self,
        param_index: usize,
        input: &ReadStorage<StaticCharDataComponent>,
    ) -> Option<Vec<String>>;
}

pub struct OwnedAutocompletionProvider(pub Vec<String>);
impl AutocompletionProvider for OwnedAutocompletionProvider {
    fn get_autocompletion_list(
        &self,
        _param_index: usize,
        _input_storage: &ReadStorage<StaticCharDataComponent>,
    ) -> Option<Vec<String>> {
        Some(self.0.clone())
    }
}

pub struct BasicAutocompletionProvider(Box<dyn Fn(usize) -> Option<Vec<String>>>);

impl BasicAutocompletionProvider {
    pub fn new<F>(callback: F) -> Box<dyn AutocompletionProvider>
    where
        F: Fn(usize) -> Option<Vec<String>> + 'static,
    {
        Box::new(BasicAutocompletionProvider(Box::new(callback)))
    }
}

impl AutocompletionProvider for BasicAutocompletionProvider {
    fn get_autocompletion_list(
        &self,
        param_index: usize,
        _input_storage: &ReadStorage<StaticCharDataComponent>,
    ) -> Option<Vec<String>> {
        (self.0)(param_index)
    }
}

pub struct AutocompletionProviderWithUsernameCompletion(
    Box<
        dyn Fn(
            usize,
            Box<dyn Fn(&ReadStorage<StaticCharDataComponent>) -> Vec<String>>,
            &ReadStorage<StaticCharDataComponent>,
        ) -> Option<Vec<String>>,
    >,
);

impl AutocompletionProviderWithUsernameCompletion {
    pub fn new<F>(callback: F) -> Box<dyn AutocompletionProvider>
    where
        F: Fn(
                usize,
                Box<dyn Fn(&ReadStorage<StaticCharDataComponent>) -> Vec<String>>,
                &ReadStorage<StaticCharDataComponent>,
            ) -> Option<Vec<String>>
            + 'static,
    {
        Box::new(AutocompletionProviderWithUsernameCompletion(Box::new(
            callback,
        )))
    }
}

impl AutocompletionProvider for AutocompletionProviderWithUsernameCompletion {
    fn get_autocompletion_list(
        &self,
        param_index: usize,
        input: &ReadStorage<StaticCharDataComponent>,
    ) -> Option<Vec<String>> {
        let username_completor: Box<dyn Fn(&ReadStorage<StaticCharDataComponent>) -> Vec<String>> =
            Box::new(|static_char_data_storage| {
                let mut usernames = Vec::with_capacity(8);
                for static_char_data in static_char_data_storage.join() {
                    if static_char_data.typ == CharType::Player {
                        usernames.push(static_char_data.name.clone());
                    }
                }
                usernames
            });
        (self.0)(param_index, username_completor, input)
    }
}

pub struct CommandDefinition {
    pub name: String,
    pub arguments: Vec<(&'static str, CommandParamType, bool)>, // name, type, mandatory
    pub action: CommandCallback,
    pub autocompletion: Box<dyn AutocompletionProvider>,
}

pub type CommandCallback = Box<
    dyn Fn(Option<EntityId<Local>>, CommandArguments, &mut World, &mut Video) -> Result<(), String>,
>;

pub enum ConsoleWordType {
    Normal,
    CommandName,
    Param,
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

    pub fn add(mut self, text: &str, typ: ConsoleWordType) -> ConsoleEntry {
        self.words.push(ConsoleWords::new(text, typ));
        self
    }
}

pub struct ConsoleEntry {
    words: Vec<ConsoleWords>,
}

pub struct ConsoleSystem;

impl ConsoleSystem {
    pub fn run(
        command_defs: &HashMap<String, CommandDefinition>,
        input: &HumanInputComponent,
        static_char_storage: &ReadStorage<StaticCharDataComponent>,
        console: &mut ConsoleComponent,
        sys_vars: &SystemVariables,
        time: &EngineTime,
    ) {
        let now = time.now();
        let console_height = (sys_vars.matrices.resolution_h / 3) as i32;
        let repeat_time = 0.1;
        if !input.is_console_open {
            if console.y_pos > 0 {
                console.y_pos -= 12;
            }
        } else {
            if console.y_pos < console_height {
                console.y_pos += 12;
            }
            if console.cursor_change.has_already_passed(time.now()) {
                console.cursor_shown = !console.cursor_shown;
                console.cursor_change = time.now().add_seconds(0.5);
            }

            if input.is_key_just_pressed(Scancode::Up) {
                if console.autocompletion_open.is_some() {
                    if console.autocompletion_index > 0 {
                        console.autocompletion_index -= 1;
                    } else {
                        console.autocompletion_index =
                            console.filtered_autocompletion_list.len() - 1;
                    }
                    console.autocompletion_selected(false, false, &command_defs);
                } else {
                    if console.history_pos < console.command_history.len() {
                        console.history_pos += 1;
                    }
                    let idx = console.command_history.len() - console.history_pos;
                    let new_input = console
                        .command_history
                        .get(idx)
                        .unwrap_or(&"".to_owned())
                        .clone();
                    console.set_input_and_cursor_x(new_input.chars().count(), new_input);
                }
            } else if input.is_key_just_pressed(Scancode::Down) {
                if console.autocompletion_open.is_some() {
                    if console.autocompletion_index < console.filtered_autocompletion_list.len() - 1
                    {
                        console.autocompletion_index += 1;
                    } else {
                        console.autocompletion_index = 0;
                    }
                    console.autocompletion_selected(false, false, &command_defs);
                } else {
                    if console.history_pos > 1 {
                        console.history_pos -= 1;
                        let idx = console.command_history.len() - console.history_pos;
                        let new_input = console.command_history[idx].clone();
                        console.set_input_and_cursor_x(new_input.chars().count(), new_input);
                    } else {
                        console.history_pos = 0;
                        console.set_input_and_cursor_x(0, String::with_capacity(32));
                    }
                }
            } else if input.is_key_down(Scancode::Left)
                && console.cursor_x > 0
                && console.key_repeat_allowed_at.has_already_passed(now)
            {
                console.handle_left_cursor(&input);
                console.key_repeat_allowed_at = now.add_seconds(repeat_time);
            } else if input.is_key_down(Scancode::Right)
                && console.cursor_x < console.input.chars().count()
                && console.key_repeat_allowed_at.has_already_passed(now)
            {
                console.handle_right_cursor(&input);
                console.key_repeat_allowed_at = now.add_seconds(repeat_time);
            } else if input.is_key_down(Scancode::Home) {
                console.set_cursor_x(0);
            } else if input.is_key_down(Scancode::End) {
                console.set_cursor_x(console.input.chars().count());
            } else if input.is_key_down(Scancode::Backspace)
                && console.cursor_x > 0
                && console.key_repeat_allowed_at.has_already_passed(now)
            {
                console.handle_backspace(&input, now, repeat_time);
            } else if input.is_key_down(Scancode::Delete)
                && console.cursor_x < console.input.chars().count()
                && console.key_repeat_allowed_at.has_already_passed(now)
            {
                console.handle_delete_key(&input, now, repeat_time);
            } else if input.ctrl_down && input.is_key_just_released(Scancode::Space) {
                let autocompletion_type = if console.cursor_parameter_index == 0 {
                    AutocompletionType::CommandName
                } else {
                    AutocompletionType::Param
                };
                console.open_autocompletion(
                    autocompletion_type,
                    static_char_storage,
                    &command_defs,
                );
            } else if (input.is_key_just_released(Scancode::Space)
                || input.is_key_just_released(Scancode::Tab)
                || (input.is_key_just_released(Scancode::Return)) && !input.ctrl_down)
                && console.autocompletion_open.is_some()
            {
                console.autocompletion_selected(
                    !input.ctrl_down,
                    input.is_key_just_released(Scancode::Return),
                    &command_defs,
                );
                if let Some(command_def) =
                    command_defs.get(console.args.get_command_name().unwrap_or(&"".to_owned()))
                {
                    if command_def
                        .arguments
                        .get((console.cursor_parameter_index as i32 - 1).max(0) as usize)
                        .map(|it| it.2)
                        .unwrap_or(false)
                    {
                        // if there is next command and it is mandatory
                        console.open_autocompletion(
                            AutocompletionType::Param,
                            &static_char_storage,
                            &command_defs,
                        );
                    }
                }
            } else if input.ctrl_down && input.is_key_just_released(Scancode::R) {
                console.set_input_and_cursor_x(0, "".to_owned());
                console.open_autocompletion(
                    AutocompletionType::CommandHistory,
                    &static_char_storage,
                    &command_defs,
                );
            } else if input.is_key_just_released(Scancode::Space) {
                if console.cursor_inside_quotes
                    || (console.cursor_x > 0
                        && !console
                            .input
                            .chars()
                            .nth(console.cursor_x - 1)
                            .unwrap_or('x')
                            .is_whitespace())
                {
                    console.insert_str_to_prompt(" ", &static_char_storage, &command_defs)
                }
            } else if !input.text.is_empty() && !input.is_key_down(Scancode::Space) {
                // spaces are handled above, because typing space can open the autocompletion, then
                // releasing it can choose the first option immediately
                // two spaces are not allowed
                console.insert_str_to_prompt(&input.text, &static_char_storage, &command_defs)
            } else if input.is_key_just_released(Scancode::Escape)
                && console.autocompletion_open.is_some()
            {
                console.close_autocompletion();
            } else if input.is_key_just_released(Scancode::Return) {
                console.input_added(input.ctrl_down, &command_defs)
            }
        }
    }
}

pub struct ConsoleRenderSystem;

impl ConsoleRenderSystem {
    pub fn run(
        console: &ConsoleComponent,
        render_commands: &mut RenderCommandCollector,
        sys_vars: &SystemVariables,
        command_defs: &HashMap<String, CommandDefinition>,
    ) {
        let console_color = [0, 0, 0, 179];
        let console_height = (sys_vars.matrices.resolution_h / 3) as i32;
        let repeat_time = 0.1;

        if console.y_pos <= 0 {
            return;
        }
        // background
        render_commands
            .rectangle_2d()
            .screen_pos(0, 0)
            .size(sys_vars.matrices.resolution_w as u16, console.y_pos as u16)
            .color(&console_color)
            .layer(UiLayer2d::Console)
            .add();
        // cursor
        if console.cursor_shown {
            render_commands
                .text_2d()
                .screen_pos(
                    3 + 2 * NORMAL_FONT_W + console.cursor_x as i32 * NORMAL_FONT_W
                        - NORMAL_FONT_W / 2,
                    console.y_pos - NORMAL_FONT_H - 3,
                )
                .color(&[255, 255, 255, console_color[3]])
                .font(Font::Normal)
                .layer(UiLayer2d::ConsoleTexts)
                .add("|")
        }

        // draw history
        let row_count = console_height / NORMAL_FONT_H;
        let input_row_y = console.y_pos - NORMAL_FONT_H - 3;
        for (i, row) in console
            .rows
            .iter()
            .rev()
            .take(row_count as usize)
            .enumerate()
        {
            let row_index = 1 + i as i32;
            ConsoleSystem::render_console_entry(
                render_commands,
                &console_color,
                input_row_y,
                row_index,
                &row,
            )
        }

        // input prompt
        let command_def =
            command_defs.get(console.args.get_command_name().unwrap_or(&"".to_owned()));
        let entry = ConsoleSystem::create_console_entry(&console.args, command_def);

        ConsoleSystem::render_console_entry(
            render_commands,
            &console_color,
            input_row_y,
            0,
            &entry,
        );
        if let Some(command_def) = command_def {
            if !command_def.arguments.is_empty() {
                let border_size = 3;
                // draw help prompt above the cursor
                let help_text_len: usize = command_def
                        .arguments
                        .iter()
                        .map(|it| it.0.chars().count())
                        .sum::<usize>()
                        + command_def.arguments.len() // spaces
                        - 1;
                let start_x =
                    ((console.cursor_x as i32 - help_text_len as i32 / 2) * NORMAL_FONT_W).max(0);
                // background
                render_commands
                    .rectangle_2d()
                    .screen_pos(start_x, console.y_pos - NORMAL_FONT_H * 2 - 3)
                    .size(
                        help_text_len as u16 * NORMAL_FONT_W as u16 + border_size as u16 * 2,
                        NORMAL_FONT_H as u16 + border_size as u16 * 2,
                    )
                    .color(&[55, 57, 57, console_color[3]])
                    .layer(UiLayer2d::ConsoleAutocompletion)
                    .add();
                // text
                let mut x: usize = border_size as usize;
                command_def
                    .arguments
                    .iter()
                    .map(|it| it.0.to_owned())
                    .enumerate()
                    .for_each(|(i, param_name)| {
                        let color = if console.cursor_parameter_index as i32 - 1 == i as i32 {
                            [255, 255, 255, console_color[3]] // active argument
                        } else {
                            [0, 0, 0, console_color[3]]
                        };
                        render_commands
                            .text_2d()
                            .screen_pos(
                                start_x + x as i32,
                                console.y_pos - NORMAL_FONT_H * 2 - 3 + border_size,
                            )
                            .color(&color)
                            .font(Font::Normal)
                            .layer(UiLayer2d::ConsoleAutocompletion)
                            .add(&param_name);
                        x += (param_name.chars().count() + 1) * NORMAL_FONT_W as usize;
                    });
            }
        }
        // autocompletion
        if console.autocompletion_open.is_some() {
            let longest_text_len: usize = console
                .filtered_autocompletion_list
                .iter()
                .take(20)
                .map(|it| it.chars().count())
                .max()
                .unwrap_or(1);
            let start_x =
                ((console.cursor_x as i32 - longest_text_len as i32 / 3) * NORMAL_FONT_W).max(0);
            // background
            render_commands
                .rectangle_2d()
                .screen_pos(start_x, console.y_pos)
                .size(
                    longest_text_len as u16 * NORMAL_FONT_W as u16,
                    NORMAL_FONT_H as u16
                        * console.filtered_autocompletion_list.iter().take(20).count() as u16,
                )
                .color(&[55, 57, 57, console_color[3]])
                .layer(UiLayer2d::ConsoleAutocompletion)
                .add();
            // texts
            for (i, line) in console
                .filtered_autocompletion_list
                .iter()
                .take(20)
                .enumerate()
            {
                let color = if i == console.autocompletion_index {
                    [255, 255, 255, console_color[3]] // active argument
                } else {
                    [0, 0, 0, console_color[3]]
                };
                render_commands
                    .text_2d()
                    .screen_pos(start_x, console.y_pos + NORMAL_FONT_H * i as i32)
                    .color(&color)
                    .font(Font::Normal)
                    .layer(UiLayer2d::ConsoleAutocompletion)
                    .add(line);
            }
        }
    }
}
