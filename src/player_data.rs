use crate::messages::{ ReusableMessage, ChannelInfo };
use crate::types::classes::Class::{ self, * };
use crate::util::access::EntityAccessor;
use crate::GameMessage;
use crate::text;
use crate::*;

use hashbrown::HashMap;
use rand::random;

use std::cmp::Ordering::{ self, * };

pub static mut PLAYER_META: Option<Vec<PlayerMeta>> = None;

pub unsafe fn setup_player_registry()
{
    PLAYER_META = Some(Vec::new());
}

//Areas can store whatever information they want.
pub type AreaRecords = HashMap<(usize, usize, usize), HashMap<&'static str, u8>>;

pub struct PlayerMeta
{
    pub channel: ChannelInfo,
    pub player_id: usize,
    pub coordinates: (usize, usize, usize),
    pub area_records: AreaRecords,
    pub entity_knowledge: Vec<EntityKnowledge>,
    pub name: String,
    pub god: String,
    pub class: Class,
    pub reusable_message: ReusableMessage
}

impl PlayerMeta
{
    pub fn update_message(&mut self)
    {
        send_message_to_channel(&self.channel, &mut self.reusable_message, 0);
    }

    pub fn get_accessor(&self) -> EntityAccessor
    {
        EntityAccessor { coordinates: self.coordinates, entity_id: self.player_id, is_player: true }
    }

    pub fn player_has_visited(&self, area: (usize, usize, usize)) -> bool
    {
        self.area_records.contains_key(&area)
    }

    pub fn add_record_book(&mut self, area: (usize, usize, usize))
    {
        self.area_records.insert(area, HashMap::new());
    }

    pub fn add_entity_knowledge(&mut self, entity_id: usize)
    {
        match self.entity_knowledge.binary_search_by(| e |
        {
            e.entity_id.cmp(&entity_id)
        }){
            Ok(_index) => panic!("You dun goofed. I already know this boy."), // Test
            Err(index) => self.entity_knowledge.insert(index, EntityKnowledge::new(entity_id))
        };
    }

    pub fn has_entity_knowledge(&self, entity_id: usize) -> bool
    {
        match self.entity_knowledge.binary_search_by(| e |
        {
            e.entity_id.cmp(&entity_id)
        }){
            Ok(_index) => true,
            Err(_index) => false
        }
    }

    pub fn get_entity_knowledge(&self, entity_id: usize) -> Option<&EntityKnowledge>
    {
        match self.entity_knowledge.binary_search_by(| e |
        {
            e.entity_id.cmp(&entity_id)
        }){
            Ok(index) => Some(self.entity_knowledge.get(index).unwrap()),
            Err(_index) => None
        }
    }

    pub fn change_entity_knowledge(&mut self, entity_id: usize) -> Option<&mut EntityKnowledge>
    {
        match self.entity_knowledge.binary_search_by(| e |
        {
            e.entity_id.cmp(&entity_id)
        }){
            Ok(index) => Some(self.entity_knowledge.get_mut(index).unwrap()),
            Err(_index) => None
        }
    }

    pub fn set_record(&mut self, coords: (usize, usize, usize), record: &'static str, val: u8)
    {
        if let Some(ref mut records) = self.area_records.get_mut(&coords)
        {
            records.insert(record, val);
            return;
        }
        self.create_record(coords, record, val);
    }

    pub fn incr_record(&mut self, coords: (usize, usize, usize), record: &'static str)
    {
        if let Some(ref mut records) = self.area_records.get_mut(&coords)
        {
            if let Some(ref mut num) = records.get_mut(record)
            {
                **num += 1;
                return;
            }
            records.insert(record, 1);
            return;
        }
        self.create_record(coords, record, 1);
    }

    pub fn get_record(&mut self, coords: (usize, usize, usize), record: &'static str) -> u8
    {
        if let Some(ref mut records) = self.area_records.get_mut(&coords)
        {
            if let Some(num) = records.get(record)
            {
                return *num;
            }
            records.insert(record, 0);
            return 0;
        }
        0
    }

    pub fn create_record(&mut self, coords: (usize, usize, usize), record: &'static str, val: u8)
    {
        let mut new_records = HashMap::new();
        new_records.insert(record, val);
        self.area_records.insert(coords, new_records);
    }
}

pub fn new_player_event(message: &GameMessage)
{
    let player_id = random();

    let new = PlayerMeta
    {
        channel: message.channel_info.clone(),
        player_id,
        coordinates: (0, 0, 0),
        area_records: HashMap::new(),
        entity_knowledge: Vec::new(),
        name: String::from("New Player"),
        god: String::from("Godless heathen"),
        class: Melee,
        reusable_message: ReusableMessage::new()
    };
    register_player_meta(new);

    register_options(text::new_player_name(player_id));
    update_options_manually(player_id);
    send_blocking_message(player_id, text::rand_new_sender(), TEXT_SPEED);
}

pub fn register_player_meta(meta: PlayerMeta)
{
    unsafe { if let Some(ref mut registry) = PLAYER_META
    {
        registry.push(meta);
    }
    else { panic!("Error: Player meta registry not loaded in time."); }}
}

/**
 * Intended for storing whatever information the
 * player knows about any given entity.
 */
pub struct EntityKnowledge
{
    pub entity_id: usize,
    pub knows_name: bool,
    pub dialogue_marker: u8
}

impl EntityKnowledge
{
    pub fn new(entity_id: usize) -> EntityKnowledge
    {
        EntityKnowledge
        {
            entity_id,
            knows_name: false,
            dialogue_marker: 0
        }
    }
}

impl PartialOrd for EntityKnowledge
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering>
    {
        Some(self.cmp(other))
    }
}

impl Ord for EntityKnowledge
{
    fn cmp(&self, other: &Self) -> Ordering
    {
        if other.entity_id > self.entity_id { Greater }
        else if other.entity_id < self.entity_id { Less }
        else { Equal }
    }
}

impl PartialEq for EntityKnowledge
{
    fn eq(&self, other: &Self) -> bool
    {
        other.entity_id == self.entity_id
    }
}

impl Eq for EntityKnowledge {}