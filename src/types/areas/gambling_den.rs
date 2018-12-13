use crate::player_data::PlayerMeta;
use crate::traits::{Area, Entity};
use crate::types::classes::Class;
use crate::util::access;
use crate::util::player_options::Response;
use crate::*;

use std::cell::RefCell;

use rand::random;

const MIN_AMOUNT_PER_TOWN: f32 = 22.15;
const WIN_CHANCE: f32 = 0.33;

static WIN_DIALOGUE: [&str; 1] = ["win dialogue"];

static LOSE_DIALOGUE: [&str; 1] = ["lose dialogue"];

static NOT_ENOUGH_MONEY: [&str; 1] = ["no money dialogue"];

#[derive(EntityHolder, AreaTools)]
pub struct GamblingDen {
    pub area_num: usize,
    entities: RefCell<Vec<Box<Entity>>>,
    coordinates: (usize, usize, usize),
    connections: RefCell<Vec<(usize, usize, usize)>>,
}

impl GamblingDen {
    pub fn new(_class: Class, area_num: usize, coordinates: (usize, usize, usize)) -> Box<Area> {
        Box::new(GamblingDen {
            area_num,
            coordinates,
            entities: RefCell::new(Vec::new()),
            connections: RefCell::new(Vec::new()),
        })
    }
}

impl Area for GamblingDen {
    fn get_type(&self) -> &'static str {
        "gambling"
    }

    fn get_map_icon(&self) -> &'static str {
        " M "
    }

    fn get_title(&self) -> String {
        String::from("Gambling Den")
    }

    fn get_specials(&self, _player: &PlayerMeta, responses: &mut Vec<Response>) {
        let min_price = (MIN_AMOUNT_PER_TOWN * self.get_town_num() as f32) as u32;

        responses.push(gamble(min_price, 2));
        responses.push(gamble(min_price * 2, 2));
        responses.push(gamble(min_price * 4, 3));
    }
}

fn gamble(amount: u32, multiple_out: u32) -> Response {
    let text = format!("Bet {}g.", amount);
    Response::_simple(text, move |player| {
        access::entity(player.get_accessor(), |entity| {
            if !entity.can_afford(amount) {
                let message = choose(&NOT_ENOUGH_MONEY);
                player.add_short_message(message);
                return;
            }

            entity.take_money(amount);

            let message = if random::<f32>() <= WIN_CHANCE {
                entity.give_money(amount * multiple_out);
                choose(&WIN_DIALOGUE)
            } else {
                choose(&LOSE_DIALOGUE)
            };

            player.add_short_message(message);
        });
    })
}
