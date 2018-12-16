use crate::text;
use crate::traits::{Area, Entity};

use parking_lot::RwLock;
use parking_lot::Mutex;

#[derive(EntityHolder, AreaTools)]
pub struct Path {
    area_title: String,
    area_num: usize,
    coordinates: (usize, usize, usize),
    entities: RwLock<Vec<Box<Entity>>>,
    connections: Mutex<Vec<(usize, usize, usize)>>,
}

impl Path {
    pub fn new(area_num: usize, coordinates: (usize, usize, usize)) -> Box<Area> {
        Box::new(Path {
            area_title: text::rand_path_name().to_string(),
            area_num,
            coordinates,
            entities: RwLock::new(Vec::new()),
            connections: Mutex::new(Vec::new()),
        })
    }
}

impl Area for Path {
    fn get_type(&self) -> &'static str {
        "path"
    }

    fn get_map_icon(&self) -> &'static str {
        "[ ]"
    }

    fn get_entrance_message(&self) -> Option<String> {
        None
    }

    fn get_title(&self) -> String {
        self.area_title.clone()
    }
}
