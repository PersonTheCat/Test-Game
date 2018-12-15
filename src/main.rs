#![feature(drain_filter)]
#![allow(dead_code)]

#[macro_use]
extern crate test_game_derive; // To-do: convert to `use` statements.

pub mod messages;
pub mod player_data;
pub mod text;
pub mod traits;
pub mod types;
pub mod util;
pub mod global_commands;

/// //////////////////////////////////////////////////
///            # Very Common Functions
/// //////////////////////////////////////////////////

pub use self::messages::*;
pub use self::player_data::PlayerMeta;
pub use self::player_options::DialogueOption::*;
pub use self::player_options::*;
pub use self::text::choose;

/// //////////////////////////////////////////////////
///               # Normal Imports
/// //////////////////////////////////////////////////

use self::util::{
    access,
    player_options::{self, DialogueResult::*},
    timed_events,
};

use self::messages::ChannelInfo::*;
use self::types::areas::area_settings;
use self::types::items::item_settings;

use std::{
    io, process,
    sync::atomic::Ordering::SeqCst,
    sync::mpsc::{self, Receiver, Sender},
    sync::Arc,
    thread,
};

use lazy_static::lazy_static;
use atomic::Atomic;
use time;

/// //////////////////////////////////////////////////
///            # Conditional Imports
/// //////////////////////////////////////////////////

#[cfg(feature = "discord")]
use self::util::discord_bot::Bot;
#[cfg(feature = "remote_clients")]
use self::util::server_host;

/// //////////////////////////////////////////////////
///                  # Settings
/// //////////////////////////////////////////////////

const UPDATES_PER_SECOND: u16 = 10;
const NUM_SPACES: u8 = 50; // Separate by lines until a TUI is implemented.
const MAX_SHORT_MESSAGES: usize = 3;
pub const TEXT_SPEED: u64 = 500;
pub const TEMP_DIALOGUE_DURATION: u64 = 20_000;
pub const LINE_LENGTH: usize = 40; // Should probably be no lower than 40.
const PRINT_FRAMES: bool = false;
const CHEATS_ENABLED: bool = true;

// Don't edit these.
const MS_BETWEEN_UPDATES: u16 = 1000 / UPDATES_PER_SECOND;

lazy_static! {
    /// A global singleton used for updating the current
    /// time in-game.
    static ref GAME_TIME: Atomic<u64> = Atomic::new(0);
}

/// The main function and primary event handler.
fn main() {
    pre_init();
    init();
    run();
}

/// To-do: Handle initializing registries from save data.
fn pre_init() {
    // player_options::setup_option_registry();
    // area_settings::setup_area_registry();
    // item_settings::setup_item_pools();
    // player_data::setup_player_registry();
    // towns::setup_town_registry();
}

/// Registers vanilla settings into the various registries.
fn init() {
    area_settings::register_vanilla_settings();
    item_settings::register_vanilla_settings();
    global_commands::register_global_commands();
}

/// Starts the main game loop.
fn run() {
    // Local variable declarations for managing
    // the current game state.
    let mut last_update = current_time();
    let mut is_running = true;
    let input = handle_inputs();

    println!("\nStarting game loop. Press enter to begin...");

    loop {
        // Updates that happen constantly.
        let time_since_update = time_since(last_update);

        // Updates that occur on a limited time interval.
        if can_continue(time_since_update) {
            // Use the reported delay since `last_update` to update
            // the current real-world time.
            last_update += time_since_update;
            // Attempt to process one message from a user.
            let message = input.try_iter().next();

            if let Some(ref msg) = message {
                // Always process global commands, regardless of
                // whether the game `is_running`.
                handle_global_commands(msg, &mut is_running);
            }
            if is_running {
                // Updates the current game-time using the reported
                // `time_since_update`.
                GAME_TIME.store(game_time() + time_since_update, SeqCst);
                // Process all current timed-events in the current
                // thread only.
                timed_events::update_timed_events();

                if let Some(msg) = message {
                    // Manage player dialogue using the received
                    // `GameMessage`.
                    handle_player_commands(&msg);
                }
                if PRINT_FRAMES {
                    println!("Game time: {} ms.", game_time());
                }
            }
        }
    }
}

/// Returns the interval in milliseconds since the input
/// `last_update`.
fn time_since(last_update: u64) -> u64 {
    let current_time = current_time();
    current_time - last_update
}

/// Determines whether sufficient time has passed for the
/// main game loop to continue.
fn can_continue(time_since_update: u64) -> bool {
    time_since_update >= MS_BETWEEN_UPDATES as u64
}

/// A public accessor which reports the current game time.
pub fn game_time() -> u64 {
    GAME_TIME.load(SeqCst)
}

/// Retrieves the current real-world time in milliseconds.
fn current_time() -> u64 {
    time::precise_time_ns() / 1_000_000
}

/// Spawns a channel for sending messages into the main
/// game thread through various sources.
fn handle_inputs() -> Receiver<GameMessage> {
    let (tx, rx) = mpsc::channel();
    handle_stdio(tx.clone());
    handle_discord(tx.clone());
    handle_server(tx);
    rx
}

/// A simple loop which awaits inputs from the user
/// via the standard input stream.
fn handle_stdio(tx: Sender<GameMessage>) {
    thread::spawn(move || loop {
        let mut input = String::new();

        io::stdin().read_line(&mut input)
            .expect("Error: Unable to parse input.");

        let message = GameMessage {
            message: input.trim().to_string(),
            channel_info: Local,
        };

        tx.send(message)
            .expect("Error: Unable to send message.");
    });
}

/// An optional method that spawns the discord bot and
/// triggers it to listen for `GameMessage`s. Most likely
/// does not work, at the moment.
#[cfg(feature = "discord")]
fn handle_discord(tx: Sender<GameMessage>) {
    thread::spawn(move || Bot::load(tx));
}

#[cfg(not(feature = "discord"))]
fn handle_discord(_tx: Sender<GameMessage>) {}

/// An optional method that spawns the dedicated server
/// and triggers it to listen for `GameMessage`s.
#[cfg(feature = "remote_clients")]
fn handle_server(tx: Sender<GameMessage>) {
    thread::spawn(move || server_host::init_listener(tx));
}

#[cfg(not(feature = "remote_clients"))]
fn handle_server(_tx: Sender<GameMessage>) {}

/// The actual contents of messages that will be sent into
/// the main game thread, containing the actual message
/// and information regarding its origins.
pub struct GameMessage {
    pub message: String,
    pub channel_info: ChannelInfo,
}


/// global commands to be used even when the game is paused.
fn handle_global_commands(message: &GameMessage, is_running: &mut bool) {
    match message.message.as_str() {
        "pause" | "p" => toggle_pause(is_running),
        "end" | "quit" => process::exit(0),
        _ => {}
    }
}

/// Pauses or unpauses the game and reports the updated
/// status to the local output stream.
fn toggle_pause(is_running: &mut bool) {
    *is_running = !*is_running;
    println!("Game is now {}.",
        if *is_running { "unpaused" } else { "paused" }
    );
}

/// Processes game messages sent the main game thread
/// by retrieving the respective player's context and
/// forwarding it to `process_options()`.
fn handle_player_commands(message: &GameMessage) {
    match access::player_meta_sender(&message.channel_info) {
        Some(player) => process_options(&*player, &message.message),
        None => player_data::new_player_event(message)
    }
}

/// To-do: Update this to potentially send error messages
/// and ensure that players have dialogue.
fn process_options(player: &PlayerMeta, input: &str) {
    // Clone references out of the lock to release it
    // and allow it to be reused.
    let matches: Vec<Arc<Dialogue>> = CURRENT_OPTIONS.lock()
        .iter()
        .filter(|o| o.player_id == player.get_player_id() || o.player_id == GLOBAL_USER)
        .map(|o| o.clone())
        .collect();

    let mut start_at = 1;
    for option in matches {
        match option.run(input, player, start_at) {
            Success => return,
            NoArgs => continue,
            NoneFound => continue,
            InvalidNumber(max) => {
                start_at += max;
                continue;
            }
        };
    }
}
