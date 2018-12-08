use crate::traits::{ Area, Entity };
use crate::text;

use std::cell::RefCell;

#[derive(EntityHolder, AreaTools)]
pub struct Path
{
    area_title: String,
    area_num: usize,
    coordinates: (usize, usize, usize),
    entities: RefCell<Vec<Box<Entity>>>,
    connections: RefCell<Vec<(usize, usize, usize)>>
}

impl Path
{
    pub fn new(area_num: usize, coordinates: (usize, usize, usize)) -> Box<Area>
    {
        Box::new(Path
        {
            area_title: text::rand_path_name().to_string(),
            area_num,
            coordinates,
            entities: RefCell::new(Vec::new()),
            connections: RefCell::new(Vec::new())
        })
    }
}

impl Area for Path
{
    fn get_type(&self) -> &'static str { "path" }

    fn get_map_icon(&self) -> &'static str { "[ ]" }

    fn get_entrance_message(&self) -> Option<String> { None }

    fn get_title(&self) -> String { self.area_title.clone() }
}