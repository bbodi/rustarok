use serde::Deserialize;
use serde::Serialize;
use std::fmt::Debug;
use std::fmt::Error;
use std::fmt::Formatter;

#[derive(Clone, Serialize, Deserialize)]
pub struct CommandArguments {
    pub args: Vec<CommandElement>,
}

impl Debug for CommandArguments {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(
            f,
            "Command({})",
            self.args
                .iter()
                .map(|it| it.text.as_str())
                .collect::<Vec<&str>>()
                .join(",")
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandElement {
    pub text: String,
    pub start_pos: usize,
    pub end_pos: usize,
    pub qouted: bool,
}

impl CommandArguments {
    pub fn new(text: &str) -> CommandArguments {
        let mut args = Vec::with_capacity(3);
        let mut qoute_started = false;
        let mut current_str = String::with_capacity(12);
        let mut text_started = false;
        let mut start_pos = 0;
        for (i, ch) in text.chars().enumerate() {
            let push = if ch == '"' && qoute_started {
                current_str.push(ch);
                true
            } else if ch == '"' && !qoute_started {
                qoute_started = true;
                text_started = true;
                start_pos = i;
                current_str.push(ch);
                false
            } else if !ch.is_whitespace() && !text_started {
                text_started = true;
                start_pos = i;
                current_str.push(ch);
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
                args.push(CommandElement {
                    text: current_str,
                    start_pos,
                    end_pos: i,
                    qouted: qoute_started,
                });
                current_str = String::with_capacity(12);
                text_started = false;
                qoute_started = false;
            }
        }
        if !current_str.is_empty() {
            // push the last param
            let len = current_str.chars().count();
            args.push(CommandElement {
                text: current_str,
                start_pos,
                end_pos: start_pos + len,
                qouted: qoute_started,
            });
        }
        CommandArguments { args }
    }

    pub fn is_cursor_inside_arg(&self, index: usize, cursor_x: usize) -> bool {
        if let Some(arg) = self.args.get(index + 1) {
            cursor_x >= arg.start_pos && cursor_x <= arg.end_pos
        } else {
            // there is no entry for this arg, check if we are after the prev arg
            let end_of_prev_arg = self.args.get(index).map(|it| it.end_pos);
            if let Some(end_of_prev_arg) = end_of_prev_arg {
                end_of_prev_arg < cursor_x
            } else {
                false
            }
        }
    }

    pub fn get_command_name(&self) -> Option<&str> {
        self.args.get(0).map(|it| it.text.as_str())
    }
    // first argument is the command name!
    pub fn as_int(&self, index: usize) -> Option<i32> {
        self.args.get(index + 1).and_then(|it| {
            if it.qouted {
                it.text[1..it.text.len() - 1].parse().ok()
            } else {
                it.text.parse().ok()
            }
        })
    }

    pub fn as_f32(&self, index: usize) -> Option<f32> {
        self.args.get(index + 1).and_then(|it| {
            if it.qouted {
                it.text[1..it.text.len() - 1].parse().ok()
            } else {
                it.text.parse().ok()
            }
        })
    }

    pub fn as_str(&self, index: usize) -> Option<&str> {
        self.args.get(index + 1).map(|it| {
            if it.qouted {
                if it.text.as_bytes()[it.text.len() - 1] == '\"' as u8 {
                    &it.text[1..it.text.len() - 1]
                } else {
                    &it.text[1..it.text.len()]
                }
            } else {
                it.text.as_str()
            }
        })
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
