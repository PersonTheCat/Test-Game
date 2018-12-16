use crate::messages::{ChannelInfo, ReusableMessage};
use crate::util::timed_events::DelayHandler;
use crate::types::classes::Class::{self, *};
use crate::messages::MessageComponent::*;
use crate::util::access::EntityAccessor;
use crate::traits::{Area, Entity};
use crate::types::towns::Town;
use crate::util::access;
use crate::GameMessage;
use crate::text;
use crate::*;

use atomic::Atomic;
use hashbrown::HashMap;
use lazy_static::lazy_static;
use parking_lot::Mutex;
use rand::random;

use std::cmp::Ordering::{self, *};
use std::sync::atomic::Ordering::*;
use std::sync::Arc;

/// Player registry is stored in a mutex so that only the game thread
/// may access it while it's running. Contents of the registry are
/// wrapped in reference counters so that player entities may also
/// access their user's information. Mutability of PlayerMeta is
/// handled in the interior to ensure that information is only
/// set and / or received, releasing the lock immediately, so that
/// multiple sources can access it while in scope.
pub type PlayerRegistry = Vec<Arc<PlayerMeta>>;

/// Areas can store whatever information they want. When accessing
/// area records from a player, the preference is that `record == 0`
/// as opposed to `None`, as this is the default state.
pub type AreaRecords = HashMap<(usize, usize, usize), HashMap<&'static str, u8>>;

lazy_static! {
    pub static ref PLAYER_META: Mutex<PlayerRegistry> = Mutex::new(Vec::new());
}

/// ##To-do:
/// This function will be used to load information about players
/// from the disk.
pub fn setup_player_registry() {}

pub struct PlayerMeta {
    channel: Mutex<ChannelInfo>,
    player_id: usize,
    coordinates: Atomic<(usize, usize, usize)>,
    area_records: Mutex<AreaRecords>,
    entity_knowledge: Mutex<Vec<EntityKnowledge>>,
    name: Mutex<String>,
    god: Mutex<String>, // Could possibly be a &'static str
    class: Atomic<Class>,
    active: Atomic<bool>,
    reusable_message: Mutex<ReusableMessage>,
    text_speed: Atomic<u64>,
    text_length: Atomic<usize>
}

impl PlayerMeta {
    /// Reuses the existing dialogue info to refresh the screen.
    pub fn refresh_message(&self) {
        self._send(0);
    }

    /// Standard dialogue to the player. Returns a DelayHandler
    /// for spawning new events upon completion.
    pub fn send_message(&self, typ: MessageComponent, msg: &str) -> DelayHandler {
        self.update_message(typ, msg);
        self._send(self.get_text_speed())
    }

    /// Sends an immediate message. Currently allows up to 3
    /// short messages to be displayed at once. In the future,
    /// this will be stored as a setting that each player can
    /// choose.
    pub fn send_short_message(&self, msg: &str) {
        self.add_short_message(msg);
        self._send(0);
    }

    /// Variant of send_message() that replaces the player's
    /// dialogue with Dialogue::empty(), temporarily
    /// preventing them from registering any inputs.
    pub fn send_blocking_message(&self, msg: &str) -> DelayHandler {
        let player_id = self.get_player_id();
        let dialogues = remove_all_options(player_id);
        let empty = Dialogue::empty(player_id);
        let empty_id = empty.id;
        register_options(empty);

        let handler = self.send_message(General, msg);
        handler.clone().then(move || {
            delete_options(empty_id);
            for dialogue in dialogues {
                _register_options(dialogue);
            }
            temp_update_options(player_id);
        });
        handler
    }

    pub fn send_current_options(&self) {
        let options_text = get_options_text(self.get_player_id());
        self.update_message(Options, &options_text);
        self._send(0);
    }

    pub fn update_options(&self) {
        let options_text = get_options_text(self.get_player_id());
        self.update_message(Options, &options_text);
    }

    pub fn get_send_area_options(&self) {
        let new_options = access::area(self.get_coordinates(), |a| a.get_dialogue(self))
            .expect("Area was somehow deleted.");
        register_options(new_options);
        self.send_current_options();
    }

    pub fn replace_send_options(&self, old_options: usize, new_options: Dialogue) {
        replace_options(self.get_player_id(), old_options, new_options);
        self.send_current_options();
    }

    pub fn replace_options(&self, old_options: usize, new_options: Dialogue) {
        replace_options(self.get_player_id(), old_options, new_options);
        self.update_options();
    }

    pub fn has_primary_dialogue(&self) -> bool {
        CURRENT_OPTIONS.lock()
            .iter()
            .find(|o| o.is_primary && o.player_id == self.player_id)
            .is_some()
    }

    pub fn update_message(&self, typ: MessageComponent, msg: &str) {
        let mut reusable_message = self.reusable_message.lock();
        match typ {
            HealthBar => reusable_message.health_bar = msg.to_string(),
            General => reusable_message.set_general(self.get_text_length(), msg),
            Options => reusable_message.options = msg.to_string(),
        };
    }

    /// Send a short message to the player. Does not update
    /// immediately. Use this to avoid repeatedly refreshing
    /// the text.
    pub fn add_short_message(&self, msg: &str) {
        let fmt = if msg.starts_with("§") {
            format!("* {}\n", text::auto_break(2, self.get_text_length(), &msg[2..]))
        } else {
            format!("* {}\n", msg)
        };
        self.reusable_message.lock().add_to_general(self.get_text_length(), fmt);
    }

    fn _send(&self, ms_speed: u64) -> DelayHandler {
        messages::send_message_to_channel(&self.channel.lock(), &mut *self.reusable_message.lock(), ms_speed)
    }

    /// Used for retrieving the actual entity controlled by the
    /// player, as this relationship is unidirectional and comes
    /// from the Entity -> PlayerMeta, not the other way around.
    pub fn get_accessor(&self) -> EntityAccessor {
        EntityAccessor {
            coordinates: self.get_coordinates(),
            entity_id: self.player_id,
            is_player: true,
        }
    }

    /// Used for interacting with the entity that represents the
    /// player without actually removing it from its container
    /// or worrying about lifetime parameters.
    pub fn entity<F, T>(&self, callback: F) -> T where F: FnOnce(&Entity) -> T {
        access::entity(self.get_accessor(), callback)
            .expect("Error: The entity associated with this player could not be found.")
    }

    /// See `entity()`. These are shorthand methods.
    pub fn area<F, T>(&self, callback: F) -> T where F: FnOnce(&Area) -> T {
        access::area(self.get_coordinates(), callback)
            .expect("Error: The area associated with this player could not be found.")
    }

    /// See `entity()`. These are shorthand methods.
    pub fn town(&self) -> Arc<Town> {
        access::town(self.get_coordinates().0)
    }

    /// Returns a cloned instance of the player's channel.
    /// Cloning occurs so the lock can be immediately released.
    pub fn get_channel(&self) -> ChannelInfo {
        self.channel.lock().clone()
    }

    pub fn set_channel(&self, channel: ChannelInfo) {
        *self.channel.lock() = channel;
    }

    pub fn get_player_id(&self) -> usize {
        self.player_id
    }

    pub fn set_coordinates(&self, area: (usize, usize, usize)) {
        self.coordinates.store(area, SeqCst);
    }

    pub fn get_coordinates(&self) -> (usize, usize, usize) {
        self.coordinates.load(SeqCst)
    }

    pub fn player_has_visited(&self, area: (usize, usize, usize)) -> bool {
        self.area_records.lock().contains_key(&area)
    }

    pub fn add_record_book(&self, area: (usize, usize, usize)) {
        self.area_records.lock().insert(area, HashMap::new());
    }

    pub fn set_record(&self, coords: (usize, usize, usize), record: &'static str, val: u8) {
        if let Some(ref mut records) = self.area_records.lock().get_mut(&coords) {
            records.insert(record, val);
            return;
        }
        self.create_record(coords, record, val);
    }

    /// Locates records for this area and increments them by one.
    /// Inserts a value of 1 when those records do not exist.
    pub fn incr_record(&self, coords: (usize, usize, usize), record: &'static str) {
        if let Some(ref mut records) = self.area_records.lock().get_mut(&coords) {
            if let Some(ref mut num) = records.get_mut(record) {
                **num += 1;
                return;
            }
            records.insert(record, 1);
            return;
        }
        self.create_record(coords, record, 1);
    }

    /// Locates records for `coords` and determines the value of
    /// `record`. Returns a value of 0 when no records exist.
    pub fn get_record(&self, coords: (usize, usize, usize), record: &'static str) -> u8 {
        if let Some(ref mut records) = self.area_records.lock().get_mut(&coords) {
            if let Some(num) = records.get(record) {
                return *num;
            }
            records.insert(record, 0);
            return 0;
        }
        0
    }

    pub fn create_record(&self, coords: (usize, usize, usize), record: &'static str, val: u8) {
        let mut new_records = HashMap::new();
        new_records.insert(record, val);
        self.area_records.lock().insert(coords, new_records);
    }

    /// Uses a binary search to locate or insert a new knowledge
    /// container for an entity with `entity_id`.
    pub fn add_entity_knowledge(&self, entity_id: usize) {
        let mut knowledge = self.entity_knowledge.lock();

        knowledge.binary_search_by(|e| e.entity_id.cmp(&entity_id))
            .err()
            .and_then(|index| Some(knowledge.insert(index, EntityKnowledge::new(entity_id))));
    }

    /// Uses a binary search to determine whether a knowledge
    /// container exists for an entity with `entity_id`.
    pub fn has_entity_knowledge(&self, entity_id: usize) -> bool {
        self.entity_knowledge.lock()
            .binary_search_by(|e| e.entity_id.cmp(&entity_id))
            .is_ok()
    }

    pub fn get_dialogue_marker(&self, entity_id: usize) -> Option<u8> {
        let knowledge = self.entity_knowledge.lock();
        knowledge.binary_search_by(|e| e.entity_id.cmp(&entity_id))
            .ok()
            .and_then(|index| {
                let of_entity: &EntityKnowledge = knowledge.get(index).unwrap();
                Some(of_entity.dialogue_marker)
            })
    }

    pub fn set_name(&self, name: String) {
        *self.name.lock() = name;
    }

    /// Returns a cloned instance of the player's name.
    /// Cloning occurs so the lock can be immediately released.
    pub fn get_name(&self) -> String {
        self.name.lock().clone()
    }

    pub fn set_god(&self, god: String) {
        *self.god.lock() = god;
    }

    pub fn get_god(&self) -> String {
        self.god.lock().clone()
    }

    pub fn set_class(&self, class: Class) {
        self.class.store(class, SeqCst);
    }

    pub fn get_class(&self) -> Class {
        self.class.load(SeqCst)
    }

    pub fn set_active(&self, b: bool) {
        self.active.store(b, SeqCst);
    }

    pub fn is_active(&self) -> bool {
        self.active.load(SeqCst)
    }

    pub fn set_text_speed(&self, val: u64) {
        self.text_speed.store(val, SeqCst);
    }

    pub fn get_text_speed(&self) -> u64 {
        self.text_speed.load(SeqCst)
    }

    pub fn set_text_length(&self, val: usize) {
        self.text_length.store(val, SeqCst);
    }

    pub fn get_text_length(&self) -> usize {
        self.text_length.load(SeqCst)
    }
}

pub fn new_player_event(message: &GameMessage) {
    let new = PlayerMeta {
        channel: Mutex::new(message.channel_info.clone()),
        player_id: random(),
        coordinates: Atomic::new((0, 0, 0)),
        area_records: Mutex::new(HashMap::new()),
        entity_knowledge: Mutex::new(Vec::new()),
        name: Mutex::new(String::from("New Player")),
        god: Mutex::new(String::from("Godless heathen")),
        class: Atomic::new(Melee),
        active: Atomic::new(true),
        reusable_message: Mutex::new(ReusableMessage::new()),
        text_speed: Atomic::new(TEXT_SPEED),
        text_length: Atomic::new(LINE_LENGTH)
    };
    let id = new.player_id;
    register_options(text::new_player_name(id));
    register_player_meta(new);
    let registered = access::player_meta(id);
    registered.update_options();
    registered.send_blocking_message(&text::rand_new_sender());
}

pub fn register_player_meta(meta: PlayerMeta) {
    PLAYER_META.lock().push(Arc::new(meta));
}

/// Intended for storing whatever information the
/// player knows about any given entity.
pub struct EntityKnowledge {
    pub entity_id: usize,
    pub knows_name: bool,
    pub dialogue_marker: u8,
}

impl EntityKnowledge {
    pub fn new(entity_id: usize) -> EntityKnowledge {
        EntityKnowledge {
            entity_id,
            knows_name: false,
            dialogue_marker: 0,
        }
    }
}

impl PartialOrd for EntityKnowledge {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for EntityKnowledge {
    fn cmp(&self, other: &Self) -> Ordering {
        if other.entity_id > self.entity_id {
            Greater
        } else if other.entity_id < self.entity_id {
            Less
        } else {
            Equal
        }
    }
}

impl PartialEq for EntityKnowledge {
    fn eq(&self, other: &Self) -> bool {
        other.entity_id == self.entity_id
    }
}

impl Eq for EntityKnowledge {}