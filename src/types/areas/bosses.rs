use crate::traits::{Area, Entity};
use crate::types::classes::Class;

use parking_lot::Mutex;

#[derive(EntityHolder, AreaTools)]
pub struct BossRoom {
    entrance_message: String,
    area_title: String,
    area_num: usize,
    entities: Mutex<Vec<Box<Entity>>>,
    coordinates: (usize, usize, usize),
    connections: Mutex<Vec<(usize, usize, usize)>>,
}

impl BossRoom {
    pub fn new(_class: Class, area_num: usize, coordinates: (usize, usize, usize)) -> Box<Area> {
        Box::new(BossRoom {
            entrance_message: String::from(
                "You see a boss who does not exist. Maybe try\na newer version of the game.",
            ),
            area_title: String::from("Test Boss Area"),
            area_num,
            coordinates,
            entities: Mutex::new(Vec::new()),
            connections: Mutex::new(Vec::new()),
        })
    }
}

impl Area for BossRoom {
    fn get_type(&self) -> &'static str {
        "boss"
    }

    fn get_map_icon(&self) -> &'static str {
        "[B]"
    }

    fn get_entrance_message(&self) -> Option<String> {
        Some(self.entrance_message.clone())
    }

    fn get_title(&self) -> String {
        self.area_title.clone()
    }
}
