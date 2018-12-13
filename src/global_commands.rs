use crate::messages::MessageComponent::*;
use crate::player_data::PLAYER_META;
use crate::util::access;
use crate::types::towns;
use crate::*;

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
fn tp_command() -> Command {
    Command::manual_desc_no_next("tp", "tp #", "Teleport to town #.", |args, player| {
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

fn tp_player_to_town(player: &PlayerMeta, town_num: usize) -> Result<(), &'static str> {
    let (x, z) = towns::STARTING_COORDS;
    tp_player(player, (town_num, x, z))
}

fn tp_player_to_area(player: &PlayerMeta, location: &str) -> Result<(), &'static str> {
    match player.town(|t| t.locate_area(location)){
        Some(coords) => tp_player(player, coords),
        None => return Err("Your town does not contain this kind of area.")
    }
}

fn tp_player(player: &PlayerMeta, coords: (usize, usize, usize)) -> Result<(), &'static str> {
    if coords == player.get_coordinates() {
        return Err("There is nowhere to go.");
    }
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

fn money_command() -> Command {
    Command::manual_desc_no_next("money", "money #", "Get # money.", |args, player| {
        if args.len() < 1 {
            player.send_short_message("Error: You need to specify how much.");
            return;
        }
        let quantity: i32 = match args[0].parse() {
            Ok(num) => num,
            Err(_) => {
                player.send_short_message("Unable to parse arguments.");
                return;
            }
        };
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

fn god_command() -> Command {
    Command::manual_desc_no_next("god", "god x", "Change your god to x.", |args, player| {
        if args.len() < 1 {
            player.send_short_message("Error: You need to specify which one.");
            return;
        }
        player.send_short_message(&format!("Setting your god to {}.", args[0]));
        player.set_god(args[0].to_string());
    })
}

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

fn settings_dialogue(player: &PlayerMeta) -> Dialogue {
    Dialogue::no_message(
        "Player Settings",
        vec![close_settings()],
        vec![text_speed_command(), text_length_command()],
        player.get_player_id(),
    )
}

fn close_settings() -> Response {
    Response::delete_dialogue("Close Settings", |_| {})
}

fn text_speed_command() -> Command {
    Command::text_only("tspeed", "tspeed", "To-do. Use main.rs.")
}

fn text_length_command() -> Command {
    Command::text_only("tlength", "tlength", "To-do. Use main.rs.")
}

fn players_command() -> Command {
    Command::action_only("players", "Display all active players.",|_args, player| {
        let message = get_players_message();
        player.send_message(General, &message, 0);
    })
}

fn get_players_message() -> String {
    let mut message = String::from("Connected players:");

    PLAYER_META.lock()
        .iter()
        .filter(|p| p.is_active())
        .enumerate()
        .for_each(|(i, p)| {
            if i % 2 == 0 {
                message += "\n";
            }
            message += &format!(" * {} {:?}", p.get_name(), p.get_coordinates());
        });

    message += "\nTo-do: Display as T: #; A: <name>.";
    return message;
}

fn message_command() -> Command {
    Command::manual_desc_no_next(
        "msg", "msg x","Send a message to x (username).",
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