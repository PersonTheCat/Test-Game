use crate::player_data::PlayerMeta;
use crate::text;
use crate::traits::{Area, Entity};
use crate::types::classes::Class;
use crate::types::effects::{Effect, EffectType::*};
use crate::types::entities::players::Player;
use crate::util::access;
use crate::util::player_options::Response;

use parking_lot::RwLock;
use parking_lot::Mutex;
use rand::random;

#[derive(EntityHolder, AreaTools)]
pub struct Fountain {
    entrance_message: String,
    area_title: String,
    area_num: usize,
    entities: RwLock<Vec<Box<Entity>>>,
    coordinates: (usize, usize, usize),
    connections: Mutex<Vec<(usize, usize, usize)>>,
}

impl Fountain {
    pub fn new(_class: Class, area_num: usize, coordinates: (usize, usize, usize)) -> Box<Area> {
        Box::new(Fountain {
            entrance_message: String::from("Welcome to the test fountain."),
            area_title: String::from("Fountain"),
            area_num,
            coordinates,
            entities: RwLock::new(Vec::new()),
            connections: Mutex::new(Vec::new()),
        })
    }
}

impl Area for Fountain {
    fn get_type(&self) -> &'static str {
        "fountain"
    }

    fn get_map_icon(&self) -> &'static str {
        "[F]"
    }

    fn can_enter(&self, _player: &Player) -> bool {
        true
    }

    fn get_entrance_message(&self) -> Option<String> {
        Some(self.entrance_message.clone())
    }

    fn get_title(&self) -> String {
        self.area_title.clone()
    }

    fn get_specials(&self, player: &PlayerMeta, responses: &mut Vec<Response>) {
        let coords = self.get_coordinates();
        let successful_donations = player.get_record(coords, "successful_donations");

        if successful_donations == 0 {
            let num_donations = player.get_record(coords, "num_donations");
            let price = get_price(num_donations, self.get_town_num());
            let text = format!("Throw a coin into the fountain ({}g).", price);
            responses.push(donate_response(text, price, coords));
        } else {
            responses.push(Response::text_only(
                "§The gods have already spoken in your favor (do nothing).",
            ));
        }
    }
}

const BASE_PRICE: u32 = 10;
const LEVEL_RATE: f32 = 7.5;

fn get_price(num_donations: u8, level_num: usize) -> u32 {
    (BASE_PRICE + (level_num as f32 * LEVEL_RATE) as u32) * (num_donations as u32 + 1)
}

fn donate_response(text: String, price: u32, coords: (usize, usize, usize)) -> Response {
    Response::_simple(text, move |player| {
        access::context(player, |town, _, entity| {
            if !entity.can_afford(price) {
                player.add_short_message("You can't afford this offering.");
                return;
            }
            player.incr_record(coords, "num_donations");
            entity.take_money(price);

            if random() {
                player.add_short_message(text::rand_donation_rejected());
                return;
            }

            player.incr_record(coords, "successful_donations");

            let effect = Effect::get_fountain_effect(town.town_num);
            println!("applying effect.");
            effect.apply(entity);

            if let Temporary(duration) = effect.effect_type {
                player.add_short_message(&format!(
                    "§The gods have blessed you with {} {} for {} seconds.",
                    effect.name,
                    effect.level,
                    duration / 1000
                ));
            } else {
                player.add_short_message(&format!(
                    "The gods have blessed you with\n{} {}.",
                    effect.name,
                    effect.level
                ));
            }
        });
    })
}