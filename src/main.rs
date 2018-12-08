#![allow(dead_code, unused_doc_comments)]
#![feature(duration_as_u128)]
#![feature(drain_filter)]

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate test_game_derive;

pub mod traits;
pub mod types;
pub mod util;
pub mod text;
pub mod messages;
pub mod player_data;

/**
 * Very common methods
 */

pub use self::player_options::DialogueOption::*;
pub use self::text::choose;
pub use self::player_options::*;
pub use self::messages::*;

/**
 * Normal imports
 */

use self::util::{
    access::{ self, EntityAccessor },
    player_options::{ self, DialogueResult::* },
    timed_events
};

use self::player_data::{ PlayerMeta, PLAYER_META };
use self::messages::MessageComponent::*;
use self::types::areas::area_settings;
use self::types::items::item_settings;
use self::messages::ChannelInfo::*;
use self::types::towns;

use std::{
    time::SystemTime,
    io,
    thread,
    process,
    sync::mpsc::{ self, Receiver, Sender },
};

/**
 * Conditional imports
 */

#[cfg(feature = "discord")]
use self::util::discord_bot::Bot;
#[cfg(feature = "remote_clients")]
use self::util::server_host;

/**
 * Settings.
 */

const UPDATES_PER_SECOND: u16 = 10;
const NUM_SPACES: u8 = 50; // Separate by lines until a TUI is implemented.
const MAX_SHORT_MESSAGES: usize = 3;
pub const TEXT_SPEED: u64 = 1500;
pub const TEMP_DIALOGUE_DURATION: u64 = 20_000;
pub const LINE_LENGTH: usize = 40; // Should probably be no lower than this.
const PRINT_FRAMES: bool = false;
const CHEATS_ENABLED: bool = true;

/**
 * Don't edit these.
 */

static mut GAME_TIME: u128 = 0;
const MS_BETWEEN_UPDATES: u16 = 1000 / UPDATES_PER_SECOND;

fn main()
{
    pre_init();
    init();
    run();
}

fn pre_init()
{
    unsafe
    {
        player_options::setup_option_registry();
        player_data::setup_player_registry();
        timed_events::setup_event_registry();
        towns::setup_town_registry();
        area_settings::setup_area_registry();
        item_settings::setup_item_pools();
    }
}

fn init()
{
    area_settings::register_vanilla_settings();
    item_settings::register_vanilla_settings();
    register_global_commands();
}

fn run()
{
    let mut last_update = SystemTime::now();
    let mut is_running = true;

    let input = handle_inputs();

    println!("\nStarting game loop. Press enter to begin...");

    loop
    {
        let (time_since_last_update, can_continue) = can_continue(&mut last_update);

        if can_continue
        {
            let message = input.try_iter().next();

            if let Some(ref msg) = message
            {
                handle_global_commands(msg, &mut is_running);
            }
            if is_running
            {
                update_game_time(time_since_last_update);

                timed_events::update_timed_events();

                if let Some(msg) = message
                {
                    handle_player_commands(&msg);
                }
                if PRINT_FRAMES { println!("Game time: {} ms.", game_time()); }
            }
        }
    }
}

pub fn game_time() -> u128
{
    unsafe { GAME_TIME }
}

fn update_game_time(add: u128)
{
    unsafe { GAME_TIME += add; }
}

fn can_continue(last_update: &mut SystemTime) -> (u128, bool)
{
    let current_time = SystemTime::now();

    let difference = current_time
        .duration_since(*last_update)
        .expect("Error: Unable to update current time.")
        .as_millis();

    if difference >= MS_BETWEEN_UPDATES as u128
    {
        *last_update = current_time;

        (difference, true)
    }
    else { (difference, false) }
}

fn handle_inputs() -> Receiver<GameMessage>
{
    let (tx , rx) = mpsc::channel();
    handle_stdio(tx.clone());
    handle_discord(tx.clone());
    handle_server(tx);
    rx
}

fn handle_stdio(tx: Sender<GameMessage>)
{
    thread::spawn(move ||
    {
        loop
        {
            let mut input = String::new();

            io::stdin().read_line(&mut input)
                .expect("Error: Unable to parse input.");

            let message = GameMessage
            {
                message: input.trim().to_string(),
                channel_info: Local
            };

            tx.send(message)
                .expect("Error: Unable to send message.");
        }
    });
}

#[cfg(feature = "discord")]
fn handle_discord(tx: Sender<GameMessage>)
{
    thread::spawn(move || Bot::load(tx));
}

#[cfg(not(feature = "discord"))]
fn handle_discord(_tx: Sender<GameMessage>) {}

#[cfg(feature = "remote_clients")]
fn handle_server(tx: Sender<GameMessage>)
{
    thread::spawn(move || server_host::init_listener(tx));
}

#[cfg(not(feature = "remote_clients"))]
fn handle_server(_tx: Sender<GameMessage>) {}

pub struct GameMessage
{
    pub message: String,
    pub channel_info: ChannelInfo
}

/**
 * These commands will be processed even when the game is paused.
 */
fn handle_global_commands(message: &GameMessage, is_running: &mut bool)
{
    match message.message.as_str()
    {
        "pause" | "p" => toggle_pause(is_running),
        "end" | "quit" => process::exit(0),
        _ => {}
    }
}

fn toggle_pause(is_running: &mut bool)
{
    if *is_running { *is_running = false; }
    else { *is_running = true; }
    println!("Game is now {}.", if *is_running { "unpaused" } else { "paused" });
}

fn handle_player_commands(message: &GameMessage)
{
    let response =

    access::player_meta_sender(&message.channel_info, | meta |
    {
        process_options(meta.player_id, &message.message);
    });

    if let None = response
    {
        player_data::new_player_event(message);
    }
}

/**
 * To-do: Update this to potentially send error messages
 * and ensure that players have dialogue.
 */
fn process_options(player_id: usize, input: &str)
{
    unsafe { if let Some(ref registry) = CURRENT_OPTIONS
    {
        let matches = registry.iter().filter(| option |
        {
            option.player_id == player_id || option.player_id == GLOBAL_USER
        });

        // No matches? -> get from area

        let mut start_at = 1;

        for option in matches
        {
            let response = option.run_as_user(input, player_id, start_at);

            match response
            {
                Success => return,
                NoArgs => continue,
                NoneFound => continue,
                InvalidNumber(max) => { start_at += max; continue;},
            };
        }
    }
    else { panic!("Error: Option registry not loaded in time."); }}
}

pub fn get_accessor_for_sender(channel: ChannelInfo) -> Option<EntityAccessor>
{
    unsafe { if let Some(ref registry) = player_data::PLAYER_META
    {
        let index = registry.iter().position(| meta |
        {
            meta.channel == channel
        });

        match index
        {
            Some(num) => return Some(registry[num].get_accessor()),
            None => return None
        };
    }}
    panic!("Error: Player meta registry not established in time. Unable to retrieve access.");
}

/**
 * Cheats / debugging
 */
fn register_global_commands()
{
    register_options(Dialogue::commands(
        "Commands",
        if CHEATS_ENABLED { vec![
            tp_command(),
            money_command(),
            god_command(),
            settings_command(),
            players_command(),
            message_command()
        ]} else { vec![
            settings_command(),
            players_command(),
            message_command()
        ]},
        GLOBAL_USER
    ));
}

/**
 * Will not display entrance message.
 */
fn tp_command() -> Command
{
    Command::manual_desc_no_next("tp", "tp #", "Teleport to town #.",
     | args, player_id |
     {
         if args.len() < 1 { add_short_message(player_id, "Error: Missing town #."); return; }

         let result = match args[0].parse()
         {
             Ok(town_num) => tp_player_to_town(player_id, town_num),
             Err(_) => tp_player_to_area(player_id, args[0])
         };

         if let Err(e) = result
         {
             send_short_message(player_id, e);
         }
         else { get_send_area_options(player_id); }
     })
}

fn tp_player_to_town(player_id: usize, town_num: usize) -> Result<(), &'static str>
{
    access::player_meta(player_id, |player |
    {
        let (x, z) = towns::STARTING_COORDS;

        tp_player(player, (town_num, x, z))
    })
    .expect("Player data no longer exists.")
}

fn tp_player_to_area(player_id: usize, location: &str) -> Result<(), &'static str>
{
    access::player_meta(player_id, |player |
    {
        let new_coords = match access::town(player.coordinates.0, |old_town |
        {
            old_town.locate_area(location)
        }){
            Some(coords) => coords,
            None => return Err("Your town does not contain this kind of area.")
        };

        tp_player(player, new_coords)
    })
    .expect("Player data no longer exists.")
}

fn tp_player(player: &PlayerMeta, coords: (usize, usize, usize)) -> Result<(), &'static str>
{
    if coords == player.coordinates { return Err("There is nowhere to go."); }

    if let Err(_) = try_delete_options(player.player_id)
    {
        return Err("Currently unable to handle player dialogue.");
    }

    access::area(player.coordinates, |old_area |
    {
        access::area(coords, |new_area |
        {
            old_area.transfer_to_area(player.player_id, new_area);
        });
    });
    Ok(())
}

fn money_command() -> Command
{
    Command::manual_desc_no_next("money", "money #", "Get # money.",
     | args, player_id |
     {
         if args.len() < 1 { send_short_message(player_id, "Error: You need to specify how much."); return; }

         let quantity: i32 = match args[0].parse()
         {
             Ok(num) => num,
             Err(_) =>
             {
                 send_short_message(player_id, "Unable to parse arguments.");
                 return;
             }
         };

         access::player(player_id, | entity |
         {
             if quantity > 0
             {
                 entity.give_money(quantity as u32);
             }
             else { entity.take_money((quantity * -1) as u32); }

             send_current_options(player_id);
         });
     })
}

fn god_command() -> Command
{
    Command::manual_desc_no_next("god", "god x", "Change your god to x.",
     | args, player_id |
     {
         if args.len() < 1 { send_short_message(player_id, "Error: You need to specify which one."); return; }

         send_short_message(player_id, &format!("Setting your god to {}.", args[0]));

         access::player_meta(player_id, |player |
         {
             player.god = args[0].to_string();
         });
     })
}

fn settings_command() -> Command
{
    Command::action_only("settings", "Change your settings.",
    | args, player_id |
    {
        let delete = if args.len() > 0
        { args[0] != "open" } else { true };

        access::player_meta(player_id, move | player |
        {
            let settings = settings_dialogue(player);
            let settings_id = settings.id;
            register_options(settings);

            if delete { Dialogue::delete_in(player_id, settings_id, TEMP_DIALOGUE_DURATION); }
            else { add_short_message(player_id, "Your settings dialogue will stay open."); }
        });
        send_current_options(player_id);
    })
}

fn settings_dialogue(player: &mut PlayerMeta) -> Dialogue
{
    Dialogue::no_message
    (
        "Player Settings",
        vec![
            close_settings()
        ],
        vec![
            text_speed_command(),
            text_length_command()
        ],
        player.player_id
    )
}

fn close_settings() -> Response
{
    Response::delete_dialogue("Close Settings", |_|{})
}

fn text_speed_command() -> Command
{
    Command::text_only("tspeed", "tspeed", "To-do. Use main.rs.")
}

fn text_length_command() -> Command
{
    Command::text_only("tlength", "tlength", "To-do. Use main.rs.")
}

fn players_command() -> Command
{
    Command::manual_desc_no_next("players", "players", "Display all active players.",
    | _args, player_id |
    {
        let message = get_players_message();
        send_message_to_player(player_id, General, &message, 0);
    })
}

fn get_players_message() -> String
{
    unsafe { if let Some(ref registry) = PLAYER_META
    {
        let mut message = String::from("Connected players:");

        let iter = registry.iter()
            .filter(| p | p.active)
            .enumerate();

        for (index, player) in iter
        {
            if index % 2 == 0 { message += "\n "; }
            message += " * ";
            message += &player.name;
            message += " ";
            message += &format!("{:?}", player.coordinates);
        }
        message += "To-do: Display as T: #; A: <name>.";
        return message;
    }
    else { panic!("Player registry was not setup in time."); }}
}

fn message_command() -> Command
{
    Command::manual_desc_no_next("msg", "msg x", "Send a message to x (username).",
    | args, player_id |
    {
        if args.len() < 1 { send_short_message(player_id, "Error: You need to specify a username."); }
        else if args.len() < 2 { send_short_message(player_id, "Error: No message to send."); }

        let mut iter = args.iter();
        let _username = iter.next().unwrap();

        send_short_message(player_id, "To-do: Come back to this when Discord is integrated.");
    })
}