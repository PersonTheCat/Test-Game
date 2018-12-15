use crate::player_data::PlayerMeta;
use crate::text;
use crate::traits::Entity;
use crate::traits::Shop;
use crate::types::classes::Class;
use crate::types::items::consumables::Consumable;
use crate::types::items::shops::{BlacksmithShop, PersistentShop};
use crate::util::player_options::{Dialogue, Response};

use atomic::Ordering::*;
use atomic::Atomic;
use rand::random;

const NORMAL_DIALOGUE: u8 = 0;
const TRADES: u8 = 1;
const SPECIAL_TRADES: u8 = 2;

pub struct NPC {
    id: usize,
    name: String,
    title: Option<String>,
    introduction_text: Option<String>,
    description: String,
    god: &'static str,
    food_trades: Box<Shop>,
    special_trades: Box<Shop>,
    coordinates: Atomic<(usize, usize, usize)>,
}

impl NPC {
    /**
     * Test constructor
     */
    pub fn new(class: Class, coordinates: (usize, usize, usize)) -> NPC {
        let info = text::rand_npc_details();

        NPC {
            id: random(),
            name: info.0.to_string(),
            title: None,
            introduction_text: None,
            description: info.1.to_string(),
            god: text::rand_god(class),
            food_trades: Box::new(PersistentShop::new(vec![Box::new(
                Consumable::poisonous_potato(),
            )])),
            special_trades: Box::new(BlacksmithShop::new(coordinates.0)),
            coordinates: Atomic::new(coordinates),
        }
    }

    pub fn with_intro(intro: String, class: Class, coordinates: (usize, usize, usize)) -> NPC {
        let info = text::rand_npc_details();

        NPC {
            id: random(),
            name: info.0.to_string(),
            title: None,
            introduction_text: Some(intro),
            description: info.1.to_string(),
            god: text::rand_god(class),
            food_trades: Box::new(PersistentShop::new(Vec::new())),
            special_trades: Box::new(BlacksmithShop::new(coordinates.0)),
            coordinates: Atomic::new(coordinates),
        }
    }

    fn get_title(&self, use_intro_title: bool) -> String {
        if use_intro_title {
            format!("Hi, I'm {}.", &self.name)
        } else {
            self.name.clone()
        }
    }

    /// Normal Dialogue
    fn get_main_dialogue(&self, player: &PlayerMeta, use_intro_title: bool) -> Dialogue {
        let mut text = None;
        let mut responses = Vec::new();
        let title = self.get_title(use_intro_title);

        responses.push(self.normal_trades_response());

        if self.god == &player.get_god(){
            text = Some(text::generic_same_god_message(&self.god));
            responses.push(self.special_trades_response());
        } else if use_intro_title {
            if let Some(ref txt) = self.introduction_text {
                text = Some(txt.clone());
            }
        }
        responses.push(self.walk_away_response());

        Dialogue {
            title,
            text,
            responses,
            player_id: player.get_player_id(),
            ..Dialogue::default()
        }
    }

    fn normal_trades_response(&self) -> Response {
        Response::goto_entity_dialogue("View main trades", TRADES, self.get_accessor())
    }

    fn special_trades_response(&self) -> Response {
        Response::goto_entity_dialogue("View special trades", SPECIAL_TRADES, self.get_accessor())
    }

    fn walk_away_response(&self) -> Response {
        Response::_text_only(format!("Walk away from {}, the {}.", self.name, self.description))
    }


    /// Normal Trades
    fn get_normal_trades(&self, player: &PlayerMeta) -> Dialogue {
        self.food_trades.get_dialogue(player, true, 1.0)
    }

    /// Special Trades
    fn get_special_trades(&self, player: &PlayerMeta) -> Dialogue {
        self.special_trades.get_dialogue(player, false, 1.0)
    }
}

impl Entity for NPC {
    fn get_id(&self) -> usize {
        self.id
    }

    fn get_name(&self) -> &String {
        &self.name
    }

    fn get_description(&self) -> Option<&String> {
        Some(&self.description)
    }

    fn set_health(&self, _health: u32) {}

    fn get_health(&self) -> u32 {
        10
    }

    fn get_response_text(&self, player: &PlayerMeta) -> Option<String> {
        let ret = if player.has_entity_knowledge(self.id) {
            match self.title {
                Some(ref t) => format!("Speak to {}: {}", self.get_name(), t),
                None => format!("Speak to {}.", self.get_name()),
            }
        } else {
            format!(
                "Speak to the {}. (debug: {})",
                self.get_description().unwrap().clone(),
                self.god
            )
        };

        Some(ret)
    }

    fn get_dialogue(&self, player: &PlayerMeta) -> Option<Dialogue> {
        let marker = match player.get_dialogue_marker(self.id) {
            Some(num) => num as i16,
            None => -1,
        };
        if marker == -1 {
            player.add_entity_knowledge(self.id);
            Some(self.get_main_dialogue(player, true))
        } else {
            self.goto_dialogue(marker as u8, player)
        }
    }

    fn goto_dialogue(&self, marker: u8, player: &PlayerMeta) -> Option<Dialogue> {
        match marker {
            NORMAL_DIALOGUE => Some(self.get_main_dialogue(player, false)),
            TRADES => Some(self.get_normal_trades(player)),
            SPECIAL_TRADES => Some(self.get_special_trades(player)),
            _ => panic!(
                "Error: Somehow skipped to a nonexistent dialogue (#{}).",
                marker
            ),
        }
    }

    fn kill_entity(&self) {}

    fn as_npc(&self) -> Option<&NPC> {
        Some(self)
    }

    fn set_coordinates(&self, coords: (usize, usize, usize)) {
        self.coordinates.store(coords, SeqCst);
    }

    fn get_coordinates(&self) -> (usize, usize, usize) {
        self.coordinates.load(SeqCst)
    }

    fn get_type(&self) -> &'static str {
        "npc"
    }
}

pub struct Shopkeeper {
    id: usize,
    name: String,
    title: String,
    god: &'static str,
    shop: Box<Shop>,
}

impl Shopkeeper {
    /// Test constructor.
    pub fn new() -> Shopkeeper {
        Shopkeeper {
            id: random(),
            name: String::from("Blacksmith Guy"),
            title: String::from("Ordinary Blacksmith"),
            god: text::rand_babylonian_god(),
            shop: Box::new(BlacksmithShop::new(0)),
        }
    }
}

impl Entity for Shopkeeper {
    fn get_id(&self) -> usize {
        self.id
    }

    fn get_name(&self) -> &String {
        &self.name
    }

    fn get_title(&self) -> Option<&String> {
        Some(&self.title)
    }

    fn set_health(&self, _health: u32) {}

    fn get_health(&self) -> u32 {
        10
    }

    fn kill_entity(&self) {}

    fn get_type(&self) -> &'static str {
        "keeper"
    }
}
