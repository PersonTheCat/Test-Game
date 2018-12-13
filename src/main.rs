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
use self::types::towns;

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
pub const TEXT_SPEED: u64 = 3000;
pub const TEMP_DIALOGUE_DURATION: u64 = 20_000;
pub const LINE_LENGTH: usize = 40; // Should probably be no lower than this.
const PRINT_FRAMES: bool = false;
const CHEATS_ENABLED: bool = true;

// Don't edit these.
const MS_BETWEEN_UPDATES: u16 = 1000 / UPDATES_PER_SECOND;

lazy_static! {
    static ref GAME_TIME: Atomic<u64> = Atomic::new(0);
}

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

    unsafe {
        towns::setup_town_registry();
    }
}

fn init() {
    area_settings::register_vanilla_settings();
    item_settings::register_vanilla_settings();
    global_commands::register_global_commands();
}

fn run() {
    let mut last_update = current_time();
    let mut is_running = true;
    let input = handle_inputs();

    println!("\nStarting game loop. Press enter to begin...");

    loop {
        let (time_since_last_update, can_continue) = can_continue(&mut last_update);

        if can_continue {
            let message = input.try_iter().next();

            if let Some(ref msg) = message {
                handle_global_commands(msg, &mut is_running);
            }
            if is_running {
                GAME_TIME.store(game_time() + time_since_last_update, SeqCst);
                timed_events::update_timed_events();

                if let Some(msg) = message {
                    handle_player_commands(&msg);
                }
                if PRINT_FRAMES {
                    println!("Game time: {} ms.", game_time());
                }
            }
        }
    }
}

fn can_continue(last_update: &mut u64) -> (u64, bool) {
    let current_time = current_time();
    let difference = current_time - *last_update;

    if difference >= MS_BETWEEN_UPDATES as u64 {
        *last_update = current_time;
        (difference, true)
    } else {
        (difference, false)
    }
}

pub fn game_time() -> u64 {
    GAME_TIME.load(SeqCst)
}

fn current_time() -> u64 {
    time::precise_time_ns() / 1_000_000
}

fn handle_inputs() -> Receiver<GameMessage> {
    let (tx, rx) = mpsc::channel();
    handle_stdio(tx.clone());
    handle_discord(tx.clone());
    handle_server(tx);
    rx
}

fn handle_stdio(tx: Sender<GameMessage>) {
    thread::spawn(move || loop {
        let mut input = String::new();

        io::stdin()
            .read_line(&mut input)
            .expect("Error: Unable to parse input.");

        let message = GameMessage {
            message: input.trim().to_string(),
            channel_info: Local,
        };

        tx.send(message)
            .expect("Error: Unable to send message.");
    });
}

#[cfg(feature = "discord")]
fn handle_discord(tx: Sender<GameMessage>) {
    thread::spawn(move || Bot::load(tx));
}

#[cfg(not(feature = "discord"))]
fn handle_discord(_tx: Sender<GameMessage>) {}

#[cfg(feature = "remote_clients")]
fn handle_server(tx: Sender<GameMessage>) {
    thread::spawn(move || server_host::init_listener(tx));
}

#[cfg(not(feature = "remote_clients"))]
fn handle_server(_tx: Sender<GameMessage>) {}

pub struct GameMessage {
    pub message: String,
    pub channel_info: ChannelInfo,
}


/// These commands will be processed even when the game is paused.
fn handle_global_commands(message: &GameMessage, is_running: &mut bool) {
    match message.message.as_str() {
        "pause" | "p" => toggle_pause(is_running),
        "end" | "quit" => process::exit(0),
        _ => {}
    }
}

fn toggle_pause(is_running: &mut bool) {
    *is_running = !*is_running;
    println!("Game is now {}.",
        if *is_running { "unpaused" } else { "paused" }
    );
}

fn handle_player_commands(message: &GameMessage) {
    match access::player_meta_sender(&message.channel_info) {
        Some(player) => process_options(&*player, &message.message),
        None => player_data::new_player_event(message)
    }
}


// To-do: Update this to potentially send error messages
// and ensure that players have dialogue.
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
