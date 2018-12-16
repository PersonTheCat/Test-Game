use crate::messages::MessageComponent::*;
use crate::player_data::PLAYER_META;
use crate::util::access;
use crate::types::towns;
use crate::*;

use self::ParseResult::*;

pub fn register_global_commands() {
    let mut commands = Vec::new();
    commands.push(settings_command());
    commands.push(players_command());
    commands.push(message_command());
    if CHEATS_ENABLED {
        commands.push(tp_command());
        commands.push(money_command());
        commands.push(god_command());
    }
    register_options(Dialogue::commands("Commands", commands, GLOBAL_USER));
}

/// Teleports the player. Cannot display entrance message.
/// Usage: `tp [<town #> | <area_type>]`
/// Examples: `tp 2`, `tp station`
fn tp_command() -> Command {
    Command::action_only(
        "tp #", "Teleport to town #.",
        |args, player| {
        if args.len() < 1 {
            player.add_short_message("Error: Missing town #.");
            return;
        }
        let tp_result = match args[0].parse() {
            Ok(town_num) => tp_player_to_town(player, town_num),
            Err(_) => tp_player_to_area(player, args[0]),
        };
        if let Err(e) = tp_result {
            player.send_short_message(e);
        } else {
            player.get_send_area_options();
        }
    })
}

/// Handles transporting the player when the input
/// refers to a town number.
fn tp_player_to_town(player: &PlayerMeta, town_num: usize) -> Result<(), &'static str> {
    let (x, z) = towns::STARTING_COORDS;
    tp_player(player, (town_num, x, z))
}

/// Handles transporting the player when the input
/// refers to a specific area type.
fn tp_player_to_area(player: &PlayerMeta, location: &str) -> Result<(), &'static str> {
    match player.town().locate_area(location) {
        Some(coords) => tp_player(player, coords),
        None => return Err("Your town does not contain this kind of area.")
    }
}

/// The actual process responsible for sending the
/// player from one area to another.
fn tp_player(player: &PlayerMeta, coords: (usize, usize, usize)) -> Result<(), &'static str> {
    if coords == player.get_coordinates() {
        return Err("There is nowhere to go.");
    }
    // We have to manually update their dialogue.
    if let Err(_) = try_delete_options(player.get_player_id()) {
        return Err("Currently unable to handle player dialogue.");
    }
    player.area(|old| {
        access::area(coords, |new| {
            old.transfer_to_area(player.get_player_id(), new);
        });
    });
    Ok(())
}

/// Gives or takes money from the player.
/// Usage: `money <amount>`
/// Examples: `money 1000`, `money -1000`
fn money_command() -> Command {
    Command::action_only(
        "money #", "Get # money.",
        |args, player| {
        // Make sure the first parameter is specified.
        if args.len() < 1 {
            player.send_short_message("Error: You need to specify how much.");
            return;
        }
        // Attempt to read the quantity from the user's input.
        let quantity: i32 = match args[0].parse() {
            Ok(num) => num,
            Err(_) => {
                player.send_short_message("Unable to parse arguments.");
                return;
            }
        };
        // Access the entity and determine whether to add or remove money.
        player.entity(|e| {
            if quantity > 0 {
                e.give_money(quantity as u32);
            } else {
                e.take_money((quantity * -1) as u32);
            }
        });
        player.send_current_options();
    })
}

/// Changes the player's god. Case sensitive.
/// Usage: `god <god_name>`
/// Examples: `god Danu`
fn god_command() -> Command {
    Command::action_only("god x", "Change your god to x.", |args, player| {
        // Make sure the first parameter is specified.
        if args.len() < 1 {
            player.send_short_message("Error: You need to specify which one.");
            return;
        }
        // Inform the user and update their god.
        player.send_short_message(&format!("Setting your god to {}.", args[0]));
        player.set_god(args[0].to_string());
    })
}

/// Opens the player's settings dialogue. Allowing them
/// clearer access to certain in-game settings.
/// Usage: `settings [open]`
/// Examples: `settings`, `settings open`
fn settings_command() -> Command {
    Command::action_only("settings", "Change your settings.", |args, player| {
        let settings = settings_dialogue(player);
        // Automatically close the menu if they don't type "open."
        if args.len() == 0 || args[0] != "open" {
            Dialogue::delete_in(player.get_player_id(), settings.id, TEMP_DIALOGUE_DURATION);
        } else {
            player.add_short_message("Your settings dialogue will stay open.");
        }
        register_options(settings);
        player.send_current_options();
    })
}

/// Generates the actual settings dialogue.
fn settings_dialogue(player: &PlayerMeta) -> Dialogue {
    Dialogue {
        title: String::from("Player Settings"),
        info: Some(String::from("Use `<cmd> reset` to reset this setting.")),
        responses: vec![close_settings()],
        commands: vec![text_speed_command(), text_length_command()],
        player_id: player.get_player_id(),
        ..Dialogue::default()
    }
}

/// An empty response used for closing the
/// settings dialogue.
fn close_settings() -> Response {
    Response::delete_dialogue("Close Settings", |_| {})
}

/// Changes the player's text speed.
/// Usage: `tspeed [<val 1-5> | reset]`
/// Examples: `tspeed 3`, `tspeed reset`
fn text_speed_command() -> Command {
    Command::action_only(
        "tspeed #", "§Sets your text speed to #, 1-5.",
        |args, player| {
            match parse_first_argument(args) {
                Number(num) => set_text_speed(player, num),
                Reset => set_text_speed(player, 3),
                TooShort => player.send_short_message("You need to specify the speed."),
                _ => player.send_short_message("Unable to parse arguments.")
            };
        }
    )
}

fn set_text_speed(player: &PlayerMeta, input: i32) {
    match input {
        1 ... 5 => {
            let msg = format!("Setting your text speed to {}", input);
            player.send_short_message(&msg);
            player.set_text_speed((1000 * input as u64) - 500);
        },
        _ => player.send_short_message("tspeed expects a value between 1 and 5.")
    };
}

/// Changes the player's line length.
/// Usage: `tlength [<val 40-150> | reset]`
/// Examples: `tlength 60`, `tlength reset`
fn text_length_command() -> Command {
    Command::action_only(
        "tlength #", "§Sets your line length to #, 40-150.",
        |args, player|{
            match parse_first_argument(args) {
                Number(num) => set_text_length(player, num),
                Reset => set_text_length(player, LINE_LENGTH as i32),
                TooShort => player.send_short_message("You need to specify the text length."),
                _ => player.send_short_message("Unable to parse arguments.")
            };
        })
}

fn set_text_length(player: &PlayerMeta, input: i32) {
    match input {
        40 ... 150 => {
            let msg = format!("Setting your text length to {}", input);
            player.send_short_message(&msg);
            player.set_text_length(input as usize);
        },
        _ => player.send_short_message("tlength expects a value between 40 and 150.")
    };
}

/// The result of parsing an argument for the
/// entire settings dialogue.
enum ParseResult {
    Number(i32),
    Boolean(bool),
    Reset,
    TooShort,
    NoMatch
}

/// Parses the first argument in `args` and returns
/// the data.
fn parse_first_argument(args: &Vec<&str>) -> ParseResult {
    if args.len() < 1 {
        TooShort
    } else if let Ok(num) = args[0].parse::<i32>() {
        Number(num)
    } else {
        match args[0].to_lowercase().as_str() {
            "reset" => Reset,
            "true" | "t" => Boolean(true),
            "false" | "f" => Boolean(false),
            _ => NoMatch
        }
    }
}

/// Displays all currently-connected players and
/// their locations.
/// Usage: `players`
fn players_command() -> Command {
    Command::action_only(
        "players", "Display all active players.",
        |_args, player| {
            let message = get_players_message();
            player.send_message(General, &message);
    })
}

fn get_players_message() -> String {
    let mut message = String::from("Connected players:");

    PLAYER_META.lock()
        .iter()
        .filter(|p| p.is_active())
        .for_each(|p| {
            let coords = p.get_coordinates();
            let area_name = access::area(coords, |a| a.get_title()).unwrap();
            message += &format!("\n * {} (T: {}; A: {})", p.get_name(), coords.0, area_name);
        });
    message
}

/// Usage: `msg <username> [<message>]`
/// Examples: `msg personthecat Hello, world.`
fn message_command() -> Command {
    Command::action_only(
        "msg x","Send a message to x (username).",
        |args, player| {
            if args.len() < 1 {
                player.send_short_message("Error: You need to specify a username.");
            } else if args.len() < 2 {
                player.send_short_message("Error: No message to send.");
            }

            let mut iter = args.iter();
            let _username = iter.next().unwrap();

            player.send_short_message("To-do: Come back to this when Discord is integrated.",);
        },
    )
}