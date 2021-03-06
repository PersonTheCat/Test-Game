use crate::traits::{Area, Entity};
use crate::types::classes::Class;

use parking_lot::RwLock;
use parking_lot::Mutex;

#[derive(EntityHolder, AreaTools)]
pub struct Dungeon {
    entrance_message: String,
    area_title: String,
    area_num: usize,
    coordinates: (usize, usize, usize),
    entities: RwLock<Vec<Box<Entity>>>,
    connections: Mutex<Vec<(usize, usize, usize)>>,
}

impl Dungeon {
    pub fn new(_class: Class, area_num: usize, coordinates: (usize, usize, usize)) -> Box<Area> {
        Box::new(Dungeon {
            entrance_message: String::from(
                "You enter an empty dungeon and remember that this game is in alpha.",
            ),
            area_title: String::from("Test Dungeon"),
            area_num,
            coordinates,
            entities: RwLock::new(Vec::new()),
            connections: Mutex::new(Vec::new()),
        })
    }
}

impl Area for Dungeon {
    fn get_type(&self) -> &'static str {
        "dungeon"
    }

    fn get_map_icon(&self) -> &'static str {
        " D "
    }

    fn get_entrance_message(&self) -> Option<String> {
        Some(self.entrance_message.clone())
    }

    fn get_title(&self) -> String {
        self.area_title.clone()
    }
}
