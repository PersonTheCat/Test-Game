use crate::messages::MessageComponent::*;
use crate::player_data::PlayerMeta;
use crate::text;
use crate::util::access::{self, EntityAccessor};
use crate::util::timed_events::{DelayHandler, DelayedEvent};
use crate::*;

use std::iter::FromIterator;
use std::sync::Arc;

use lazy_static::lazy_static;
use parking_lot::Mutex;
use rand::random;

use self::DialogueOption::*;
use self::DialogueResult::*;

pub const GLOBAL_USER: usize = 01001010100101010;

lazy_static! {
    pub static ref CURRENT_OPTIONS: Mutex<Vec<Arc<Dialogue>>> = Mutex::new(Vec::new());
}

pub fn register_options(options: Dialogue) {
    _register_options(Arc::new(options));
}

pub fn _register_options(options: Arc<Dialogue>) {
    CURRENT_OPTIONS.lock().push(options);
}

pub fn delete_options(option_id: usize) -> Option<Dialogue> {
    let mut registry = CURRENT_OPTIONS.lock();
    registry.iter()
        .position(|o| o.id == option_id && o.player_id != GLOBAL_USER)
        .and_then(|i| Arc::try_unwrap(registry.remove(i)).ok())
}

/// A variant of delete_options() which will only
/// succeed when the player has exactly one dialogue.
pub fn try_delete_options(player_id: usize) -> Result<Arc<Dialogue>, &'static str> {
    let mut registry = CURRENT_OPTIONS.lock();
    let matches: Vec<usize> = registry.iter()
        .enumerate()
        .filter(|(_, d)| d.player_id == player_id)
        .map(|(i, _)| i)
        .collect();

    if matches.len() != 1 {
        return Err("Multiple dialogues were found. Not sure which to remove.");
    }
    Ok(registry.remove(matches[0]))
}

/// Removes all options associated with this player.
pub fn remove_all_options(player_id: usize) -> Vec<Arc<Dialogue>> {
    CURRENT_OPTIONS.lock()
        .drain_filter(|d| d.player_id == player_id)
        .collect()
}

/// Locates the player ID associated with this dialogue.
pub fn get_player_for_options(option_id: usize) -> Option<usize> {
    CURRENT_OPTIONS.lock()
        .iter()
        .find(|o| o.id == option_id)
        .and_then(|o| Some(o.player_id))
}

/// Generates the formatted dialogue text for this player.
pub fn get_options_text(for_player: usize) -> String {
    let mut options_text = String::new();
    let mut first_response = 1;
    CURRENT_OPTIONS.lock()
        .iter()
        .filter(|o| o.player_id == for_player)
        .for_each(|o| {
            options_text += &format!("\n{}", o.get_display(first_response));
            first_response += o.responses.len();
        });
    options_text
}

/// A convenience function used for deleting one dialogue
/// and replacing it with another.
pub fn replace_options(player_id: usize, old_options: usize, new_options: Dialogue) {
    if let Some(options) = delete_options(old_options) {
        if player_id != options.player_id {
            println!(
                "Debug: A call was somehow sent to replace dialogue\n\
                 for one player with that of another. This message\n\
                 Is temporary and should be fixed.\n\
                 From id:{}\n\
                 To id:{}",
                options.player_id, player_id
            );
            register_options(options);
            return;
        }
    }
    register_options(new_options);
}

/// Attempts to delete one player's dialogue and resend
/// it it. This works based on the assumption that the
/// player only has one dialogue, which therefor must
/// be associated with their current area. If there is
/// more than one dialogue, the assumption is wrong
/// and the function will fail. Returns a boolean
/// indicating this outcome.
pub fn try_refresh_options(player_id: usize) -> bool {
    try_delete_options(player_id)
        .and_then(|_| Ok(temp_get_send_area_options(player_id)))
        .is_ok()
}

/// Attempts to locate the player data associated with
/// this id and display their current options to the
/// screen. This will most likely freeze the game if
/// run from the main thread, as it tries to acquire
/// a lock on the player registry, which may already
/// be taken in an earlier scope. To avoid this issue,
/// only run this option when the user's PlayerMeta
/// object is not currently in scope.
pub fn temp_send_current_options(to_player: usize) {
    let options_text = get_options_text(to_player);
    temp_send_message_to_player(to_player, Options, &options_text, 0);
}

pub fn temp_update_options(for_player: usize) {
    let options_text = get_options_text(for_player);
    temp_update_player_message(for_player, Options, &options_text);
}

pub fn temp_get_send_area_options(player_id: usize) {
    access::player_meta(player_id).get_send_area_options();
}

pub fn temp_replace_send_options(player_id: usize, old_options: usize, new_options: Dialogue) {
    access::player_meta(player_id).replace_send_options(old_options, new_options);
}

pub fn temp_replace_no_send_options(player_id: usize, old_options: usize, new_options: Dialogue) {
    access::player_meta(player_id).replace_options(old_options, new_options);
}

#[derive(Debug)]
pub enum DialogueResult {
    Success,
    InvalidNumber(usize),
    NoneFound,
    NoArgs,
}

pub enum DialogueOption {
    FromArea,
    Ignore,
    Delete, // Dialogue will be automatically resent; Don't double-do it.
    Generate(Box<Fn(&PlayerMeta) -> Dialogue>),
}

/// A shorthand function for creating `Generate()`
/// dialogue options.
pub fn gen_dialogue<F>(run: F) -> DialogueOption
    where F: Fn(&PlayerMeta) -> Dialogue + 'static
{
    Generate(Box::new(run))
}

pub struct Dialogue {
    pub title: String,
    pub text: Option<String>,
    pub info: Option<String>,
    pub responses: Vec<Response>,
    pub commands: Vec<Command>,
    pub text_handler: Option<TextHandler>,
    pub player_id: usize,
    pub id: usize,
}

impl Default for Dialogue {
    fn default() -> Dialogue {
        Dialogue {
            title: String::from("Unnamed Dialogue"),
            text: None,
            info: None,
            responses: Vec::new(),
            commands: Vec::new(),
            text_handler: None,
            player_id: GLOBAL_USER,
            id: random()
        }
    }
}

/// These implementations should theoretically not be used.
/// Do not use these to send Dialogues between threads.
/// Instead, let all references to them be handled by the
/// main thread. These should act as a temporary solution
/// until a better data structure can be worked out for
/// storing dialogue in the main thread only. The reason
/// for this issue is that closures cannot be passed
/// between threads. It may be possible to avoid using
/// closures altogether by instead using function
/// references; however, this would place some fairly
/// severe limitations on what Dialogues can handle.
/// As such, the way forward is not exactly clear and
/// will have to be ignored unless this project becomes
/// more serious.
unsafe impl Send for Dialogue {}
unsafe impl Sync for Dialogue {}

impl Dialogue {
    pub fn simple(title: String, text: String, responses: Vec<Response>, player_id: usize) -> Dialogue {
        Dialogue {
            title,
            text: Some(text),
            responses,
            player_id,
            ..Self::default()
        }
    }

    pub fn no_message(title: &str, responses: Vec<Response>, commands: Vec<Command>, player_id: usize) -> Dialogue {
        Dialogue {
            title: String::from(title),
            responses,
            commands,
            player_id,
            ..Self::default()
        }
    }

    pub fn handle_text(title: String, text: Option<String>, text_handler: TextHandler, player_id: usize) -> Dialogue {
        Dialogue {
            title,
            text,
            text_handler: Some(text_handler),
            player_id,
            ..Self::default()
        }
    }

    pub fn commands(title: &str, commands: Vec<Command>, player_id: usize) -> Dialogue {
        Dialogue {
            title: String::from(title),
            commands,
            player_id,
            ..Self::default()
        }
    }

    pub fn commands_with_text(title: &str, text: String, commands: Vec<Command>, player_id: usize) -> Dialogue {
        Dialogue {
            title: String::from(title),
            text: Some(text),
            commands,
            player_id,
            ..Self::default()
        }
    }

    pub fn empty(player_id: usize) -> Dialogue {
        Dialogue {
            title: String::from("..."),
            player_id,
            ..Self::default()
        }
    }

    pub fn from_area(player: &PlayerMeta) -> Dialogue {
        access::area(player.get_coordinates(), |a| a._get_dialogue(player))
            .expect("Area was somehow deleted.")
    }

    pub fn confirm_action<F1, F2>(player_id: usize,temporary: bool, on_yes: F1, on_no: F2) -> Dialogue
        where F1: Fn(&PlayerMeta) + 'static,
              F2: Fn(&PlayerMeta) + 'static
    {
        let id = random();
        let responses = vec![
            Response::delete_dialogue("Yes", on_yes),
            Response::delete_dialogue("No", on_no)
        ];
        if temporary {
            Self::delete_in(player_id, id, TEMP_DIALOGUE_DURATION);
        }

        Dialogue {
            title: String::from("Confirm Action"),
            info: Some(String::from("Are you sure?")),
            responses,
            player_id,
            id,
            ..Self::default()
        }
    }

    pub fn confirm_action_then<F1, F2, F3>(player_id: usize, on_yes: F1, then: F2, else_then: F3,) -> Dialogue
        where F1: Fn(&PlayerMeta) + 'static,
              F2: Fn(&PlayerMeta) -> Dialogue + 'static,
              F3: Fn(&PlayerMeta) -> Dialogue + 'static
    {
        let responses = vec![
            Response::new("Yes", on_yes, then),
            Response::new("No", |_: &PlayerMeta| {}, else_then)
        ];

        Dialogue {
            title: String::from("Confirm Action"),
            info: Some(String::from("Are you sure?")),
            responses,
            player_id,
            ..Self::default()
        }
    }

    pub fn run(&self, args: &str, player: &PlayerMeta, first_response: usize) -> DialogueResult {
        let mut split = args.split_whitespace();
        let command = match split.next() {
            Some(cmd) => cmd,
            None => return NoArgs,
        };

        let num: usize = command.parse().unwrap_or(0);
        let num = num - (first_response - 1);

        // Handle numbered responses.
        if num > 0 {
            if self.responses.len() >= num {
                let option: &Response = self.responses.get(num - 1).unwrap();
                option.run(player, self);
                return Success;
            }
            return InvalidNumber(self.responses.len());
        }

        // Handle commands
        let cmd = self.commands.iter()
            .find(|c| c.name == command);
        if let Some(c) = cmd {
            let args: Vec<&str> = Vec::from_iter(split);
            c.run(&args, player, &self);
            return Success;
        }
        // Handle normal text input. If this exists,
        // it will always return a success.
        if let Some(ref th) = &self.text_handler {
            th.run(player, args, &self);
            return Success;
        }
        NoneFound
    }

    pub fn get_display(&self, first_response: usize) -> String {
        let mut ret = String::new();

        ret += &format!("### {} ###\n\n", self.title);

        if let Some(ref description) = self.info {
            ret += &format!("> {}\n", description.replace("\n", "\n> "));
            ret += "\n";
        }

        let mut option_num = first_response;

        for option in &self.responses {
            ret += &option.get_display(option_num);
            option_num += 1;
        }
        if let Some(ref th) = self.text_handler {
            ret += &format!("_: {}", th.text);
        }
        if self.commands.len() > 0 {
            ret += "\n";
        }
        for command in &self.commands {
            ret += &command.get_display();
        }
        ret
    }

    pub fn get_id(&self) -> usize {
        self.id
    }

    pub fn delete_in(player_id: usize, option_id: usize, delay_ms: u64) -> DelayHandler {
        DelayedEvent::no_flags(delay_ms, move || {
            delete_options(option_id).and_then(|_| {
                Some(access::player_meta(player_id).send_current_options())
            });
        });
        DelayHandler::new(delay_ms)
    }
}

pub struct Response {
    pub text: String,
    pub execute: Option<Box<Fn(&PlayerMeta) + 'static>>,
    pub next_dialogue: DialogueOption,
}

impl Response {
    pub fn new<F1, F2>(text: &str, run: F1, then: F2) -> Response
        where F1: Fn(&PlayerMeta) + 'static,
              F2: Fn(&PlayerMeta) -> Dialogue + 'static
    {
        Response {
            text: String::from(text),
            execute: Some(Box::new(run)),
            next_dialogue: Generate(Box::new(then)),
        }
    }

    pub fn simple<F>(text: &str, run: F) -> Response
        where F: Fn(&PlayerMeta) + 'static
    {
        Self::_simple(String::from(text), run)
    }

    pub fn _simple<F>(text: String, run: F) -> Response
        where F: Fn(&PlayerMeta) + 'static
    {
        Response {
            text,
            execute: Some(Box::new(run)),
            next_dialogue: FromArea,
        }
    }

    pub fn action_only<F>(text: &str, run: F) -> Response
        where F: Fn(&PlayerMeta) + 'static
    {
        Self::_action_only(String::from(text), run)
    }

    pub fn _action_only<F>(text: String, run: F) -> Response
        where F: Fn(&PlayerMeta) + 'static
    {
        Response {
            text,
            execute: Some(Box::new(run)),
            next_dialogue: Ignore,
        }
    }

    pub fn delete_dialogue<F>(text: &str, run: F) -> Response
        where F: Fn(&PlayerMeta) + 'static
    {
        Self::_delete_dialogue(String::from(text), run)
    }

    pub fn _delete_dialogue<F>(text: String, run: F) -> Response
        where F: Fn(&PlayerMeta) + 'static
    {
        Response {
            text,
            execute: Some(Box::new(run)),
            next_dialogue: Delete,
        }
    }

    pub fn text_only(text: &str) -> Response {
        Self::_text_only(String::from(text))
    }

    pub fn _text_only(text: String) -> Response {
        Response {
            text,
            execute: None,
            next_dialogue: FromArea,
        }
    }

    pub fn goto_dialogue<F>(text: &str, next_dialogue: F) -> Response
        where F: Fn(&PlayerMeta) -> Dialogue + 'static
    {
        Self::_goto_dialogue(String::from(text), next_dialogue)
    }


    // To-do: Better name needed.
    pub fn _goto_dialogue<F>(text: String, next_dialogue: F) -> Response
        where F: Fn(&PlayerMeta) -> Dialogue + 'static
    {
        Response {
            text,
            execute: None,
            next_dialogue: Generate(Box::new(next_dialogue)),
        }
    }

    pub fn get_entity_dialogue(text: &str, accessor: EntityAccessor) -> Response {
        Self::_get_entity_dialogue(String::from(text), accessor)
    }

    pub fn _get_entity_dialogue(text: String, accessor: EntityAccessor) -> Response {
        Response {
            text,
            execute: None,
            next_dialogue: gen_dialogue(move |player| {
                match access::entity(accessor, |e| {
                    e._get_dialogue(player)
                        .expect("Called get_entity_dialogue() for an entity that does not have dialogue.")
                }) {
                    Some(d) => d,
                    None => access::area(accessor.coordinates, |a| {
                        player.add_short_message("They got bored and walked away.");
                        a._get_dialogue(player)
                    })
                    .expect("Player's current area somehow disappeared.")
                }
            })
        }
    }

    pub fn goto_entity_dialogue(text: &str, marker: u8, accessor: EntityAccessor) -> Response {
        Self::_goto_entity_dialogue(String::from(text), marker, accessor)
    }

    pub fn _goto_entity_dialogue(text: String, marker: u8, accessor: EntityAccessor) -> Response {
        Response {
            text,
            execute: None,
            next_dialogue: gen_dialogue(move |player| {
                match access::entity(accessor, |e| {
                    e._goto_dialogue(marker, player)
                        .expect("Called goto_entity_dialogue() for an entity that does not have dialogue.")
                }) {
                    Some(d) => d,
                    None => access::area(accessor.coordinates, |a| {
                        player.add_short_message("They got bored and walked away.");
                        a._get_dialogue(player)
                    })
                        .expect("Player's current area somehow disappeared.")
                }
            })
        }
    }

    pub fn run(&self, player: &PlayerMeta, current_dialogue: &Dialogue) {
        if let Some(ref exe) = self.execute {
            (exe)(player);
        }

        let next_dialogue = match &self.next_dialogue {
            Generate(ref d) => Some((d)(player)),
            FromArea => Some(Dialogue::from_area(player)),
            Delete => {
                delete_options(current_dialogue.id);
                player.send_current_options();
                None
            }
            Ignore => None,
        };

        if let Some(dialogue) = next_dialogue {
            let text = dialogue.text.clone();

            if let Some(ref txt) = text {
                delete_options(current_dialogue.id);
                register_options(dialogue);
                player.update_options();
                player.send_blocking_message(txt, TEXT_SPEED);
            } else {
                player.replace_send_options(current_dialogue.id, dialogue);
            }
        }
    }

    pub fn get_display(&self, option_num: usize) -> String {
        if self.text.starts_with("§") {
            let text = text::auto_break(3, &self.text[2..]);
            format!("{}: {}\n", option_num, text)
        } else {
            format!("{}: {}\n", option_num, self.text)
        }
    }
}

pub struct Command {
    pub name: String,
    pub input_desc: String,
    pub output_desc: String,
    pub run: Box<Fn(&Vec<&str>, &PlayerMeta) + 'static>,
    pub next_dialogue: DialogueOption,
}

impl Command {
    pub fn new<F1, F2>(input: &str, desc: &str, output: &str, run: F1, next_dialogue: F2) -> Command
        where F1: Fn(&Vec<&str>, &PlayerMeta) + 'static,
              F2: Fn(&PlayerMeta) -> Dialogue + 'static
    {
        Command {
            name: String::from(input),
            input_desc: String::from(desc),
            output_desc: String::from(output),
            run: Box::new(run),
            next_dialogue: Generate(Box::new(next_dialogue)),
        }
    }

    pub fn simple<F>(input: &str, output: &str, run: F) -> Command
        where F: Fn(&Vec<&str>, &PlayerMeta) + 'static
    {
        Command {
            name: String::from(input),
            input_desc: String::from(input),
            output_desc: String::from(output),
            run: Box::new(run),
            next_dialogue: FromArea,
        }
    }

    pub fn action_only<F>(input: &str, output: &str, run: F) -> Command
        where F: Fn(&Vec<&str>, &PlayerMeta) + 'static
    {
        Command {
            name: String::from(input),
            input_desc: String::from(input),
            output_desc: String::from(output),
            run: Box::new(run),
            next_dialogue: Ignore,
        }
    }

    pub fn text_only(input: &str, desc: &str, output: &str) -> Command {
        Command {
            name: String::from(input),
            input_desc: String::from(desc),
            output_desc: String::from(output),
            run: Box::new(|_, _| {}),
            next_dialogue: FromArea,
        }
    }

    pub fn delete_dialogue<F>(input: &str, desc: &str, output: &str, run: F) -> Command
        where F: Fn(&Vec<&str>, &PlayerMeta) + 'static
    {
        Command {
            name: String::from(input),
            input_desc: String::from(desc),
            output_desc: String::from(output),
            run: Box::new(run),
            next_dialogue: Delete,
        }
    }

    pub fn manual_desc<F>(input: &str, desc: &str, output: &str, run: F) -> Command
        where F: Fn(&Vec<&str>, &PlayerMeta) + 'static
    {
        Command {
            name: String::from(input),
            input_desc: String::from(desc),
            output_desc: String::from(output),
            run: Box::new(run),
            next_dialogue: FromArea,
        }
    }

    pub fn manual_desc_no_next<F>(input: &str, desc: &str, output: &str, run: F) -> Command
        where F: Fn(&Vec<&str>, &PlayerMeta) + 'static
    {
        Command {
            name: String::from(input),
            input_desc: String::from(desc),
            output_desc: String::from(output),
            run: Box::new(run),
            next_dialogue: Ignore,
        }
    }

    pub fn goto_dialogue<F>(input: &str, output: &str, dialogue: F) -> Command
        where F: Fn(&PlayerMeta) -> Dialogue + 'static
    {
        Command {
            name: String::from(input),
            input_desc: String::from(input),
            output_desc: String::from(output),
            run: Box::new(|_, _| {}),
            next_dialogue: Generate(Box::new(dialogue)),
        }
    }

    pub fn run(&self, args: &Vec<&str>, player: &PlayerMeta, current_dialogue: &Dialogue) {
        (self.run)(args, player);

        let next_dialogue = match &self.next_dialogue {
            Generate(ref d) => Some((d)(player)),
            FromArea => Some(Dialogue::from_area(player)),
            Delete => {
                delete_options(current_dialogue.id);
                player.send_current_options();
                None
            }
            Ignore => None,
        };

        if let Some(dialogue) = next_dialogue {
            let text = dialogue.text.clone();

            if let Some(ref txt) = text {
                delete_options(current_dialogue.id);
                register_options(dialogue);
                player.update_options();
                player.send_blocking_message(txt, TEXT_SPEED);
            } else {
                player.replace_send_options(current_dialogue.id, dialogue);
            }
        }
    }

    pub fn get_display(&self) -> String {
        let text = format!("| {} | -> {}\n", self.input_desc, self.output_desc);
        if self.output_desc.starts_with("§") {
            text::auto_break(3, &text)
        } else {
            text
        }
    }
}

pub struct TextHandler {
    pub text: String,
    pub execute: Box<Fn(&PlayerMeta, &str) + 'static>,
    pub next_dialogue: DialogueOption,
}

impl TextHandler {
    pub fn run(&self, player: &PlayerMeta, args: &str, current_dialogue: &Dialogue) {
        (self.execute)(player, args);

        let next_dialogue = match &self.next_dialogue {
            Generate(ref d) => Some((d)(player)),
            FromArea => Some(Dialogue::from_area(player)),
            Delete => {
                delete_options(current_dialogue.id);
                player.send_current_options();
                None
            }
            Ignore => None,
        };
        if let Some(dialogue) = next_dialogue {
            let text = dialogue.text.clone();

            if let Some(ref txt) = text {
                delete_options(current_dialogue.id);
                register_options(dialogue);
                player.update_options();
                player.send_blocking_message(txt, TEXT_SPEED);
            } else {
                player.replace_send_options(current_dialogue.id, dialogue);
            }
        }
    }
}
