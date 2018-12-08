use crate::types::items::shops::{ PersistentShop, BlacksmithShop };
use crate::types::items::consumables::Consumable;
use crate::util::player_options::{ Dialogue, Response };
use crate::player_data::PlayerMeta;
use crate::types::classes::Class;
use crate::traits::Entity;
use crate::traits::Shop;
use crate::text;

use rand::random;

use std::cell::Cell;

const NORMAL_DIALOGUE: u8 = 0;
const TRADES: u8 = 1;
const SPECIAL_TRADES: u8 = 2;

pub struct NPC
{
    id: usize,
    name: String,
    title: Option<String>,
    introduction_text: Option<String>,
    description: String,
    god: String,
    food_trades: Box<Shop>,
    special_trades: Box<Shop>,
    coordinates: Cell<(usize, usize, usize)>
}

impl NPC
{
    /**
     * Test constructor
     */
    pub fn new(class: Class, coordinates: (usize, usize, usize)) -> NPC
    {
        let info = text::rand_npc_details();

        NPC
        {
            id: random(),
            name: info.0.to_string(),
            title: None,
            introduction_text: None,
            description: info.1.to_string(),
            god: text::rand_god(class).to_string(),
            food_trades: Box::new(PersistentShop::new(vec![Box::new(Consumable::poisonous_potato())])),
            special_trades: Box::new(BlacksmithShop::new(coordinates.0)),
            coordinates: Cell::new(coordinates)
        }
    }

    pub fn with_intro(intro: String, class: Class, coordinates: (usize, usize, usize)) -> NPC
    {
        let info = text::rand_npc_details();

        NPC
        {
            id: random(),
            name: info.0.to_string(),
            title: None,
            introduction_text: Some(intro),
            description: info.1.to_string(),
            god: text::rand_god(class).to_string(),
            food_trades: Box::new(PersistentShop::new(Vec::new())),
            special_trades: Box::new(BlacksmithShop::new(coordinates.0)),
            coordinates: Cell::new(coordinates)
        }
    }

    /**
     * NORMAL_DIALOGUE
     */
    fn get_main_dialogue(&self, player: &mut PlayerMeta, use_intro_title: bool) -> Dialogue
    {
        let mut text = Vec::new();
        let mut responses = Vec::new();
        let accessor = self.get_accessor();
        let player_id = player.player_id;

        let title = if use_intro_title
        {
            if let Some(ref txt) = self.introduction_text
            {
                text.push(txt.clone());
            }
            format!("Hi, I'm {}.", &self.name)
        }
        else { self.name.clone() };

        responses.push(Response::goto_entity_dialogue
        (
            "View main Trades", TRADES, accessor, player_id
        ));

        if player.god == self.god
        {
            text.push(text::generic_same_god_message(&self.name, &self.god));

            responses.push(Response::goto_entity_dialogue
            (
                "View Special Trades", SPECIAL_TRADES, accessor, player_id
            ));
        }

        responses.push(Response::_text_only
        (
            format!("Walk away from {}, the {}.", self.name, self.description)
        ));

        Dialogue::simple_2(title, text, Vec::new(), responses, player_id)
    }

    /**
     * TRADES
     */
    fn get_normal_trades(&self, player: &mut PlayerMeta) -> Dialogue
    {
        self.food_trades.get_dialogue(player, true, 1.0)
    }

    /**
     * SPECIAL_TRADES
     */
    fn get_special_trades(&self, player: &mut PlayerMeta) -> Dialogue
    {
        self.special_trades.get_dialogue(player, false, 1.0)
    }
}

impl Entity for NPC
{
    fn get_id(&self) -> usize { self.id }

    fn get_name(&self) -> &String { &self.name }

    fn get_description(&self) -> Option<&String> { Some(&self.description) }

    fn set_health(&self, _health: u32) { }

    fn get_health(&self) -> u32 { 10 }

    fn get_response_text(&self, player: &mut PlayerMeta) -> Option<String>
    {
        let ret = if player.has_entity_knowledge(self.id)
        {
            match self.title
            {
                Some(ref t) => format!("Speak to {}: {}", self.get_name(), t),
                None => format!("Speak to {}.", self.get_name())
            }
        }
        else { format!("Speak to the {}. (debug: {})", self.get_description().unwrap().clone(), self.god) };

        Some(ret)
    }

    fn _get_dialogue(&self, player: &mut PlayerMeta) -> Option<Dialogue>
    {
        let marker = match player.get_entity_knowledge(self.id)
        {
            Some(ref knowledge) => knowledge.dialogue_marker as i16,
            None => -1
        };
        if marker == -1
        {
            player.add_entity_knowledge(self.id);
            Some(self.get_main_dialogue(player, true))
        }
        else { self._goto_dialogue(marker as u8, player) }
    }

    fn _goto_dialogue(&self, marker: u8, player: &mut PlayerMeta) -> Option<Dialogue>
    {
        match marker
        {
            NORMAL_DIALOGUE => Some(self.get_main_dialogue(player, false)),
            TRADES => Some(self.get_normal_trades(player)),
            SPECIAL_TRADES => Some(self.get_special_trades(player)),
            _ => panic!("Error: Somehow skipped to a nonexistent dialogue (#{}).", marker)
        }
    }

    fn kill_entity(&self) {}

    fn as_npc(&self) -> Option<&NPC> { Some(self) }

    fn set_coordinates(&self, coords: (usize, usize, usize)) { self.coordinates.set(coords); }

    fn get_coordinates(&self) -> (usize, usize, usize) { self.coordinates.get() }

    fn get_type(&self) -> &str { "npc" }
}

pub struct Shopkeeper
{
    id: usize,
    name: String,
    title: String,
    god: String,
    shop: Box<Shop>
}

impl Shopkeeper
{
    /**
     * Test constructor.
     *
     * In the future, there will only be one
     * Shopkeeper struct that accepts
     * different kinds of shops.
     */
    pub fn new() -> Shopkeeper
    {
        Shopkeeper
        {
            id: random(),
            name: String::from("Blacksmith Guy"),
            title: String::from("Ordinary Blacksmith"),
            god: text::rand_babylonian_god().to_string(),
            shop: Box::new(BlacksmithShop::new(0))
        }
    }
}

impl Entity for Shopkeeper
{
    fn get_id(&self) -> usize { self.id }

    fn get_name(&self) -> &String { &self.name }

    fn get_title(&self) -> Option<&String> { Some(&self.title) }

    fn set_health(&self, _health: u32) {}

    fn get_health(&self) -> u32 { 10 }

    fn kill_entity(&self) {}

    fn get_type(&self) -> &str { "keeper" }
}