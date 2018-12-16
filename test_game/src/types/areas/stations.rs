use crate::player_data::PlayerMeta;
use crate::text;
use crate::traits::{Area, Entity, Item};
use crate::types::classes::Class;
use crate::types::items::pass_books::PassBook;
use crate::util::access;
use crate::util::player_options::{Command, Dialogue, Response};
use crate::*;

use rand::{thread_rng, Rng};
use parking_lot::RwLock;
use parking_lot::Mutex;

static ENTRANCE_TEXT: [&str; 5] = [
    "§Welcome to station #<station>. Our trains can make it \
     as far as <south>km south, while our north-bound travels \
     are going as far as <north>km.∫ Ask the conductor to find \
     out we're going.",
    "§Welcome to station #<station>. These trains are going \
     down about <south>km from here. We're also travelling \
     all the way up as far as <north> km.∫ Ask the conductor \
     for more information.",
    "§Welcome to station #<station>. These old rails can \
     make it anywhere from <south>km south to <north>km north.∫ \
     Feel free to ask the conductor for more information about \
     our travels.",
    "§Hello and welcome to station #<station>. Our conductor's \
     fares will go as far as <south>km south and <north>km north.∫ \
     Enjoy your travels and don't hesitate to ask the \
     conductor for more information.",
    "§Hello, there! Welcome to station #<station>. We're \
     currently offering travels south-bound as far as \
     <south>km from here, and roughly <north>km north-bound.∫ \
     Enjoy your travels and feel free to speak to the \
     conductor, if you need anything else.",
];

static TRAVEL_PASS_INFO_TEXT: [&str; 3] = [
    "§You can use travel passes to travel between towns. \
     Travel passes are stored inside of a booklet, which \
     is fairly cheap and can hold up to five passes at a \
     time.∫ Depending on the class you purchase, passes \
     can be reused for free until they run out.",
    "§Travel passes can be used to travel between towns. \
     You can hold up to five passes inside of a travel \
     booklet, which can be purchased for fairly cheap.∫ \
     Depending on which class you purchase, you'll be \
     able to reuse them for free until they run out.",
    "§We sell travel passes, which can be used at any \
     station to travel between towns. You can hold \
     up to five of these passes inside of a travel \
     booklet.∫ Each pass can be reused a certain \
     number of times, depending on which class you \
     purchase.",
];

static PASS_PURCHASE_INFO_TEXT: [&str; 3] = [
    "§We're currently selling passes at about <rate>g per km.∫0.5 \
     If you need a booklet to hold more, you can buy one \
     for about <booklet>g.",
    "§Our travel passes are currently going for about <rate>g \
     per km.∫0.5 if your booklet is running low on space or \
     if you need to purchase a new one, you can buy one from us \
     for about <booklet>g.",
    "§We sell travel passes for roughly <rate>g per km.∫0.5 \
     If needed, you can also buy a booklet for about <booklet>g.",
];

static PASS_PURCHASE_TEXT: [&str; 3] = [
    "§We'll sell you a pass for anywhere from town #<south> \
     to town #<north>. The current rate per town is <rate>g.∫0.5 \
     If you want to purchase a reusable ticket, just let \
     me know and I'll give you an upgrade.",
    "§Looks like we're selling tickets for anywhere from \
     town #<south> to town #<north>. They're going at \
     about <rate>g per town.∫0.5 If you want me to upgrade \
     your ticket so you can reuse it, just let me know and \
     I'll see what I can arrange.",
    "§We're currently offering travel passes that will take \
     you from town #<south> to town #<north>. But it's not \
     easy going long distance, so each kilometer will cost \
     you an extra <rate>g.∫0.5 If you like, I can upgrade \
     your pass for you so you can reuse it later on. Just \
     say so, if that's what you need.",
];

static PASS_USE_TEXT: [&str; 3] = [
    "§Very well. Just let me know the number of the town \
     you'd like to travel to and we'll set off.",
    "§Ah, yes. Just let me know which town you'd like to \
     travel to and we'll be on our way shortly.",
    "§Very good. Just tell me which town you'd like to \
     travel to and we'll leave shortly.",
];

/// The increase per-station, not from/to stations.
const RATE_PER_TOWN: f32 = 1.26;

/// The price increase for each subsequent reuse.
const REUSE_PRICE_RATE: f32 = 1.05;

/// The minimum price of each pass purchased here.
const STARTING_PRICE: u32 = 600;

#[derive(EntityHolder, AreaTools)]
pub struct Station {
    area_title: String,
    area_num: usize,
    entities: RwLock<Vec<Box<Entity>>>,
    coordinates: (usize, usize, usize),
    connections: Mutex<Vec<(usize, usize, usize)>>,
    distance_south: usize,
    distance_north: usize,
}

impl Station {
    /// The standard constructor format required by `area_settings`.
    pub fn new(_class: Class, area_num: usize, coordinates: (usize, usize, usize)) -> Box<Area> {
        let town_num = coordinates.0;
        // The distance for towns #1-3 is fixed.
        let (distance_south, distance_north) = match town_num {
            1 => (0, 9), // Town #1 - 10
            2 => (1, 5), // Town #1 - 7
            3 => (2, 2), // Town #1 - 5
            _ => {
                // Random; farther south than north
                let variance = (town_num / 3) + 1; // Variance increases every 3 towns.
                let max_distance = thread_rng().gen_range(town_num - variance, town_num + variance);
                ((max_distance as f32 * 0.6) as usize + 1,
                 (max_distance as f32 * 0.5) as usize + 1)
            }
        };

        Box::new(Station {
            area_title: String::from("Travel Station"),
            area_num,
            coordinates,
            entities: RwLock::new(Vec::new()),
            connections: Mutex::new(Vec::new()),
            distance_south,
            distance_north,
        })
    }
}

impl Area for Station {
    fn get_type(&self) -> &'static str {
        "station"
    }

    fn get_map_icon(&self) -> &'static str {
        " T "
    }

    fn get_entrance_message(&self) -> Option<String> {
        let replacements = vec![
            ("<station>", self.get_town_num().to_string()),
            ("<south>", self.distance_south.to_string()),
            ("<north>", self.distance_north.to_string()),
        ];

        let choose = choose(&ENTRANCE_TEXT);
        let ret = text::apply_replacements(choose, &replacements);

        Some(ret)
    }

    fn get_title(&self) -> String {
        self.area_title.clone()
    }

    fn get_specials(&self, player: &PlayerMeta, responses: &mut Vec<Response>) {
        let town_num = self.get_town_num();
        let south_dist = self.distance_south;
        let north_dist = self.distance_north;

        responses.push(travel_pass_info(
            "§Ask for more information about travel passes.",
        ));
        responses.push(pass_purchase_info(
            town_num,
            "Ask about buying travel passes.",
        ));
        responses.push(use_pass(
            player.get_player_id(),
            town_num,
            south_dist,
            north_dist,
            "Use one of your passes.",
        ));
        responses.push(purchase_booklet(
            player.get_player_id(),
            town_num,
            "Buy a new travel booklet.",
        ));
        responses.push(purchase_pass(
            player.get_player_id(),
            town_num,
            south_dist,
            north_dist,
            "Add a pass to your booklet.",
        ));
    }
}

/// The rate of traveling to another town from here.
pub fn get_travel_rate(town_num: usize) -> f32 {
    (STARTING_PRICE as f32 / town_num as f32) + (RATE_PER_TOWN * town_num as f32)
}

/// The specific price for traveling to another town
/// from the specified station.
pub fn get_travel_price(town_num: usize, travel_to: usize) -> u32 {
    let rate = get_travel_rate(town_num);
    let distance = (travel_to as isize - town_num as isize).abs();
    (rate * distance as f32) as u32
}

/// The price of purchasing a pass with the specified
/// number of uses.
pub fn get_ticket_price(travel_price: u32, num_uses: u32) -> u32 {
    travel_price + (num_uses as f32 * REUSE_PRICE_RATE) as u32
}

/// The price of purchasing an empty `Passbook` from
/// this station.
pub fn get_booklet_price(town_num: usize) -> u32 {
    (town_num as f32 * RATE_PER_TOWN) as u32 + 10
}

/// Displays information to the user about buying what
/// travel passes do.
pub fn travel_pass_info(text: &str) -> Response {
    Response::action_only(text, |player| {
        let info = choose(&TRAVEL_PASS_INFO_TEXT);
        player.send_blocking_message(info);
    })
}

/// Displays information to the user about purchasing
/// a pass from this specific station.
pub fn pass_purchase_info(town_num: usize, text: &'static str) -> Response {
    Response::action_only(text, move |player| {
        let info = choose(&PASS_PURCHASE_INFO_TEXT);
        let replacements = vec![
            ("<rate>", (get_travel_rate(town_num) as u64).to_string()),
            ("<booklet>", get_booklet_price(town_num).to_string()),
        ];
        let info = text::apply_replacements(info, &replacements);

        player.send_blocking_message(&info);
    })
}

/// Takes the player to `_use_pass()`, a dialogue used
/// for travelling to other towns.
pub fn use_pass(player_id: usize, town_num: usize, south_dist: usize, north_dist: usize, text: &'static str) -> Response {
    Response::goto_dialogue(text, move |_| {
        _use_pass(player_id, town_num, south_dist, north_dist)
    })
}

/// The actual dialogue used for travelling to other
/// towns.
pub fn _use_pass(player_id: usize, town_num: usize, south_dist: usize, north_dist: usize) -> Dialogue {
    let south_bound = town_num - south_dist;
    let north_bound = town_num + north_dist;

    let responses = vec![
        Response::text_only("Walk away.")
    ];
    let commands = vec![
        use_pass_command(north_bound, south_bound)
    ];

    Dialogue {
        title: String::from("Use a Pass"),
        text: text::choose_text(&PASS_USE_TEXT),
        responses,
        commands,
        player_id,
        ..Dialogue::default()
    }
}

/// The command used by `_use_pass()`, which handles
/// the users input to determine where to them, and
/// subsequently takes them there.
fn use_pass_command(north_bound: usize, south_bound: usize) -> Command {
    Command::action_only(
        "goto #", "Go to town #.",
        move |args, player| {
            parse_use_pass_arguments(args, player, north_bound, south_bound)
                .ok()
                .and_then(|new_coords| Some(handle_use_pass(player, new_coords)));
        },
    )
}

/// Handles parsing the arguments sent to
/// `use_pass_command()`. Informs the player of
/// anything that goes wrong.
fn parse_use_pass_arguments(args: &Vec<&str>, player: &PlayerMeta, north_bound: usize, south_bound: usize) -> Result<(usize, usize, usize), ()> {
    // Ensure that there are enough arguments.
    if args.len() < 1 {
        player.send_short_message("Excuse me?");
        return Err(());
    }
    // Ensure that the town number can be parsed correctly.
    let town_num: usize = match args[0].parse() {
        Ok(num) => num,
        Err(_) => {
            player.send_short_message("§I'm not sure exactly where you're trying to go.");
            return Err(());
        }
    };
    // Ensure that the town number is within this station's bounds.
    if town_num > north_bound || town_num < south_bound {
        player.send_short_message(
            "§Sorry, but we can't quite take you home from here. \
             You'll need to make a connection to get that far."
        );
        return Err(());
    }
    // Ensure that the player has a valid pass.
    if !player_has_pass(player, town_num) {
        player.send_short_message(
            "§Looks like you don't actually have a pass \
             for this area. Maybe buy one or try again."
        );
        return Err(());
    }
    // Assume that the player's current dialogue is an
    // area dialogue and delete it, if so.
    if let Err(_) = try_delete_options(player.get_player_id()) {
        player.send_short_message(
            "§You should finish your current \
             dialogues before moving on."
        );
        return Err(());
    }

    Ok(access::town(town_num).locate_area("station")
        .expect("This town's station did not generate correctly."))
}

/// Determines whether the entity associated with `player`
/// has a pass to the input `town_num`. Does not yet
/// check outside of the main inventory.
fn player_has_pass(player: &PlayerMeta, town_num: usize) -> bool {
    player.entity(|e|{
        e.get_inventory()
            .expect("Player no longer has an inventory.")
            .for_each_item(|item| test_use_pass(item, town_num))
            .is_some()
    })
}

/// Responsible for transferring the player to its new
/// area and displaying the "animation" to the screen.
fn handle_use_pass(player: &PlayerMeta, new_coords: (usize, usize, usize)) {
    access::area(player.get_coordinates(), |current_area| {
        access::area(new_coords, |new_area| {
            current_area.transfer_to_area(player.get_player_id(), new_area);
            let next = new_area.get_dialogue(player);
            register_options(next);
            player.update_options();
            player.send_blocking_message("∫0.3.∫0.3 .∫0.3 .∫0.3 .∫0.3 .");
        })
    });
}

/// Takes the player to `_purchase_booklet()`, a
/// dialogue used for the player to purchase a new
/// travel booklet.
pub fn purchase_booklet(player_id: usize, town_num: usize, text: &'static str) -> Response {
    Response::goto_dialogue(text, move |_| _purchase_booklet(player_id, town_num))
}

/// The actual dialogue used for purchasing a new
/// travel booklet.
pub fn _purchase_booklet(player_id: usize, town_num: usize) -> Dialogue {
    let price = get_booklet_price(town_num);
    let title = String::from("Confirm Purchase");
    let text = format!("Sure thing! That'll be {}g.", price);
    let responses = vec![
        purchase_booklet_walk_away(),
        purchase_booklet_response(price)
    ];
    Dialogue::simple(title, text, responses, player_id)
}

/// A simple response used by `_purchase_booklet()`
/// which returns the player to the main dialogue,
/// displaying a short message.
fn purchase_booklet_walk_away() -> Response {
    Response::simple("Walk away.", |player| {
        player.add_short_message("§No harm done. Just let me know if you need anything else.");
    })
}

/// The response used by `_purchase_booklet()`
/// responsible for taking
fn purchase_booklet_response(price: u32) -> Response {
    Response::simple("Purchase item.", move |player| {
        player.entity(|entity| {
            let inventory = entity
                .get_inventory()
                .expect("Player no longer has an inventory.");

            let booklet = PassBook::new();

            if inventory.can_add_item(&booklet) {
                inventory.add_item(Box::new(booklet), Some(entity));
                entity.take_money(price);
                player.add_short_message("Thanks for your purchase!");
            } else {
                player.add_short_message(
                    "§Looks like you don't have enough space \
                     for that. Make some and come back later."
                );
            }
        });
    })
}

/// Registers an additional dialogue for the player
/// which lets them confirm whether they would like
/// to purchase the new booklet.
pub fn confirm_purchase_booklet(player: &PlayerMeta, price: u32) {
    let on_yes = move |player: &PlayerMeta| {
        player.entity(|entity| {
            let inventory = entity
                .get_inventory()
                .expect("Player no longer has an inventory.");

            let booklet = PassBook::new();

            if inventory.can_add_item(&booklet) {
                inventory.add_item(Box::new(booklet), Some(entity));
                entity.take_money(price);

                player.add_short_message("Thanks for your purchase!");
            } else {
                player.add_short_message(
                    "§Looks like you don't have enough space \
                     for that. Make some and come back later."
                );
            }
        });
    };
    let on_no = |player: &PlayerMeta| {
        player.add_short_message(
            "No harm done. Just let me know if you\n\
             need anything else."
        );
    };

    let dialogue = Dialogue::confirm_action(player.get_player_id(), true, on_yes, on_no);
    register_options(dialogue);
    player.update_options();
}

/// A response which directs the player to `purchase_pass()`.
pub fn purchase_pass(player_id: usize, town_num: usize, south_dist: usize, north_dist: usize, text: &'static str) -> Response {
    Response::goto_dialogue(text, move |_| {
        _purchase_pass(player_id, town_num, south_dist, north_dist)
    })
}

/// The actual dialogue used by `purchase_pass()`, responsible
/// for letting the player add a new pass to its travel booklet.
pub fn _purchase_pass(player_id: usize, town_num: usize, south_dist: usize, north_dist: usize)-> Dialogue {
    let south_bound = town_num - south_dist;
    let north_bound = town_num + north_dist;
    let rate = get_travel_rate(town_num);

    let responses = vec![
        Response::text_only("Walk away.")
    ];
    let commands = vec![
        purchase_pass_command(town_num, north_bound, south_bound)
    ];
    let replacements = vec![
        ("<south>", south_bound.to_string()),
        ("<north>", north_bound.to_string()),
        ("<rate>", (rate as u32).to_string())
    ];

    Dialogue {
        title: String::from("Buy a Pass"),
        text: Some(text::generate_text(&PASS_PURCHASE_TEXT, &replacements)),
        responses,
        commands,
        player_id,
        ..Dialogue::default()
    }
}

/// A command used by `_purchase_pass()` which lets the player
/// specify which town they would like to purchase a pass to.
fn purchase_pass_command(town_num: usize, north_bound: usize, south_bound: usize) -> Command {
    Command {
        input: String::from("buy #x #y"),
        output_desc: String::from("Buy a pass for town #x with #y uses."),
        run: Box::new(move |args: &Vec<&str>, player: &PlayerMeta| {
            parse_purchase_pass_arguments(args, player, north_bound, south_bound)
                .ok()
                .and_then(|(travel_to, num_uses)| {
                    Some(handle_purchase_pass(player, town_num, travel_to, num_uses))
                });
        }),
        next_dialogue: Ignore,
    }
}

/// Parses the arguments sent to `purchase_pass_command()`
/// and informs the user if anything goes wrong.
fn parse_purchase_pass_arguments(args: &Vec<&str>, player: &PlayerMeta, north_bound: usize, south_bound: usize) -> Result<(usize, u32), ()> {
    // Make sure enough arguments were specified.
    if args.len() < 1 {
        player.send_short_message("Excuse me?");
        return Err(());
    }
    // Make sure the arguments can be parsed correctly.
    let travel_to: usize = match args[0].parse() {
        Ok(num) => num,
        Err(_) => {
            player.send_short_message("You may need to speak up, there.");
            return Err(());
        }
    };
    // Make sure the station is willing to travel this far.
    if travel_to > north_bound || travel_to < south_bound {
        player.send_short_message(
            "§Sorry, but we can't quite take you home from here. \
                     You'll need to make a connection to get that far."
        );
        return Err(());
    }
    // Determine the number of uses to purchase the pass with.
    let num_uses: u32 = if args.len() > 1 {
        if let Ok(num) = args[1].parse() {
            num
        } else { // Invalid argument at position 3
            player.send_short_message("§I'm not really sure how many uses you're looking for.");
            return Err(());
        }
    } else {
        1
    };
    return Ok((travel_to, num_uses))
}

/// The actual process responsible for handling the transaction
/// of purchasing a new travel pass.
fn handle_purchase_pass(player: &PlayerMeta, town_num: usize, travel_to: usize, num_uses: u32) {
    player.entity(|entity| {
        // Calculate a price for this pass.
        let travel_price = get_travel_price(town_num, travel_to);
        let full_price = get_ticket_price(travel_price, num_uses);

        // Ensure that the player has enough money.
        if !entity.can_afford(full_price) {
            player.send_short_message("Sorry, there, but you can't afford that.");
            return;
        }
        // Verify that the player has a passbook with
        // enough space.
        let confirmation_sent = entity.as_player()
            .unwrap() // Entity is known to be a player
            .main_inventory
            .for_each_item(|item|
                test_confirm_purchase(item, player, full_price, travel_to, num_uses))
            .is_some();
        // They did not have a book with enough space.
        if !confirmation_sent {
            player.send_short_message(
                "§Looks like you don't have a place for that. \
                         You might want to buy a new travel book."
            );
            return;
        }
    });
}

/// Lets the user confirm whether they would like to like
/// to purchase the aforementioned pass.
fn confirm_purchase_pass(player: &PlayerMeta, price: u32, travel_to: usize, num_uses: u32) {
    let text = format!("Thanks! That's gonna be {}g.", price);

    let on_yes = move |player: &PlayerMeta| {
        player.entity(|entity| {
            let found = entity.get_inventory()
                .expect("Player no longer has an inventory.")
                .for_each_item(|item|
                    test_add_item(item, travel_to, num_uses))
                .is_some();

            if found {
                entity.take_money(price);
                player.add_short_message(
                    "§Thanks for doing business with us! \
                     You can use this whenever you like."
                );
            } else {
                player.add_short_message("§Huh... That's odd. Looks like you no longer have a book.");
            }
        });
    };
    let on_no = |player: &PlayerMeta| {
        player.add_short_message("That's too bad∫0.2.∫0.2.∫0.2.∫0.3 Let me know if you\nneed anything else.");
    };
    register_options(Dialogue::confirm_action(player.get_player_id(), true, on_yes, on_no));
    player.update_options();
    player.send_blocking_message(&text);
}

/// Verifies that the item is a passbook and, if so,
/// verifies that it can be used before using it.
fn test_use_pass(passbook: &Item, town_num: usize) -> Option<bool> {
    if let Some(ref pass) = Any::downcast_ref::<PassBook>(passbook.as_any()) {
        if pass.has_pass(town_num) {
            pass.use_pass(town_num);
            return Some(true);
        }
    }
    None
}

/// Verifies that the item is a passbook and, if so,
/// adds a new pass to it.
fn test_add_item(passbook: &Item, travel_to: usize, num_uses: u32) -> Option<bool> {
    if let Some(ref pass) = Any::downcast_ref::<PassBook>(passbook.as_any()) {
        pass.add_pass(travel_to, num_uses);
        return Some(true);
    }
    None
}

/// Verifies that the item is a passbook and, if so,
/// sends the player a new confirmation dialogue after
/// ensuring that the booklet can hold more passes.
fn test_confirm_purchase(passbook: &Item, player: &PlayerMeta, full_price: u32, travel_to: usize, num_uses: u32) -> Option<bool> {
    if let Some(ref pass) = Any::downcast_ref::<PassBook>(passbook.as_any()) {
        if pass.can_hold_more() {
            confirm_purchase_pass(player, full_price, travel_to, num_uses);
            return Some(true);
        }
    }
    None
}