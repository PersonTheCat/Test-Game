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

/// An ID used for determining that the current
/// dialogue can be used for any player. Different
/// from having no user.
pub const GLOBAL_USER: usize = 01001010100101010;

lazy_static! {
    /// Player dialogue is stored statically.
    pub static ref CURRENT_OPTIONS: Mutex<Vec<Arc<Dialogue>>> = Mutex::new(Vec::new());
}

/// A function used for registering new options,
/// automatically wrapping them in reference
/// counters.
pub fn register_options(options: Dialogue) {
    _register_options(Arc::new(options));
}

/// A sub-function of `register_options()` which
/// accepts the completed form of the dialogue,
/// already wrapped in a reference counter.
pub fn _register_options(options: Arc<Dialogue>) {
    CURRENT_OPTIONS.lock().push(options);
}

/// Deletes and attempts to unwrap the dialogue.
/// It's worth noting that this will only succeed
/// when there are no references currently in scope.
/// This may impose clarity issues and might need
/// to be adjusted, as a result.
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
    let length = access::player_meta(for_player).get_text_length();
    let mut first_response = 1;
    CURRENT_OPTIONS.lock()
        .iter()
        .filter(|o| o.player_id == for_player)
        .for_each(|o| {
            options_text += &format!("\n{}", o.get_display(length, first_response));
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
    temp_send_message_to_player(to_player, Options, &options_text);
}

/// A static function for refreshing the input player's
/// current dialogue.
pub fn temp_update_options(for_player: usize) {
    let options_text = get_options_text(for_player);
    temp_update_player_message(for_player, Options, &options_text);
}

/// A static function for retrieving dialogue for the
/// player from their current area, which handles
/// automatically refreshing that dialogue to their
/// display.
pub fn temp_get_send_area_options(player_id: usize) {
    access::player_meta(player_id).get_send_area_options();
}

/// A static function for replacing the input dialogue
/// with new options for the specified player.
pub fn temp_replace_send_options(player_id: usize, old_options: usize, new_options: Dialogue) {
    access::player_meta(player_id).replace_send_options(old_options, new_options);
}

/// A variant of `temp_replace_send_options()` which
/// does not automatically refresh the information
/// to the player.
pub fn temp_replace_options(player_id: usize, old_options: usize, new_options: Dialogue) {
    access::player_meta(player_id).replace_options(old_options, new_options);
}

/// The result of processing the current dialogue.
/// Informs the game of whether to continue checking
/// through additional dialogues or if arguments are
/// missing.
#[derive(Debug)]
pub enum DialogueResult {
    Success,
    InvalidNumber(usize),
    NoneFound,
    NoArgs
}

/// An option for determining what to do after a
/// dialogue has been processed.
pub enum DialogueOption {
    /// Generates the next dialogue from the player's
    /// current area.
    FromArea,
    /// Do nothing, ignoring whether further dialogue
    /// should be generated or potentially handling it
    /// internally.
    Ignore,
    /// Do nothing and delete the existing dialogue.
    /// Used when players have multiple dialogues
    /// prompting them for input. This will also
    /// automatically handle resending the current
    /// dialogue to the player. And thus any calls
    /// to `PlayerMeta#send_message()` or another
    /// such variant should be substituted with
    /// `PlayerMeta#udpate_message()`.
    Delete,
    /// Generate the next dialogue from the input
    /// function. Using `gen_dialogue` with a supplied
    /// closure may produce a cleaner syntax in many
    /// cases.
    Generate(Box<Fn(&PlayerMeta) -> Dialogue>)
}

/// A shorthand function for creating `Generate()`
/// dialogue options.
pub fn gen_dialogue<F>(run: F) -> DialogueOption
    where F: Fn(&PlayerMeta) -> Dialogue + 'static
{
    Generate(Box::new(run))
}

pub struct Dialogue {
    /// The title to be displayed at the top of the dialogue.
    pub title: String,

    /// The optional text to be sent to the player before
    /// the actual dialogue / options are displayed.
    pub text: Option<String>,

    /// An optional field used for displaying about the
    /// current dialogue to the player. Displayed
    /// immediately below the title.
    pub info: Option<String>,

    /// A vector of type `Response`, used for displaying
    /// automatically-numbered options to the user.
    pub responses: Vec<Response>,

    /// A vector of type `Command`, used for displaying
    /// named options with arguments to the user.
    pub commands: Vec<Command>,

    /// An optional field of type `TextHandler`, used for
    /// handling any possible input from the user. This
    /// will be the last of each input type to be processed
    /// and is guaranteed to run regardless of any other
    /// circumstances.
    pub text_handler: Option<TextHandler>,

    /// Indicates whether this dialogue belongs to the
    /// user's current area, and thus whether it is safe
    /// to delete.
    pub is_primary: bool,

    /// The unique identifier of the player associated with
    /// this dialogue.
    pub player_id: usize,

    /// This dialogue's unique identifier.
    pub id: usize
}

/// The default implementation for Dialogue, used for
/// reducing some of the boilerplate that comes with it.
impl Default for Dialogue {
    fn default() -> Dialogue {
        Dialogue {
            title: String::from("Unnamed Dialogue"),
            text: None,
            info: None,
            responses: Vec::new(),
            commands: Vec::new(),
            text_handler: None,
            is_primary: false,
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
///
/// If you're offended by these implementations, please
/// help me by suggesting an alternative.
unsafe impl Send for Dialogue {}
unsafe impl Sync for Dialogue {}

impl Dialogue {
    /// A simple constructor for generating dialogue using
    /// the input `title`, a message to display, and a
    /// vector of type `Response` for the specified player.
    pub fn simple(title: String, text: String, responses: Vec<Response>, player_id: usize) -> Dialogue {
        Dialogue {
            title,
            text: Some(text),
            responses,
            player_id,
            ..Self::default()
        }
    }

    /// A variant of `simple()` which does not contain a
    /// message. Also features a vector of type `Command`,
    /// and thus probably deserves to be renamed.
    pub fn no_message(title: &str, responses: Vec<Response>, commands: Vec<Command>, player_id: usize) -> Dialogue {
        Dialogue {
            title: String::from(title),
            responses,
            commands,
            player_id,
            ..Self::default()
        }
    }

    /// Another variant of `simple()` which replaces the
    /// vector of `Response`s with a text handler. In this
    /// case, text is optional, which is a bit inconsistent,
    /// but was specifically designed for internal use.
    /// Probably needs to be updated for the sake of clarity.
    pub fn handle_text(title: String, text: Option<String>, text_handler: TextHandler, player_id: usize) -> Dialogue {
        Dialogue {
            title,
            text,
            text_handler: Some(text_handler),
            player_id,
            ..Self::default()
        }
    }

    /// Constructs a `Dialogue` from only a title and vector
    /// of type `Command` for the specified player.
    pub fn commands(title: &str, commands: Vec<Command>, player_id: usize) -> Dialogue {
        Dialogue {
            title: String::from(title),
            commands,
            player_id,
            ..Self::default()
        }
    }

    /// Variant of `commands()` which couples the dialogue
    /// with a message to the player.
    pub fn commands_with_text(title: &str, text: String, commands: Vec<Command>, player_id: usize) -> Dialogue {
        Dialogue {
            title: String::from(title),
            text: Some(text),
            commands,
            player_id,
            ..Self::default()
        }
    }

    /// A dialogue used internally for blocking input from
    /// the user by simply doing nothing.
    pub fn empty(player_id: usize) -> Dialogue {
        Dialogue {
            title: String::from("..."),
            player_id,
            ..Self::default()
        }
    }

    /// Retrieves the appropriate constructor from the player's
    /// current area.
    pub fn from_area(player: &PlayerMeta) -> Dialogue {
        player.area( |a| a.get_dialogue(player))
    }

    /// A dialogue which features two events for handling `yes`
    /// or `no` from the user.
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

    /// Variant of `confirm_action()` which specifies how
    /// the dialogue should be continued in either case.
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

    /// Indicates that a dialogue with the identifier
    /// `option_id` should be deleted after `delay_ms`
    /// milliseconds have passed.
    pub fn delete_in(player_id: usize, option_id: usize, delay_ms: u64) -> DelayHandler {
        DelayedEvent::no_flags(delay_ms, move || {
            delete_options(option_id).and_then(|_| {
                Some(access::player_meta(player_id).send_current_options())
            });
        });
        DelayHandler::new(delay_ms)
    }


    /// The main function used for processing this dialogue.
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
            .find(|c| c.matches_input(command));
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

    /// Formats each component of this dialogue into a clean
    /// display, which will be sent to the user starting at
    /// the response number indicated by `first_response`.
    /// This will typically be `1`, except when used recursively.
    pub fn get_display(&self, length: usize, first_response: usize) -> String {
        let mut ret = String::new();
        ret += &format!("### {} ###\n\n", self.title);

        if let Some(ref description) = self.info {
            ret += &format!("> {}\n", description.replace("\n", "\n> "));
            ret += "\n";
        }

        let mut option_num = first_response;
        for option in &self.responses {
            ret += &option.get_display(length, option_num);
            option_num += 1;
        }
        if let Some(ref th) = self.text_handler {
            ret += &th.get_display(length);
        }
        if self.commands.len() > 0 {
            ret += "\n";
        }
        for command in &self.commands {
            ret += &command.get_display(length);
        }
        ret
    }

    /// Reports whether this dialogue is intended to
    /// function for any user.
    pub fn is_global(&self) -> bool {
        self.player_id == GLOBAL_USER
    }

    pub fn get_id(&self) -> usize {
        self.id
    }
}

/// A type of Dialogue option used for handling
/// automatically-numbered responses.
pub struct Response {
    pub text: String,
    pub execute: Option<Box<Fn(&PlayerMeta) + 'static>>,
    pub next_dialogue: DialogueOption,
}

impl Response {
    /// A standard constructor which handles all fields
    /// in `Response`. This may look nicer in some
    /// contexts.
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

    /// A simple response which runs the specified
    /// closure and refreshes the dialogue from the
    /// player's current area.
    pub fn simple<F>(text: &str, run: F) -> Response
        where F: Fn(&PlayerMeta) + 'static
    {
        Self::_simple(String::from(text), run)
    }

    /// Variant of `simple()` which accepts an owned
    /// string instead of a slice.
    pub fn _simple<F>(text: String, run: F) -> Response
        where F: Fn(&PlayerMeta) + 'static
    {
        Response {
            text,
            execute: Some(Box::new(run)),
            next_dialogue: FromArea,
        }
    }

    /// Variant of `simple()` which does not refresh the
    /// player's dialogue after running.
    pub fn action_only<F>(text: &str, run: F) -> Response
        where F: Fn(&PlayerMeta) + 'static
    {
        Self::_action_only(String::from(text), run)
    }

    /// Variant of `action_only()` which accepts an owned
    /// string instead of a slice.
    pub fn _action_only<F>(text: String, run: F) -> Response
        where F: Fn(&PlayerMeta) + 'static
    {
        Response {
            text,
            execute: Some(Box::new(run)),
            next_dialogue: Ignore,
        }
    }

    /// Variant of `simple` which will delete its owner
    /// upon running.
    pub fn delete_dialogue<F>(text: &str, run: F) -> Response
        where F: Fn(&PlayerMeta) + 'static
    {
        Self::_delete_dialogue(String::from(text), run)
    }

    /// Variant of `delete_dialogue()` which accepts an
    /// owned string instead of a slice.
    pub fn _delete_dialogue<F>(text: String, run: F) -> Response
        where F: Fn(&PlayerMeta) + 'static
    {
        Response {
            text,
            execute: Some(Box::new(run)),
            next_dialogue: Delete,
        }
    }

    /// Constructs a response that has no action and simply
    /// refreshes the dialogue from the player's area upon
    /// running.
    pub fn text_only(text: &str) -> Response {
        Self::_text_only(String::from(text))
    }

    /// Variant of `text_only()` which accepts an owned
    /// string instead of a slice.
    pub fn _text_only(text: String) -> Response {
        Response {
            text,
            execute: None,
            next_dialogue: FromArea,
        }
    }

    /// Constructs a response which has no action, but directs
    ///  the dialogue to a new source using the input closure.
    pub fn goto_dialogue<F>(text: &str, next_dialogue: F) -> Response
        where F: Fn(&PlayerMeta) -> Dialogue + 'static
    {
        Self::_goto_dialogue(String::from(text), next_dialogue)
    }

    /// Variant of `goto_dialogue()` which accepts an owned
    /// string instead of a slice.
    pub fn _goto_dialogue<F>(text: String, next_dialogue: F) -> Response
        where F: Fn(&PlayerMeta) -> Dialogue + 'static
    {
        Response {
            text,
            execute: None,
            next_dialogue: Generate(Box::new(next_dialogue)),
        }
    }

    /// Constructs a response that generates dialogue from
    /// the input entity.
    pub fn get_entity_dialogue(text: &str, accessor: EntityAccessor) -> Response {
        Self::_get_entity_dialogue(String::from(text), accessor)
    }

    /// Variant of `get_entity_dialogue()` which accepts an
    /// owned string instead of a slice.
    pub fn _get_entity_dialogue(text: String, accessor: EntityAccessor) -> Response {
        Response {
            text,
            execute: None,
            next_dialogue: gen_dialogue(move |player| {
                match access::entity(accessor, |e| {
                    e.get_dialogue(player)
                        .expect("Called get_entity_dialogue() for an entity that does not have dialogue.")
                }) {
                    Some(d) => d,
                    None => access::area(accessor.coordinates, |a| {
                        player.add_short_message("They got bored and walked away.");
                        a.get_dialogue(player)
                    })
                    .expect("Player's current area somehow disappeared.")
                }
            })
        }
    }

    /// Variant of `get_entity_dialogue()` which returns the
    /// player to the dialogue at the specified `marker`.
    pub fn goto_entity_dialogue(text: &str, marker: u8, accessor: EntityAccessor) -> Response {
        Self::_goto_entity_dialogue(String::from(text), marker, accessor)
    }

    /// Variant of `goto_entity_dialogue()` that accepts an
    /// owned string instead of a slice.
    pub fn _goto_entity_dialogue(text: String, marker: u8, accessor: EntityAccessor) -> Response {
        Response {
            text,
            execute: None,
            next_dialogue: gen_dialogue(move |player| {
                match access::entity(accessor, |e| {
                    e.goto_dialogue(marker, player)
                        .expect("Called goto_entity_dialogue() for an entity that does not have dialogue.")
                }) {
                    Some(d) => d,
                    None => access::area(accessor.coordinates, |a| {
                        player.add_short_message("They got bored and walked away.");
                        a.get_dialogue(player)
                    })
                        .expect("Player's current area somehow disappeared.")
                }
            })
        }
    }

    /// The main method used for processing this response. Handles
    /// its execution, sending any possible messages to the user
    /// while blocking their input, and ultimately generating the
    /// next dialogue that will follow.
    pub fn run(&self, player: &PlayerMeta, current_dialogue: &Dialogue) {
        if let Some(ref exe) = self.execute {
            (exe)(player);
        }
        post_run(player, current_dialogue, &self.next_dialogue);
    }

    /// Formats this response to be displayed to the user.
    pub fn get_display(&self, length: usize, option_num: usize) -> String {
        if self.text.starts_with("§") {
            let text = text::auto_break(3, length,&self.text[2..]);
            format!("{}: {}\n", option_num, text)
        } else {
            format!("{}: {}\n", option_num, self.text)
        }
    }
}

/// Variant of `Response` which can be referred to by
/// its name while also allowing argument parameters.
/// `input` specifies the description of input shown
/// to the user, e.g. `money #`. The portion of the
/// string that precedes the first space in this
/// description is what will be matched to determine
/// whether to process the command.
pub struct Command {
    pub input: String,
    pub output_desc: String,
    pub run: Box<Fn(&Vec<&str>, &PlayerMeta) + 'static>,
    pub next_dialogue: DialogueOption,
}

impl Command {
    /// Constructs a new command while manually resolving its
    /// fields. May look nicer in some contexts.
    pub fn new<F1, F2>(input: &str, output: &str, run: F1, next_dialogue: F2) -> Command
        where F1: Fn(&Vec<&str>, &PlayerMeta) + 'static,
              F2: Fn(&PlayerMeta) -> Dialogue + 'static
    {
        Command {
            input: String::from(input),
            output_desc: String::from(output),
            run: Box::new(run),
            next_dialogue: Generate(Box::new(next_dialogue)),
        }
    }

    /// Constructs a simple command using only the name required
    /// for calling it, a description of what it will do, and
    /// its actual closure. Generates new dialogue from the
    /// player's current area.
    pub fn simple<F>(input: &str, output: &str, run: F) -> Command
        where F: Fn(&Vec<&str>, &PlayerMeta) + 'static
    {
        Command {
            input: String::from(input),
            output_desc: String::from(output),
            run: Box::new(run),
            next_dialogue: FromArea,
        }
    }

    /// Variant of `simple()` which does not handle generating
    /// new dialogue.
    pub fn action_only<F>(input: &str, output: &str, run: F) -> Command
        where F: Fn(&Vec<&str>, &PlayerMeta) + 'static
    {
        Command {
            input: String::from(input),
            output_desc: String::from(output),
            run: Box::new(run),
            next_dialogue: Ignore,
        }
    }

    /// Constructs a command that performs no action, refreshing
    /// the dialogue from the player's current area when run.
    pub fn text_only(input: &str, output: &str) -> Command {
        Command {
            input: String::from(input),
            output_desc: String::from(output),
            run: Box::new(|_, _| {}),
            next_dialogue: FromArea,
        }
    }

    /// Variant of `simple()` that deletes the current
    /// dialogue instead of refreshing it.
    pub fn delete_dialogue<F>(input: &str, output: &str, run: F) -> Command
        where F: Fn(&Vec<&str>, &PlayerMeta) + 'static
    {
        Command {
            input: String::from(input),
            output_desc: String::from(output),
            run: Box::new(run),
            next_dialogue: Delete,
        }
    }

    /// Variant of `simple()` that generates a new dialogue
    /// from the input closure instead of executing a
    /// process for general purposes.
    pub fn goto_dialogue<F>(input: &str, output: &str, dialogue: F) -> Command
        where F: Fn(&PlayerMeta) -> Dialogue + 'static
    {
        Command {
            input: String::from(input),
            output_desc: String::from(output),
            run: Box::new(|_, _| {}),
            next_dialogue: Generate(Box::new(dialogue)),
        }
    }

    /// The main method used for processing this command. Handles
    /// its execution, sending any possible messages to the user
    /// while blocking their input, and ultimately generating the
    /// next dialogue that will follow.
    pub fn run(&self, args: &Vec<&str>, player: &PlayerMeta, current_dialogue: &Dialogue) {
        (self.run)(args, player);
        post_run(player, current_dialogue, &self.next_dialogue);
    }

    /// Determines whether the initial value inside of
    /// `self.input` matches given string slice. Different
    /// from using `self.input.starts_with()` in that it
    /// requires the entire section to match.
    pub fn matches_input(&self, input: &str) -> bool {
        match self.input.find(" ") {
            Some(index) => &self.input[0..index] == input,
            None => &self.input == input
        }
    }

    /// Formats this response to be displayed to the user.
    pub fn get_display(&self, length: usize) -> String {
        if self.output_desc.starts_with("§") {
            let text = format!("| {} | -> {}\n", self.input, &self.output_desc[2..]);
            text::auto_break(3, length, &text)
        } else {
            format!("| {} | -> {}\n", self.input, self.output_desc)
        }
    }
}

/// Variant of `Response` which does not have a qualifier
/// and is guaranteed to consume all inputs after other
/// options have failed.
pub struct TextHandler {
    pub text: String,
    pub execute: Box<Fn(&PlayerMeta, &str) + 'static>,
    pub next_dialogue: DialogueOption,
}

impl TextHandler {
    /// The main method used for processing this option. Handles
    /// its execution, sending any possible messages to the user
    /// while blocking their input, and ultimately generating the
    /// next dialogue that will follow.
    pub fn run(&self, player: &PlayerMeta, args: &str, current_dialogue: &Dialogue) {
        (self.execute)(player, args);
        post_run(player, current_dialogue, &self.next_dialogue);
    }

    /// Formats this option to be displayed to the user.
    pub fn get_display(&self, length: usize) -> String {
        if self.text.starts_with("§") {
            let text = text::auto_break(3, length, &self.text[2..]);
            format!("_: {}", text)
        } else {
            format!("_: {}", self.text)
        }
    }
}

/// Handles sending any messages to the player, deleting
/// old dialogues, and registering new dialogues.
fn post_run(player: &PlayerMeta, current_dialogue: &Dialogue, next: &DialogueOption) {
    // Determine whether next dialogue is intended.
    let next_dialogue = match next {
        // The author supplied a function for manually
        // generating the dialogue to follow. Trust that
        // this is the right choice.
        Generate(ref d) => Some((d)(player)),
        // The author has indicated that the following dialogue
        // should come from the player's current area.
        FromArea => {
            // Ensure that the current dialogue also originates
            // from the player's area. Prevents some duplicate
            // dialogues from generating.
//            if current_dialogue.is_primary {
                Some(Dialogue::from_area(player))
//            } else {
//                player.send_current_options(); // Refresh.
//                None // To-do: log this information.
//            }
        },
        // The author indicated that the current dialogue
        // should cease to exist upon executing this function.
        Delete => {
            // Go ahead and remove the dialogue, as it can't
            // be handled below. Continue.
            delete_options(current_dialogue.id);
            player.send_current_options();
            None
        },
        // The author wishes to ignore any outcome that might
        // follow.
        Ignore => None,
    };
    if let Some(dialogue) = next_dialogue {
        // Get any possible messages from the dialogue to follow.
        let text = dialogue.text.clone();
        if let Some(ref txt) = text {
            // Send a blocking message and replace the current options.
            delete_options(current_dialogue.id);
            register_options(dialogue);
            player.update_options();
            player.send_blocking_message(txt);
        } else {
            // There is no message. Just replace and refresh.
            player.replace_send_options(current_dialogue.id, dialogue);
        }
    }
}