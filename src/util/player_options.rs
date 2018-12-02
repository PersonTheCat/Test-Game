use std::iter::FromIterator;

use timed_events::{ DelayHandler, DelayedEvent };
use var_access::{ self, EntityAccessor };
use rand::{ thread_rng, random, Rng };
use messages::MessageComponent::*;
use text;

use self::DialogueOption::*;
use self::DialogueResult::*;

pub const GLOBAL_USER: usize = 01001010100101010;

pub static mut CURRENT_OPTIONS: Option<Vec<Dialogue>> = None;

pub unsafe fn setup_option_registry()
{
    CURRENT_OPTIONS = Some(Vec::new());
}

pub fn register_options(options: Dialogue)
{
    unsafe { match CURRENT_OPTIONS
    {
        Some(ref mut o) =>
        {
            o.push(options);
        },
        None => panic!("Error: Options registry not yet initialized.")
    }; }
}

pub fn delete_options(option_id: usize) -> bool
{
    unsafe { match CURRENT_OPTIONS
    {
        Some(ref mut o) =>
        {
            let index = o.iter()
                .position(| option | option.id == option_id);

            if let Some(num) = index
            {
                if o[num].player_id == GLOBAL_USER
                {
                    return false;
                }
                o.remove(num);
                return true;
            }
        },
        None => panic!("Error: Options registry not yet initialized.")
    }; }
    false
}

/**
 * A variant of delete_options() which will only
 * succeed when the player has exactly one dialogue.
 */
pub fn try_delete_options(player_id: usize) -> Result<Dialogue, &'static str>
{
    unsafe { match CURRENT_OPTIONS
    {
        Some(ref mut o) =>
        {
            let mut matches = Vec::new();
            let mut index = 0;
            for dialogue in o.iter()
            {
                if dialogue.player_id == player_id
                {
                    matches.push(index);
                }
                index += 1;
            }
            if matches.len() == 1
            {
                Ok(o.remove(matches[0]))
            }
            else { Err("Multiple dialogues were found. Not sure which to remove.") }
        },
        None => panic!("Error: Options registry not yet initialized.")
    }}
}

pub fn remove_all_options(player_id: usize) -> Vec<Dialogue>
{
    unsafe { match CURRENT_OPTIONS
    {
        Some(ref mut o) =>
        {
            let filter =
                o.drain_filter(| d |{ d.player_id == player_id });

            return Vec::from_iter(filter)
        },
        None => panic!("Error: Options registry not yet initialized.")
    }}
}

pub fn send_current_options(to_player: usize)
{
    unsafe { if let Some(ref registry) = CURRENT_OPTIONS
    {
        let mut options_text = String::new();
        let mut first_response = 1;

        for option in registry.iter()
        {
            if option.player_id == to_player
            {
                options_text += "\n";
                options_text += &option.get_display(first_response);
            }
            first_response += option.responses.len();
        }
        ::send_message_to_player(to_player, Options, &options_text, 0);
    }
    else { panic!("Error: Option registry not established in time."); }}
}

pub fn update_options_manually(for_player: usize)
{
    unsafe { if let Some(ref registry) = CURRENT_OPTIONS
    {
        let mut options_text = String::new();
        let mut first_response = 1;

        for option in registry.iter()
        {
            if option.player_id == for_player
            {
                options_text += "\n";
                options_text += &option.get_display(first_response);
            }
            first_response += option.responses.len();
        }
        ::update_player_message(for_player, Options, &options_text);
    }
    else { panic!("Error: Option registry not established in time."); }}
}

pub fn get_send_area_options(player_id: usize)
{
    let new_options = var_access::access_player_context(player_id, | player, _, area, _ |
    {
        area._get_dialogue_for_player(player)
    })
    .expect("Player data no longer exists.");

    register_options(new_options);
    send_current_options(player_id);
}

pub fn replace_send_options(player_id: usize, old_options: usize, new_options: Dialogue)
{
    delete_options(old_options);
    register_options(new_options);
    send_current_options(player_id);
}

pub fn replace_no_send_options(player_id: usize, old_options: usize, new_options: Dialogue)
{
    delete_options(old_options);
    register_options(new_options);
    update_options_manually(player_id);
}

pub enum DialogueResult
{
    Success,
    InvalidNumber(usize),
    NoneFound,
    NoArgs
}

pub enum DialogueOption
{
    FromArea,
    Ignore,
    Delete, // Dialogue will be automatically resent; Don't double-do it.
    Generate(Box<Fn() -> Dialogue>)
}

/** Shorthand */
pub fn gen_dialogue<F>(run: F) -> DialogueOption
    where F: Fn() -> Dialogue + 'static
{
    Generate(Box::new(run))
}

pub struct Dialogue
{
    pub title: String,
    pub text: Option<String>,
    pub info: Option<String>,
    pub responses: Vec<Response>,
    pub commands: Vec<Command>,
    pub text_handler: Option<TextHandler>,
    pub player_id: usize,
    pub id: usize
}

impl Dialogue
{
    pub fn new
    (
        title: String,
        text: &[&str],
        replacements: Vec<(&'static str, String)>,
        info: Option<String>,
        responses: Vec<Response>,
        commands: Vec<Command>,
        text_handler: Option<TextHandler>,
        player_id: usize
    )
        -> Dialogue
    {
        let text = match thread_rng().choose(text)
        {
            Some(txt) => Some(text::apply_replacements(&String::from(*txt), &replacements)),
            None => None
        };

        Dialogue
        {
            title,
            text,
            info,
            responses,
            commands,
            text_handler,
            player_id,
            id: random()
        }
    }

    /**
     * Try to remove this.
     */
    pub fn new_2
    (
        title: String,
        text: Vec<String>,
        replacements: Vec<(&'static str, String)>,
        info: Option<String>,
        responses: Vec<Response>,
        commands: Vec<Command>,
        text_handler: Option<TextHandler>,
        player_id: usize
    )
        -> Dialogue
    {
        let text = match thread_rng().choose(&text)
        {
            Some(txt) => Some(text::apply_replacements(txt, &replacements)),
            None => None
        };

        Dialogue
        {
            title,
            text,
            info,
            responses,
            commands,
            text_handler,
            player_id,
            id: random()
        }
    }

    pub fn simple
    (
        title: String,
        text: &[&str],
        replacements: Vec<(&'static str, String)>,
        responses: Vec<Response>,
        player_id: usize
    )
        -> Dialogue
    {
        let text = match thread_rng().choose(text)
        {
            Some(txt) => Some(text::apply_replacements(&String::from(*txt), &replacements)),
            None => None
        };

        Dialogue
        {
            title,
            text,
            info: None,
            responses,
            commands: Vec::new(),
            text_handler: None,
            player_id,
            id: random()
        }
    }

    /**
     * Try to remove this.
     */
    pub fn simple_2
    (
        title: String,
        text: Vec<String>,
        replacements: Vec<(&'static str, String)>,
        responses: Vec<Response>,
        player_id: usize
    )
        -> Dialogue
    {
        let text = match thread_rng().choose(&text)
        {
            Some(txt) => Some(text::apply_replacements(txt, &replacements)),
            None => None
        };

        Dialogue
        {
            title,
            text,
            info: None,
            responses,
            commands: Vec::new(),
            text_handler: None,
            player_id,
            id: random()
        }
    }

    pub fn handle_text
    (
        title: String,
        text: &[&str],
        replacements: Vec<(&'static str, String)>,
        text_handler: TextHandler,
        player_id: usize
    )
        -> Dialogue
    {
        let text = match thread_rng().choose(text)
        {
            Some(txt) => Some(text::apply_replacements(*txt, &replacements)),
            None => None
        };

        Dialogue
        {
            title,
            text,
            info: None,
            responses: Vec::new(),
            commands: Vec::new(),
            text_handler: Some(text_handler),
            player_id,
            id: random()
        }
    }

    pub fn confirm_action<F1, F2>(player_id: usize, temporary: bool, on_yes: F1, on_no: F2) -> Dialogue
        where F1: Fn(usize) + 'static, F2: Fn(usize) + 'static
    {
        let id = random();
        let mut responses = Vec::new();

        responses.push(Response::delete_dialogue("Yes", on_yes));
        responses.push(Response::delete_dialogue("No", on_no));

        if temporary
        {
            Dialogue::delete_in(player_id, id, ::TEMP_DIALOGUE_DURATION);
        }

        Dialogue
        {
            title: String::from("Confirm Action"),
            text: None,
            info: Some(String::from("Are you sure?")),
            responses,
            commands: Vec::new(),
            text_handler: None,
            player_id,
            id
        }
    }

    pub fn confirm_action_then<F1, F2, F3>(player_id: usize, on_yes: F1, then: F2, else_then: F3) -> Dialogue
        where F1: Fn(usize) + 'static, F2: Fn() -> Dialogue + 'static, F3: Fn() -> Dialogue + 'static
    {
        let mut responses = Vec::new();

        responses.push(Response::new("Yes", on_yes, then));
        responses.push(Response::new("No", |_:usize|{}, else_then));

        Dialogue
        {
            title: String::from("Confirm Action"),
            text: None,
            info: Some(String::from("Are you sure?")),
            responses,
            commands: Vec::new(),
            text_handler: None,
            player_id,
            id: random()
        }
    }

    pub fn commands(title: &str, commands: Vec<Command>, player_id: usize) -> Dialogue
    {
        Dialogue
        {
            title: String::from(title),
            text: None,
            info: None,
            responses: Vec::new(),
            commands,
            text_handler: None,
            player_id,
            id: random()
        }
    }

    pub fn commands_with_text
    (
        title: &str,
        text: &[&str],
        replacements: Vec<(&'static str, String)>,
        commands: Vec<Command>,
        player_id: usize
    )
        -> Dialogue
    {
        let text = match thread_rng().choose(text)
        {
            Some(txt) => Some(text::apply_replacements(&String::from(*txt), &replacements)),
            None => None
        };

        Dialogue
        {
            title: String::from(title),
            text,
            info: None,
            responses: Vec::new(),
            commands,
            text_handler: None,
            player_id,
            id: random()
        }
    }

    pub fn from_area(player_id: usize) -> Dialogue
    {
        var_access::access_player_meta(player_id, | meta |
        {
            var_access::access_area(meta.coordinates, | area |
            {
                area.get_dialogue_for_player(player_id)
            })
            .expect("Area no longer exists.")
        })
        .expect("Player data no longer exists.")
    }

    pub fn empty(player_id: usize) -> Dialogue
    {
        Dialogue
        {
            title: String::from("..."),
            text: None,
            info: None,
            responses: Vec::new(),
            commands: Vec::new(),
            text_handler: None,
            player_id,
            id: random()
        }
    }

    pub fn run(&self, args: &String, first_response: usize) -> DialogueResult
    {
        self.run_as_user(args, self.player_id, first_response)
    }

    pub fn run_as_user(&self, args: &String, player_id: usize, first_response: usize) -> DialogueResult
    {
        let mut split = args.split_whitespace();

        let command = split.next();

        match command
        {
            Some(cmd) =>
            {
                let num: usize = cmd.parse().unwrap_or(0);
                let num = num - (first_response - 1);

                if num > 0
                {
                    if self.responses.len() >= num
                    {
                        let option: &Response = self.responses.get(num - 1).unwrap();

                        option.run(player_id, self);

                        return Success;
                    }
                    else { return InvalidNumber(self.responses.len()); }
                }
                else
                {
                    for c in &self.commands
                    {
                        if c.name == cmd
                        {
                            let args: Vec<&str> = Vec::from_iter(split);

                            c.run(&args, player_id, &self);

                            return Success;
                        }
                    }
                    if let Some(ref th) = self.text_handler
                    {
                        th.run(player_id, args, self);

                        return Success;
                    }
                    NoneFound
                }
            },
            None => NoArgs
        }
    }

    pub fn get_display(&self, first_response: usize) -> String
    {
        let mut ret = String::new();

        ret += &format!("### {} ###\n\n", self.title);

        if let Some(ref description) = self.info
        {
            ret += &format!("> {}\n", description.replace("\n", "\n> "));
            ret += "\n";
        }

        let mut option_num = first_response;

        for option in &self.responses
        {
            ret += &format!("{}: {}\n", option_num, option.text);
            option_num += 1;
        }

        if let Some(ref th) = self.text_handler
        {
            ret += &format!("_: {}", th.text);
        }

        if self.commands.len() > 0
        {
            ret += "\n";
        }

        for command in &self.commands
        {
            ret += &format!("{}\n", command.get_display());
        }
        ret
    }

    pub fn get_id(&self) -> usize { self.id }

    pub fn delete_in(player_id: usize, option_id: usize, delay_ms: u64) -> DelayHandler
    {
        DelayedEvent::no_flags(delay_ms,move ||
        {
            if delete_options(option_id)
            {
                send_current_options(player_id);
            }
        });

        DelayHandler::new(delay_ms)
    }
}

pub struct Response
{
    pub text: String,
    pub execute: Option<Box<Fn(usize)>>,
    pub next_dialogue: DialogueOption
}

impl Response
{
    pub fn new<F1, F2>(text: &'static str, run: F1, then: F2) -> Response
        where F1: Fn(usize) + 'static, F2: Fn() -> Dialogue + 'static
    {
        Response
        {
            text: String::from(text),
            execute: Some(Box::new(run)),
            next_dialogue: Generate(Box::new(then))
        }
    }

    pub fn simple<F>(text: &'static str, run: F) -> Response
        where F: Fn(usize) + 'static
    {
        Self::_simple(String::from(text), run)
    }

    pub fn _simple<F>(text: String, run: F) -> Response
        where F: Fn(usize) + 'static
    {
        Response
        {
            text,
            execute: Some(Box::new(run)),
            next_dialogue: FromArea
        }
    }

    pub fn action_only<F>(text: &'static str, run: F) -> Response
        where F: Fn(usize) + 'static
    {
        Self::_action_only(String::from(text), run)
    }

    pub fn _action_only<F>(text: String, run: F) -> Response
        where F: Fn(usize) + 'static
    {
        Response
        {
            text,
            execute: Some(Box::new(run)),
            next_dialogue: Ignore
        }
    }

    pub fn delete_dialogue<F>(text: &'static str, run: F) -> Response
        where F: Fn(usize) + 'static
    {
        Self::_delete_dialogue(String::from(text), run)
    }

    pub fn _delete_dialogue<F>(text: String, run: F) -> Response
        where F: Fn(usize) + 'static
    {
        Response
        {
            text,
            execute: Some(Box::new(run)),
            next_dialogue: Delete
        }
    }

    pub fn text_only(text: &'static str) -> Response
    {
        Self::_text_only(String::from(text))
    }

    pub fn _text_only(text: String) -> Response
    {
        Response
        {
            text,
            execute: None,
            next_dialogue: FromArea
        }
    }

    pub fn goto_dialogue<F>(text: &'static str, next_dialogue: F) -> Response
        where F: Fn() -> Dialogue + 'static
    {
        Self::_goto_dialogue(String::from(text), next_dialogue)
    }

    /**
     * To-do: Better name needed.
     */
    pub fn _goto_dialogue<F>(text: String, next_dialogue: F) -> Response
        where F: Fn() -> Dialogue + 'static
    {
        Response
        {
            text,
            execute: None,
            next_dialogue: Generate(Box::new(next_dialogue))
        }
    }

    pub fn get_entity_dialogue(text: &'static str, accessor: EntityAccessor, player_id: usize) -> Response
    {
        Self::_get_entity_dialogue(String::from(text), accessor, player_id)
    }

    pub fn _get_entity_dialogue(text: String, accessor: EntityAccessor, player_id: usize) -> Response
    {
        Response
        {
            text,
            execute: None,
            next_dialogue: Generate(Box::new(move ||
            {
                match var_access::access_entity(accessor, | e |
                {
                    e.get_dialogue_for_player(player_id).expect("Expected this function to return Dialogue.")
                }){
                    Some(dialogue) => dialogue,
                    None => var_access::access_area(accessor.coordinates, | a |
                    {
                        ::add_short_message(player_id, &String::from("They got bored and walked away."));
                        a.get_dialogue_for_player(player_id)
                    })
                    .expect("Area no longer exists")
                }
            }))
        }
    }

    pub fn goto_entity_dialogue(text: &'static str, marker: u8, accessor: EntityAccessor, player_id: usize) -> Response
    {
        Self::_goto_entity_dialogue(String::from(text), marker, accessor, player_id)
    }

    pub fn _goto_entity_dialogue(text: String, marker: u8, accessor: EntityAccessor, player_id: usize) -> Response
    {
        Response
        {
            text,
            execute: None,
            next_dialogue: Generate(Box::new(move ||
            {
                match var_access::access_entity(accessor, | e |
                {
                    e.goto_dialogue_for_player(marker, player_id).expect("Expected this function to return Dialogue.")
                }){
                    Some(dialogue) => dialogue,
                    None => var_access::access_area(accessor.coordinates, | a |
                    {
                        ::add_short_message(player_id, &String::from("They got bored and walked away."));
                        a.get_dialogue_for_player(player_id)
                    })
                    .expect("Area no longer exists")
                }
            }))
        }
    }

    pub fn run(&self, player_id: usize, current_dialogue: &Dialogue)
    {
        if let Some(ref exe) = self.execute
        {
            (exe)(player_id);
        }

        let next_dialogue = match self.next_dialogue
        {
            Generate(ref d) => Some((d)()),
            FromArea => Some(Dialogue::from_area(player_id)),
            Delete =>
            {
                delete_options(current_dialogue.id);
                send_current_options(player_id);
                None
            },
            Ignore => None
        };

        if let Some(dialogue) = next_dialogue
        {
            let text = dialogue.text.clone();

            if let Some(ref txt) = text
            {
                delete_options(current_dialogue.id);
                register_options(dialogue);
                update_options_manually(player_id);

                ::send_blocking_message(player_id, txt, ::TEXT_SPEED);
            }
            else { ::replace_send_options(player_id, current_dialogue.id, dialogue); }
        }
    }
}

pub struct Command
{
    pub name: String,
    pub input_desc: String,
    pub output_desc: String,
    pub run: Box<Fn(&Vec<&str>, usize)>,
    pub next_dialogue: DialogueOption
}

impl Command
{
    pub fn new<F1, F2>(input: &'static str, desc: &'static str, output: &'static str, run: F1, next_dialogue: F2) -> Command
        where F1: Fn(&Vec<&str>, usize) + 'static,
              F2: Fn() -> Dialogue + 'static
    {
        Command
        {
            name: String::from(input),
            input_desc: String::from(desc),
            output_desc: String::from(output),
            run: Box::new(run),
            next_dialogue: Generate(Box::new(next_dialogue))
        }
    }

    pub fn simple<F>(input: &'static str, output: &'static str, run: F) -> Command
        where F: Fn(&Vec<&str>, usize) + 'static
    {
        Command
        {
            name: String::from(input),
            input_desc: String::from(input),
            output_desc: String::from(output),
            run: Box::new(run),
            next_dialogue: FromArea
        }
    }

    pub fn manual_desc<F>(input: &'static str, desc: &'static str, output: &'static str, run: F) -> Command
        where F: Fn(&Vec<&str>, usize) + 'static
    {
        Command
        {
            name: String::from(input),
            input_desc: String::from(desc),
            output_desc: String::from(output),
            run: Box::new(run),
            next_dialogue: FromArea
        }
    }

    pub fn manual_desc_no_next<F>(input: &'static str, desc: &'static str, output: &'static str, run: F) -> Command
        where F: Fn(&Vec<&str>, usize) + 'static
    {
        Command
        {
            name: String::from(input),
            input_desc: String::from(desc),
            output_desc: String::from(output),
            run: Box::new(run),
            next_dialogue: Ignore
        }
    }

    pub fn goto_dialogue<F>(input: &'static str, output: &'static str, dialogue: F) -> Command
        where F: Fn() -> Dialogue + 'static
    {
        Command
        {
            name: String::from(input),
            input_desc: String::from(input),
            output_desc: String::from(output),
            run: Box::new(|_,_|{}),
            next_dialogue: Generate(Box::new(dialogue))
        }
    }

    pub fn run(&self, args: &Vec<&str>, player_id: usize, current_dialogue: &Dialogue)
    {
        (self.run)(args, player_id);

        let next_dialogue = match self.next_dialogue
        {
            Generate(ref d) => Some((d)()),
            FromArea => Some(Dialogue::from_area(player_id)),
            Delete =>
            {
                delete_options(current_dialogue.id);
                send_current_options(player_id);
                None
            },
            Ignore => None
        };

        if let Some(dialogue) = next_dialogue
        {
            let text = dialogue.text.clone();

            if let Some(ref txt) = text
            {
                delete_options(current_dialogue.id);
                register_options(dialogue);
                update_options_manually(player_id);

                ::send_blocking_message(player_id, txt, ::TEXT_SPEED);
            }
            else { ::replace_send_options(player_id, current_dialogue.id, dialogue); }
        }
    }

    pub fn get_display(&self) -> String
    {
        format!("| {} | -> {}", self.input_desc, self.output_desc)
    }
}

pub struct TextHandler
{
    pub text: String,
    pub execute: Box<Fn(&String)>,
    pub next_dialogue: DialogueOption
}

impl TextHandler
{
    pub fn run(&self, player_id: usize, args: &String, current_dialogue: &Dialogue)
    {
        (self.execute)(args);

        let next_dialogue= match self.next_dialogue
        {
            Generate(ref d) => Some((d)()),
            FromArea => Some(Dialogue::from_area(player_id)),
            Delete =>
            {
                delete_options(current_dialogue.id);
                send_current_options(player_id);
                None
            },
            Ignore => None
        };

        if let Some(dialogue) = next_dialogue
        {
            let text = dialogue.text.clone();

            if let Some(ref txt) = text
            {
                delete_options(current_dialogue.id);
                register_options(dialogue);
                update_options_manually(player_id);

                ::send_blocking_message(player_id, txt, ::TEXT_SPEED);
            }
            else { ::replace_send_options(player_id, current_dialogue.id, dialogue); }
        }
    }
}