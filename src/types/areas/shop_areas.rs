use crate::text;
use crate::traits::{Area, Entity};
use crate::types::classes::Class;
use crate::types::entities::npcs::{Shopkeeper, NPC};
use crate::*;

use lazy_static::lazy_static;
use parking_lot::Mutex;
use regex::Regex;

use rand::{thread_rng, Rng};

static PUB_LOCATIONS: [&'static str; 8] = [
    "standing by the wall",
    "sitting at the first table",
    "leaning on the bar",
    "sitting at the bar",
    "staring at the entrance",
    "sitting near the back",
    "talking to the shopkeeper",
    "near the center of the room",
];
static WALK_IN: [&'static str; 6] = [
    "As you walk into the bar, you notice",
    "You walk into the bar, look around, and see",
    "You quickly look through the room and notice",
    "As you look into the establishment, you see",
    "Making your way inside, you look around and see",
    "As you make your way inside, you notice",
];
static SEE_BARTENDER: [&'static str; 5] = [
    " as well as the bartender, <name>, standing nearby.",
    " and the bartender, <name>, behind the counter.",
    " and also <name>, the owner, standing nearby.",
    " and even <name>, the owner of the pub.",
    " as well as the bartender, <name>.",
];

#[derive(EntityHolder, AreaTools)]
pub struct Pub {
    owner_name: String,
    owner_title: String,
    area_title: String,
    area_num: usize,
    entities: Mutex<Vec<Box<Entity>>>,
    location_order: Vec<u8>,
    coordinates: (usize, usize, usize),
    connections: Mutex<Vec<(usize, usize, usize)>>,
}

impl Pub {
    pub fn new(class: Class, area_num: usize, coordinates: (usize, usize, usize)) -> Box<Area> {
        let mut entities: Vec<Box<Entity>> = Vec::new();
        entities.push(Box::new(Shopkeeper::new()));
        entities.push(Box::new(NPC::new(class, coordinates)));
        entities.push(Box::new(NPC::with_intro(
            String::from(
                "I've lived a terrible, boring life.∫\n\
                 I have nothing else to say∫0.2.∫0.2.∫0.2.∫0.4\n\
                 and nothing to sell.",
            ),
            class,
            coordinates,
        )));

        Box::new(Pub {
            owner_name: text::rand_npc_name(),
            owner_title: String::from("Shop Keeper"),
            area_title: String::from("Pub"),
            area_num,
            coordinates,
            entities: Mutex::new(entities),
            location_order: random_pub_location_order(2),
            connections: Mutex::new(Vec::new()),
        })
    }
}

impl Area for Pub {
    fn get_type(&self) -> &'static str {
        "shop"
    }

    fn get_map_icon(&self) -> &'static str {
        " S "
    }

    fn get_entrance_message(&self) -> Option<String> {
        let entities = self.entities.lock();
        let mut index = 0;
        let mut text = String::from("§"); // Start in auto-break mode.
        text += *choose(&WALK_IN);

        for entity in entities.iter().filter(|e| e.get_type() == "npc") {
            let loc_index = self.location_order[index] as usize;
            let location = PUB_LOCATIONS[loc_index];
            let description = entity
                .get_description()
                .expect("NPCs must have descriptions");
            let article = if starts_with_vowel(description) {
                "an"
            } else {
                "a"
            };

            text += &format!(" {} {} {},", article, description, location);
            index += 1;
        }
        let bartender = choose(&SEE_BARTENDER);
        let replacements = vec![("<name>", self.owner_name.clone())];
        text += &text::apply_replacements(bartender, &replacements);

        Some(text)
    }

    fn get_title(&self) -> String {
        self.area_title.clone()
    }
}

fn random_pub_location_order(size: usize) -> Vec<u8> {
    let mut vec: Vec<u8> = (0..PUB_LOCATIONS.len() as u8).collect();
    let slice: &mut [u8] = &mut vec;
    thread_rng().shuffle(slice);
    slice[0..size].to_vec()
}

// To-do: Regex
fn starts_with_vowel(text: &str) -> bool {
    lazy_static! {
        static ref vowel_pattern: Regex = Regex::new(r"^[aeiou]").unwrap();
    }
    vowel_pattern.is_match(text)
}
