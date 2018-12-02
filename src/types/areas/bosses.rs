use traits::{ Area, Entity };
use types::classes::Class;

use std::cell::RefCell;

#[derive(EntityHolder, AreaTools)]
pub struct BossRoom
{
    entrance_message: String,
    pub area_title: String,
    pub area_num: usize,
    entities: RefCell<Vec<Box<Entity>>>,
    coordinates: (usize, usize, usize),
    connections: RefCell<Vec<(usize, usize, usize)>>
}

impl BossRoom
{
    pub fn new(_class: Class, area_num: usize, coordinates: (usize, usize, usize)) -> Box<Area>
    {
        Box::new(BossRoom
        {
            entrance_message: String::from("You see a boss who does not exist. Maybe try\na newer version of the game."),
            area_title: String::from("Test Boss Area"),
            area_num,
            coordinates,
            entities: RefCell::new(Vec::new()),
            connections: RefCell::new(Vec::new())
        })
    }
}

impl Area for BossRoom
{
    fn get_type(&self) -> &'static str { "boss" }

    fn get_map_icon(&self) -> &'static str { "[B]" }

    fn get_entrance_message(&self) -> Option<String> { Some(self.entrance_message.clone()) }

    fn get_title(&self) -> String { self.area_title.clone() }
}