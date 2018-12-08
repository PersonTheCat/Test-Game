use crate::util::player_options::{ Response, Dialogue, Command};
use crate::types::items::pass_books::PassBook;
use crate::traits::{ Area, Entity };
use crate::player_data::PlayerMeta;
use crate::types::classes::Class;
use crate::util::access;
use crate::text;
use crate::*;

use std::cell::RefCell;

use rand::{ Rng, thread_rng };

static ENTRANCE_TEXT: [&str; 5] =
[
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
    conductor, if you need anything else."
];

static TRAVEL_PASS_INFO_TEXT: [&str; 3] =
[
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
    purchase."
];

static PASS_PURCHASE_INFO_TEXT: [&str; 3] =
[
    "§We're currently selling passes at about <rate>g per km.∫0.5 \
    If you need a booklet to hold more, you can buy one \
    for about <booklet>g.",

    "§Our travel passes are currently going for about <rate>g \
    per km.∫0.5 if your booklet is running low on space or \
    if you need to purchase a new one, you can buy one from us \
    for about <booklet>g.",

    "§We sell travel passes for roughly <rate>g per km.∫0.5 \
    If needed, you can also buy a booklet for about <booklet>g."
];

static PASS_PURCHASE_TEXT: [&str; 3] =
[
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
    say so, if that's what you need."
];

static PASS_USE_TEXT: [&str; 3] =
[
    "§Very well. Just let me know the number of the town \
    you'd like to travel to and we'll set off.",

    "§Ah, yes. Just let me know which town you'd like to \
    travel to and we'll be on our way shortly.",

    "§Very good. Just tell me which town you'd like to \
    travel to and we'll leave shortly."
];

//The increase per-station, not from/to stations.
const RATE_PER_TOWN: f32 = 1.26;
const REUSE_PRICE_RATE: f32 = 1.05;
const STARTING_PRICE: u32 = 600;

#[derive(EntityHolder, AreaTools)]
pub struct Station
{
    pub area_title: String,
    pub area_num: usize,
    entities: RefCell<Vec<Box<Entity>>>,
    coordinates: (usize, usize, usize),
    connections: RefCell<Vec<(usize, usize, usize)>>,
    distance_south: usize,
    distance_north: usize
}

impl Station
{
    pub fn new(_class: Class, area_num: usize, coordinates: (usize, usize, usize)) -> Box<Area>
    {
        let town_num = coordinates.0;

        let (distance_south, distance_north) = match town_num
        {
            1 => (0, 9), // Town #1 - 10
            2 => (1, 5), // Town #1 - 7
            3 => (2, 2), // Town #1 - 5
            _ =>         // Random; farther south than north
            {
                let variance = (town_num / 3) + 1; // Variance increases every 3 towns.
                let max_distance = thread_rng().gen_range(town_num - variance, town_num + variance);

                ((max_distance as f32 * 0.6) as usize + 1,
                 (max_distance as f32 * 0.5) as usize + 1)
            }
        };

        Box::new(Station
        {
            area_title: String::from("Travel Station"),
            area_num,
            coordinates,
            entities: RefCell::new(Vec::new()),
            connections: RefCell::new(Vec::new()),
            distance_south,
            distance_north
        })
    }
}

impl Area for Station
{
    fn get_type(&self) -> &'static str { "station" }

    fn get_map_icon(&self) -> &'static str { " T " }

    fn get_entrance_message(&self) -> Option<String>
    {
        let replacements = vec!
        [
            ("<station>", self.get_town_num().to_string()),
            ("<south>", self.distance_south.to_string()),
            ("<north>", self.distance_north.to_string())
        ];

        let choose = choose(&ENTRANCE_TEXT);
        let ret = text::apply_replacements(choose, &replacements);

        Some(ret)
    }

    fn get_title(&self) -> String { self.area_title.clone() }

    fn get_specials(&self, player: &mut PlayerMeta, responses: &mut Vec<Response>)
    {
        let town_num = self.get_town_num();
        let south_dist = self.distance_south;
        let north_dist = self.distance_north;

        responses.push(travel_pass_info(
            "§Ask for more information about travel passes."
        ));
        responses.push(pass_purchase_info(town_num,
            "Ask about buying travel passes."
        ));
        responses.push(use_pass(player.player_id, town_num, south_dist, north_dist,
            "Use one of your passes."
        ));
        responses.push(purchase_booklet(player.player_id, town_num,
            "Buy a new travel booklet."
        ));
        responses.push(purchase_pass(player.player_id, town_num, south_dist, north_dist,
            "Add a pass to your booklet."
        ));
    }
}

//The rate of traveling to another town from here.
pub fn get_travel_rate(town_num: usize) -> f32
{
    (STARTING_PRICE as f32 / town_num as f32) + (RATE_PER_TOWN * town_num as f32)
}

pub fn get_travel_price(town_num: usize, travel_to: usize) -> u32
{
    let rate = get_travel_rate(town_num);
    let distance = (travel_to as isize - town_num as isize).abs();

    (rate * distance as f32) as u32
}

pub fn get_ticket_price(travel_price: u32, num_uses: u32) -> u32
{
    travel_price + (num_uses as f32 * REUSE_PRICE_RATE) as u32
}

pub fn get_booklet_price(town_num: usize) -> u32
{
    (town_num as f32 * RATE_PER_TOWN) as u32 + 10
}

pub fn travel_pass_info(text: &'static str) -> Response
{
    Response::action_only(text, | player |
    {
        let info = choose(&TRAVEL_PASS_INFO_TEXT);
        send_blocking_message(player, info, TEXT_SPEED);
    })
}

pub fn pass_purchase_info(town_num: usize, text: &'static str) -> Response
{
    Response::action_only(text, move | player |
    {
        let info = choose(&PASS_PURCHASE_INFO_TEXT);

        let replacements = vec!
        [
            ("<rate>", (get_travel_rate(town_num) as u64).to_string()),
            ("<booklet>", get_booklet_price(town_num).to_string())
        ];

        let info = text::apply_replacements(info, &replacements);

        send_blocking_message(player, &info, TEXT_SPEED);
    })
}

pub fn use_pass(player_id: usize, town_num: usize, south_dist: usize, north_dist: usize, text: &'static str) -> Response
{
    Response::goto_dialogue(text, move ||
    {
        _use_pass(player_id, town_num, south_dist, north_dist)
    })
}

pub fn _use_pass(player_id: usize, town_num: usize, south_dist: usize, north_dist: usize) -> Dialogue
{
    let mut responses = Vec::new();
    let mut commands = Vec::new();

    let south_bound = town_num - south_dist;
    let north_bound = town_num + north_dist;

    responses.push(Response::text_only("Walk away."));

    commands.push(Command::manual_desc_no_next("goto", "goto #", "Go to town #.",
    move | args, player |
    {
        if args.len() < 1
        {
            send_short_message(player, "Excuse me?");
            return;
        }
        let town_num: usize = match args[0].parse()
        {
            Ok(num) => num,
            Err(_) =>
            {
                send_short_message(player, "I'm not sure exactly where you're trying to go.");
                return;
            }
        };
        if town_num > north_bound || town_num < south_bound
        {
            send_short_message(player,
                "Sorry, but we can't quite take you home from here.\n\
                You'll need to make a connection to get that far."
            );
            return;
        }

        access::player_meta(player, |meta |
        {
            if !player_has_pass(meta, town_num)
            {
                send_short_message(player,
                    "Looks like you don't actually have a pass\n\
                    for this area. Maybe buy one or try again."
                );
            }

            let new_coords = access::town(town_num, |town |
            {
                town.locate_area("station")
                    .expect("Tried to travel to a town without a station.")
            });

            access::area(meta.coordinates, |current_area |
            {
                access::area(new_coords, |new_area |
                {
                    if let Err(_) = try_delete_options(player)
                    {
                        send_short_message(player,
                            "You should finish your current\n\
                            dialogues before moving on."
                        );
                        return;
                    }

                    current_area.transfer_to_area(player, new_area);
                    let next = new_area.get_dialogue(player);
                    register_options(next);
                    update_options_manually(player);

                    send_blocking_message(player, "∫0.3.∫0.3 .∫0.3 .∫0.3 .∫0.3 .",TEXT_SPEED);
                })
            })
            .expect("Player's current area could not be found.");
        });
    }));

    Dialogue::new
    (
        String::from("Use a Pass"),
        &PASS_USE_TEXT,
        Vec::new(),
        None,
        responses,
        commands,
        None,
        player_id
    )
}

fn player_has_pass(player: &PlayerMeta, town_num: usize) -> bool
{
    access::entity(player.get_accessor(), |entity |
    {
        let inventory = entity.get_inventory()
            .expect("Player no longer has an inventory.");

        let ret =
        inventory.for_each_item(| item |
        {
            if let Some(ref pass) = Any::downcast_ref::<PassBook>(item.as_any())
            {
                if pass.has_pass(town_num)
                {
                    pass.use_pass(town_num);
                    return Some(true); // from for-each
                }
            }
            None
        });
        match ret
        {
            Some(_) => true,
            None => false
        }
    })
    .expect("Unable to locate player entity.")
}

pub fn purchase_booklet(player_id: usize, town_num: usize, text: &'static str) -> Response
{
    Response::goto_dialogue(text, move ||
    {
        _purchase_booklet(player_id, town_num)
    })
}

pub fn _purchase_booklet(player_id: usize, town_num: usize) -> Dialogue
{
    let price = get_booklet_price(town_num);
    let text = format!("Sure thing! That'll be {}g.", price);

    let mut responses = Vec::new();

    responses.push(Response::simple("Walk away.",
    | player: usize |
    {
        add_short_message(player,
            "No harm done. Just let me know if you\n\
            need anything else."
        );
    }));
    responses.push(Response::simple("Purchase item.",
    move | player: usize |
    {
        access::player_context(player, |_, _, _, entity |
        {
            let inventory = entity.get_inventory()
                .expect("Player no longer has an inventory.");

            let booklet = PassBook::new();

            if inventory.can_add_item(&booklet)
            {
                inventory.add_item(Box::new(booklet), Some(entity));
                entity.take_money(price);

                add_short_message(player, "Thanks for your purchase!");
            }
            else
            {
                add_short_message(player,
                    "Looks like you don't have enough space\n\
                    for that. Make some and come back later."
                );
            }
        });
    }));

    Dialogue::simple_2
    (
        String::from("Confirm Purchase"),
        vec![text],
        Vec::new(),
        responses,
        player_id
    )
}

pub fn confirm_purchase_booklet(player_id: usize, price: u32)
{
    let on_yes = move | player: usize |
    {
        access::player_context(player, |_, _, _, entity |
        {
            let inventory = entity.get_inventory()
                .expect("Player no longer has an inventory.");

            let booklet = PassBook::new();

            if inventory.can_add_item(&booklet)
            {
                inventory.add_item(Box::new(booklet), Some(entity));
                entity.take_money(price);

                add_short_message(player, "Thanks for your purchase!");
            }
            else
            {
                add_short_message(player,
                    "Looks like you don't have enough space\n\
                    for that. Make some and come back later."
                );
            }
        });
    };
    let on_no = | player: usize |
    {
        add_short_message(player,
            "No harm done. Just let me know if you\n\
            need anything else."
        );
    };
    register_options(Dialogue::confirm_action(player_id, true, on_yes, on_no));
    update_options_manually(player_id);
}

pub fn purchase_pass(player_id: usize, town_num: usize, south_dist: usize, north_dist: usize, text: &'static str) -> Response
{
    Response::goto_dialogue(text, move ||
    {
        _purchase_pass(player_id, town_num, south_dist, north_dist)
    })
}

pub fn _purchase_pass(player_id: usize, town_num: usize, south_dist: usize, north_dist: usize) -> Dialogue
{
    let south_bound = town_num - south_dist;
    let north_bound = town_num + north_dist;
    let rate = get_travel_rate(town_num);

    let mut responses = Vec::new();
    let mut commands = Vec::new();
    let mut replacements = Vec::new();

    replacements.push(("<south>", south_bound.to_string()));
    replacements.push(("<north>", north_bound.to_string()));
    replacements.push(("<rate>", (rate as u32).to_string()));

    responses.push(Response::text_only("Walk away."));

    commands.push(Command
    {
        name: String::from("buy"),
        input_desc: String::from("buy #x #y"),
        output_desc: String::from("Buy a pass for town #x with #y uses."),
        run: Box::new(move | args: &Vec<&str>, player: usize |
        {
            if args.len() < 1 { send_short_message(player, "Excuse me?"); return; }

            let travel_to: usize = match args[0].parse()
            {
                Ok(num) => num,
                Err(_) =>
                {
                    send_short_message(player, "You may need to speak up, there.");
                    return;
                }
            };

            if travel_to > north_bound || travel_to < south_bound
            {
                send_short_message(player,
                    "Sorry, but we can't quite take you home from here.\n\
                    You'll need to make a connection to get that far."
                );
                return;
            }

            let mut num_uses = 1;

            if args.len() > 1
            {
                if let Ok(num) = args[1].parse::<u32>()
                {
                    num_uses = num;
                }
                else
                {
                    send_short_message(player,
                        "I'm not really sure how many uses\n\
                        you're looking for."
                    );
                    return;
                }
            }

            access::player_context(player, |_, _, _, entity |
            {
                let travel_price = get_travel_price(town_num, travel_to);
                let full_price = get_ticket_price(travel_price, num_uses);

                if !entity.can_afford(full_price)
                {
                    send_short_message(player, "Sorry, there, but you can't afford that.");
                    return;
                }

                let as_player = match entity.as_player()
                {
                    Some(p) => p,
                    None =>
                    {
                        send_short_message(player, "I smell hax.");
                        return;
                    }
                };

                let inventory = &as_player.main_inventory;

                let found =
                inventory.for_each_item(| item |
                {
                    match Any::downcast_ref::<PassBook>(item.as_any())
                    {
                        Some(ref book) =>
                        {
                            if book.can_hold_more()
                            {
                                confirm_purchase_pass(player, full_price, travel_to, num_uses);
                                Some(true)
                            }
                            else { None }
                        },
                        None => None
                    }
                });

                if let None = found
                {
                    send_short_message(player,
                        "Looks like you don't have a place for that.\n\
                        You might want to buy a new travel book."
                    );
                    return;
                }
            });
        }),
        next_dialogue: Ignore
    });

    Dialogue::new
    (
        String::from("Buy a Pass"),
        &PASS_PURCHASE_TEXT,
        replacements,
        None,
        responses,
        commands,
        None,
        player_id
    )
}

fn confirm_purchase_pass(player_id: usize, price: u32, travel_to: usize, num_uses: u32)
{
    let text = format!("Thanks! That's gonna be {}g.", price);

    let on_yes = move | player: usize |
    {
        access::player_context(player, |_, _, _, entity |
        {
            let inventory = entity.get_inventory()
                .expect("Player no longer has an inventory.");

            let found =
            inventory.for_each_item(| item |
            {
                match Any::downcast_ref::<PassBook>(item.as_any())
                {
                    Some(ref book) =>
                    {
                        if book.can_hold_more()
                        {
                            book.add_pass(travel_to, num_uses);
                            Some(true)
                        }
                        else { None }
                    },
                    None => None
                }
            });

            if let Some(_) = found
            {
                entity.take_money(price);

                add_short_message(player,
                    "Thanks for doing business with us!\n\
                    You can use this whenever you like."
                );
            }
            else
            {
                add_short_message(player, "Huh... That's odd. Looks like you no longer have a book.");
            }
        });
    };
    let on_no = | player: usize |
    {
        add_short_message(player,
            "That's too bad∫0.2.∫0.2.∫0.2.∫0.3 Let me know if you\n\
            need anything else."
        );
    };
    register_options(Dialogue::confirm_action(player_id, true, on_yes, on_no));
    update_options_manually(player_id);
    send_blocking_message(player_id, &text, TEXT_SPEED);
}