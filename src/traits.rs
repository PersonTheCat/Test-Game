use crate::types::effects::Effect;
use crate::types::entities::{mobs::Mob, npcs::NPC, players::Player};
use crate::types::items::{self, bows::Bow, display_info::ItemDisplayInfo, inventories::Inventory, swords::Sword};
use crate::player_data::PlayerMeta;
use crate::text;
use crate::util::access::{self, EntityAccessor};
use crate::util::player_options::{Command, Dialogue, Response};
use crate::*;

use self::AttemptedPurchase::*;
use self::AttemptedSale::*;

use std::any::Any;

use parking_lot::MutexGuard;
use lazy_static::lazy_static;
use rand::random;

/// //////////////////////////////////////////////////////
///                     # Areas
/// //////////////////////////////////////////////////////

/// The standard interface which allows dynamic dispatch
/// for structs that serve as Areas in-game.
pub trait Area: EntityHolder + AreaTools {
    /// A formatted variant of `get_title()` which, by default,
    /// includes both the area's number and town number.
    fn get_formatted_title(&self) -> String {
        format!("Town #{}; Area #{}: {}",
            self.get_town_num(),
            self.get_area_num(),
            self.get_title()
        )
    }

    /// This area's identifier. A string is required by default
    /// to allow classes to be added to the game by external
    /// binaries.
    fn get_type(&self) -> &'static str;

    /// A typically three-character identifier that will
    /// represent this area on the map.
    fn get_map_icon(&self) -> &'static str;

    /// Determines whether the input `player` can enter this
    /// area at the given time.
    fn can_enter(&self, _player: &Player) -> bool {
        true
    }

    /// An optional message that will be displayed when the
    /// player first enters the area.
    fn get_entrance_message(&self) -> Option<String> {
        None
    }

    /// This area's title.
    fn get_title(&self) -> String;

    /// Whether mobs should spawn in this area.
    fn should_mobs_spawn(&self) -> bool {
        false
    }

    /// Whether a particular item can be used in this area.
    /// May currently be unused.
    fn can_use_item(&self, _item: &Item) -> bool {
        true
    }

    /// Whether the input `player` is allowed to leave the
    /// area at the current time. By default, players are
    /// not allowed to leave whenever the area currently
    /// contains mobs, and thus must complete the ongoing
    /// fight sequence beforehand.
    fn can_leave(&self, _player: &Player) -> bool {
        !self.contains_mobs()
    }

    /// This area's guaranteed drop. This will be used to
    /// ensure that at least one area in any given town
    /// drops a key to the exit gate.
    fn set_guaranteed_item(&self, _item: Box<Item>) {}

    fn get_guaranteed_item(&self) -> Option<Box<Item>> {
        None
    }

    /// Optionally provides info for the player's dialogue
    /// while in this area. By default, this info is just a
    /// map of the current town, but it would be possible to
    /// concatenate additional info manually.
    fn get_dialogue_info(&self, player: &PlayerMeta) -> Option<String> {
        Some(get_new_map(self.get_coordinates().0, player))
    }

    /// To-do
    fn fight_sequence(&self, player: &PlayerMeta) -> Dialogue {
        Dialogue::empty(player.get_player_id())
    }

    /// These responses allow the player to move between areas.
    /// By default, these movements are generated from the area's
    /// `connections`, a required field; however, this method
    /// could be overridden to add or remove connections to any
    /// area. Other types of responses should be organized into
    /// `get_specials()`, as this method would more clearly
    /// indicate their purpose.
    fn get_movements(&self, _player: &PlayerMeta, responses: &mut Vec<Response>) {
        let current = self.get_coordinates();
        let connections = self.get_connections();
        let num_connections = connections.len();

        for coordinates in connections {
            let text = get_direction_label(num_connections, current, coordinates);
            responses.push(Response::_simple(text, move |p: &PlayerMeta| {
                access::area(current, |old| {
                    access::area(coordinates, |new| {
                        old.transfer_to_area(p.get_player_id(), new);
                    });
                });}
            ));
        }
    }

    /// These responses will generate interactions between
    /// entities, e.g. waving to another player. Responses
    /// that are unique to the area's purpose (i.e. why a
    /// player would visit the area) might make more sense
    /// under `get_specials()`.
    fn get_entity_interactions(&self, player: &PlayerMeta, responses: &mut Vec<Response>) {
        let lock = self.borrow_entity_lock();
        let entities = lock.iter()
            .filter(|e| e.get_id != player.get_player_id());

        for entity in entities {
            // Make sure there is response info and then generate dialogue from it.
            if let Some(text) = entity.get_response_text(player) {
                responses.push(Response::_get_entity_dialogue(text, entity.get_accessor()));
            }
            // Special interactions for other players.
            if let Some(_) = entity.as_player() {
                responses.push(wave_response(entity));
                responses.push(trade_response(entity));
            }
        }
    }

    /// Special responses related to this area. Example uses
    /// include throwing coins into a fountain, praying to
    /// altars, and gambling.
    fn get_specials(&self, _player: &PlayerMeta, _responses: &mut Vec<Response>) {}

    /// Standard commands to be generated for this area. By
    /// default, these commands include opening the player's
    /// inventory and (if applicable) using their secondary
    /// item. Could be overridden to allow additional commands.
    /// In the future, global commands or commands that are
    /// not intended to be displayed should be registered
    /// through `global_commands`, although this function is
    /// not yet ready for use.
    fn get_commands(&self, player: &PlayerMeta, commands: &mut Vec<Command>) {
        commands.push(Command::goto_dialogue(
            "i", "View your inventory",
            move |player| {
                player.entity(|entity| {
                    entity.get_inventory()
                        .expect("Player does not have an inventory.")
                        .get_dialogue(player)
                })
            },
        ));

        if player.entity(|e| e.get_secondary() != "None") {
            commands.push(Command::simple("s", "Use your secondary item.", |_, p| {
                p.entity(|e| e.use_secondary());
            }));
        }
    }

    /// Handles generating the dialogue that will be
    /// displayed while the player is in this area. There
    /// should be no need to override this method.
    fn get_dialogue(&self, player: &PlayerMeta) -> Dialogue {
        if self.contains_mobs() {
            return self.fight_sequence(player);
        }

        let mut responses = Vec::new();
        let mut commands = Vec::new();

        self.get_movements(player, &mut responses);
        self.get_specials(player, &mut responses);
        self.get_entity_interactions(player, &mut responses);
        self.get_commands(player, &mut commands);

        let coordinates = self.get_coordinates();
        let entrance_message = if !player.player_has_visited(coordinates) {
            //To-do: find a better place for this.
            player.add_record_book(coordinates);
            self.get_entrance_message()
        } else {
            None
        };

        Dialogue {
            title: self.get_formatted_title(),
            text: entrance_message,
            info: self.get_dialogue_info(player),
            responses,
            commands,
            text_handler: None,
            player_id: player.get_player_id(),
            id: random(),
        }
    }
}

/// //////////////////////////////////////////////////////
///           # Area: Default impl functions
/// //////////////////////////////////////////////////////

// To-do: Work on all of these a bit.

/// Accesses the area at the specified coordinates to retrieve
/// its title. Returns `""` if nothing is found, but this
/// should be impossible.
fn get_new_area_title(coords: (usize, usize, usize)) -> String {
    match access::area(coords, |a| a.get_title()) {
        Some(title) => title,
        None => String::new(),
    }
}

/// Determines whether to display `Walk away from...` or
/// `Go [direction]: [title]`
fn get_direction_label(num_connections: usize, from: (usize, usize, usize), to: (usize, usize, usize)) -> String {
    let direction = get_direction(from, to)
        .expect("get_direction_label() did not error correctly.");
    if num_connections == 1 {
        format!("Walk away from the {}", get_new_area_title(from))
    } else {
        format!("Go {}: {}", direction, get_new_area_title(to))
    }
}

/// To-do: Possibly just use "next" / "previous."
/// Would have to add on world gen. This is
/// mildly bad.
fn get_direction(from: (usize, usize, usize), to: (usize, usize, usize)) -> Option<&'static str> {
    if to.2 == from.2 {
        if to.1 == from.1 + 1 {
            return Some("forward");
        } else if to.1 == from.1 - 1 {
            return Some("backward");
        }
        panic!("Error: Indirect connections are not yet implemented. Tried to skip a z coordinate.");
    } else if to.1 == from.1 {
        if to.2 == from.2 + 1 {
            return Some("right");
        } else if to.2 == from.2 - 1 {
            return Some("left");
        }
        panic!("Error: Indirect connections are not yet implemented. Tried to skip an x coordinate.");
    }
    panic!("Error: Indirect connections are not yet implemented. Tried to move diagonally.");
}

/// Wave to another player.
fn wave_response(entity: &Entity) -> Response {
    let receiver_id = entity.get_id();
    let text = format!("Wave to {}.", entity.get_name());
    Response::_action_only(text, move |p| {
        let msg = *choose(&[
            "<name> says hello!",
            "<name> says hi!",
            "<name>, a fellow player, has called out\nto you.",
            "You have been contacted by <name>.",
            "A strange creature known as \"<name>\"\nis shaking its hands at you.",
            "You notice a bizarre machination which\ncalls itself \"<name>\" staring in your\ndirection.",
            "You can't help but notice you're being\nwatched by <name>.",
            "You stop and gaze upon the horror that\nis <name>.",
            "Frightened, you turn around to get away\nfrom <name>.",
            "You must be special. <name> has been\nwatching you."
        ]);

        let formatted = text::apply_replacements(msg, &[("<name>", p.get_name())]);
        temp_add_short_message(receiver_id, &formatted);

        if !try_refresh_options(receiver_id) {
            p.send_short_message(*choose(&[
                "They were too busy to notice you, but heard your message.",
                "They didn't see you there, but got your message.",
            ]), );
        } else {
            p.send_current_options();
        } // Manually trigger refresh. There is a very strange bug associated.
    })
}

/// Currently does nothing.
fn trade_response(entity: &Entity) -> Response {
    Response::_text_only(format!("Trade with {}", entity.get_name()))
}

/// Derivable methods for `Area`.
pub trait AreaTools: Send + Sync {
    fn get_area_num(&self) -> usize;

    fn get_town_num(&self) -> usize;

    fn get_coordinates(&self) -> (usize, usize, usize);

    fn add_connection(&self, connection: (usize, usize, usize));

    fn get_connections(&self) -> Vec<(usize, usize, usize)>;

    fn as_entity_holder(&self) -> &EntityHolder;

    fn as_any(&self) -> &Any;
}

/// Derivable functions for `Area`. Can also be used
/// as a general purpose storage for entities.
pub trait EntityHolder {
    /// Returns whether the area contains an entity
    /// with the given type identifier.
    fn contains_type(&self, typ: &'static str) -> bool;

    /// Places a new entity in this area, calling
    /// `Entity#on_enter_area()` to handle related
    /// events.
    fn add_entity(&self, entity: Box<Entity>);

    /// Removes an entity from the area.
    fn remove_entity(&self, id: usize) -> Option<Box<Entity>>;

    /// Transfers an entity from this area to another
    /// Entity holder.
    fn transfer_entity(&self, id: usize, to: &EntityHolder);

    /// Determines whether an entity with the given
    /// `id` currently exists in this area.
    fn contains_entity(&self, id: usize) -> bool;

    /// Determines the entity's index inside of the
    /// `entities` vector. Used internally to allow
    /// entities to be handled more easily.
    fn get_entity_index(&self, id: usize) -> Option<usize>;

    /// Removes an entity from the area based on their
    /// index in the `entities` vector.
    fn take_entity_by_index(&self, index: usize) -> Box<Entity>;

    /// Determines whether any entity in the area is
    /// of type `mob`.
    fn contains_mobs(&self) -> bool {
        self.contains_type("mob")
    }

    /// Determines whether any entity in the area is
    /// of type `player`.
    fn contains_players(&self) -> bool {
        self.contains_type("player")
    }

    /// Determines whether any entity in the area is
    /// of type `npc`.
    fn contains_npcs(&self) -> bool {
        self.contains_type("npc")
    }

    /// A (hopefully temporary) method which allows
    /// entities inside of the `entities` vector to
    /// be accessed by external processes.
    fn borrow_entity_lock(&self) -> MutexGuard<Vec<Box<Entity>>>;

    /// A nicer-looking implementation of `transfer_
    /// entity`, which should look nicer in-use when
    /// transferring entities between actual `Area`s.
    fn transfer_to_area(&self, id: usize, area: &Area) {
        self.transfer_entity(id, area.as_entity_holder());
    }
}

/// //////////////////////////////////////////////////////
///                     # Items
/// //////////////////////////////////////////////////////

/// Generic speed caps.
pub const ATTACK_SPEED_MIN: i32 = -5000;
pub const ITEM_SPEED_MIN: i32 = -8000;

/// The standard interface which allows dynamic dispatch
/// for structs that serve as entities in-game.
pub trait Entity: Send + Sync {
    /// This entity's unique identifier.
    fn get_id(&self) -> usize;

    /// This area's in-game name.
    fn get_name(&self) -> &String;

    /// This entity's optional title, a subtext of their name.
    fn get_title(&self) -> Option<&String> {
        None
    }

    /// This entity's description. Used for generating monologue
    /// about them.
    fn get_description(&self) -> Option<&String> {
        None
    }

    fn set_max_health(&self, _val: u32) {}

    fn get_max_health(&self) -> u32 {
        15
    }

    fn set_health(&self, health: u32);

    fn get_health(&self) -> u32;

    /// Display's this user's current health bar.
    fn get_health_bar(&self) -> String {
        format!(
            "HP: ({} / {}); Dps: ({}); Gold: {}g\n\
             Prim: {}; Sec: {}",
            self.get_health(),
            self.get_max_health(),
            items::format_damage_2(self.get_base_damage(), self.get_attack_speed()),
            self.get_money(),
            self.get_primary(),
            self.get_secondary()
        )
    }

    /// An event used for retrieving the entity's health bar
    /// from `get_health_bar()` and displaying it to the screen.
    fn update_health_bar(&self) {}

    fn add_health(&self, health: i32) {
        let prior = self.get_health();

        self.set_health((prior as i32 + health) as u32);

        let adjusted = self.get_health();

        let max = self.get_max_health();

        if adjusted > max {
            self.set_health(max);
        }
    }

    fn remove_health(&self, health: u32) {
        let prior = self.get_health();

        self.set_health(prior - health);

        if self.get_health() == 0 {
            self.kill_entity()
        }
    }

    fn set_base_damage(&self, _val: u32) {}

    fn get_base_damage(&self) -> u32 {
        5
    }

    fn set_attack_speed(&self, _val: i32) {}

    fn add_attack_speed(&self, val: i32) {
        let current = self.get_attack_speed();
        let new = current + val;
        if new < ATTACK_SPEED_MIN {
            self.set_attack_speed(ATTACK_SPEED_MIN);
        } else {
            self.set_attack_speed(new);
        }
    }

    fn get_attack_speed(&self) -> i32 {
        0
    }

    fn set_item_speed(&self, _val: i32) {}

    fn add_item_speed(&self, val: i32) {
        let current = self.get_item_speed();
        let new = current + val;
        if new < ITEM_SPEED_MIN {
            self.set_item_speed(ITEM_SPEED_MIN);
        } else {
            self.set_item_speed(new);
        }
    }

    fn get_item_speed(&self) -> i32 {
        0
    }

    /// Borrows a reference to the entity's inventory.
    fn get_inventory(&self) -> Option<&Inventory> {
        None
    }

    /// Optionally retrieves the text that will be displayed
    /// for players to interact with this entity.
    fn get_response_text(&self, _player: &PlayerMeta) -> Option<String> {
        None
    }

    /// Optionally retrieves dialogue for players to interact
    /// with this entity.
    fn get_dialogue(&self, _player: &PlayerMeta) -> Option<Dialogue> {
        None
    }

    /// Allows separate dialogues to be retrieved on the
    /// basis of a u8 marker. To be used internally by
    /// implementors.
    fn goto_dialogue(&self, _marker: u8, _player: &PlayerMeta) -> Option<Dialogue> {
        None
    }

    /// A set of responses that are unique to this entity's
    /// trades. Currently unused.
    fn get_trades(&self, _player_id: usize, _trades: &mut Vec<Response>) {}

    fn give_item(&self, _item: Box<Item>) {}

    /// Takes an item from this entity based on its `id`.
    fn take_item_id(&self, _id: usize) -> Option<Box<Item>> {
        None
    }

    /// A function called to equip an item from this entity's
    /// inventory into one of their main slots.
    fn equip_item(&self, _slot_num: usize) {}

    fn unequip_item(&self, _id: usize) {}

    /// A function called to use the item in the specified
    /// slot, optionally applying its effect to `use_on`.
    fn use_item(&self, _item_num: usize, _use_on: Option<&Entity>) {}

    /// Uses the item in the entity's primary slot on
    /// the entity.
    fn use_primary(&self) {}

    /// Uses the item in the entity's secondary slot on
    /// the entity.
    fn use_secondary(&self) {}

    /// Retrieves text to display the entity's primary
    /// item on screen.
    fn get_primary(&self) -> String {
        String::from("None")
    }

    /// Retrieves text to display the entity's secondary
    /// item on screen.
    fn get_secondary(&self) -> String {
        String::from("None")
    }

    fn give_money(&self, _amount: u32) {}

    fn take_money(&self, _amount: u32) {}

    fn get_money(&self) -> u32 {
        0
    }

    fn can_afford(&self, amount: u32) -> bool {
        self.get_money() >= amount
    }

    fn has_effect(&self, _name: &str) -> bool {
        false
    }

    fn give_effect(&self, _effect: Effect) {}

    fn apply_effect(&self, _name: &str) {}

    fn remove_effect(&self, _name: &str) {}

    fn clear_effects(&self) {}

    /// The event that will be called whenever the entity
    /// is killed.
    fn kill_entity(&self);

    /// A convenience method for casting entities to `Player`s.
    fn as_player(&self) -> Option<&Player> {
        None
    }

    /// A convenience method for casting entities to `Mob`s.
    /// Might be removed.
    fn as_mob(&self) -> Option<&Mob> {
        None
    }

    /// A convenience method for casting entities to `Npc`s.
    /// Might be removed.
    fn as_npc(&self) -> Option<&NPC> {
        None
    }

    fn set_coordinates(&self, _coords: (usize, usize, usize)) {}

    fn get_coordinates(&self) -> (usize, usize, usize) {
        (0, 0, 0)
    }

    /// An event called by `EntityHolder#add_entity()` that
    /// fires as the player enters the area.
    fn on_enter_area(&self, _coords: (usize, usize, usize)) {}

    /// This entity's type identifier.
    fn get_type(&self) -> &'static str;

    /// Returns information related to retrieving the entity
    /// statically from its current position. Does not provide
    /// a safe, updatable solution for accessing entities, but
    /// is potentially faster than using reference counters.
    /// Maybe not, though.
    fn get_accessor(&self) -> EntityAccessor {
        EntityAccessor {
            coordinates: self.get_coordinates(),
            entity_id: self.get_id(),
            is_player: self.get_type() == "player",
        }
    }
}

lazy_static! {
    /// Could be removed.
    static ref NO_NAME: String = String::from("");
}

/// The standard Item trait. Designed to allow dynamic
/// dispatch for structs that will serve as in-game
/// items.
pub trait Item: ItemTools {
    /// This item's unique identifier.
    fn get_id(&self) -> usize;

    /// This items in-game identifier.
    fn get_name(&self) -> &String {
        &NO_NAME
    }

    fn get_level(&self) -> u32 {
        0
    }

    fn is_tradable(&self) -> bool {
        true
    }

    fn is_weapon(&self) -> bool {
        false
    }

    fn as_weapon(&self) -> Option<&Weapon> {
        None
    }

    fn get_price(&self) -> u32 {
        10
    }

    /// Allows item prices to be factored for special
    /// trades and other purposes.
    fn get_adjusted_price(&self, factor: f32) -> u32 {
        (self.get_price() as f32 * factor) as u32
    }

    /// The maximum amount of this item that can fit
    /// in an `ItemSlot`.
    fn max_stack_size(&self) -> u32 {
        4
    }

    /// This item's type identifier.
    fn get_type(&self) -> &'static str;

    /// A convenience method for casting items to `Sword`s.
    /// Will probably be removed.
    fn as_sword(&self) -> Option<&Sword> {
        None
    }

    /// A convenience method for casting items to `Bow`s.
    /// Will probably be removed.
    fn as_bow(&self) -> Option<&Bow> {
        None
    }

    /// Whether this item will apply effects to the Entity
    /// who uses it. To-do: Verify that.
    fn has_entity_effect(&self) -> bool {
        false
    }

    /// Returns whether the item can be used in the given
    /// area. Currently unused.
    fn can_use_item(&self, _area: &Area) -> bool {
        true
    }

    /// Uses the item, optionally applying information to
    /// the item's user and/or entity to be used on.
    /// Currently returns a string containing a message
    /// to the player upon use. This might be removed.
    fn use_item(&self, _user: Option<&Entity>, _use_on: Option<&Entity>, _area: &Area) -> Option<String> {
        None
    }

    /// An event called by `Inventory` that fires when the
    /// entity receives this item.
    fn on_get(&self, _entity: Option<&Entity>) {}

    /// An event called by `Inventory` that fires when the
    /// entity loses this item.
    fn on_lose(&self, _entity: Option<&Entity>) {}

    /// An event that fires when the entity equips this item
    /// to its main slots.
    fn on_equip(&self, _entity: &Entity) {}

    /// An event that fires when the entity removes this item
    /// from its main slots.
    fn on_unequip(&self, _entity: &Entity) {}

    /// The maximum number of times this item can be used.
    fn get_max_uses(&self) -> u32 {
        1
    }

    fn set_num_uses(&self, _val: u32) {}

    fn decrement_uses(&self) {
        self.set_num_uses(self.get_num_uses().checked_sub(1).unwrap_or(0));
    }

    fn get_num_uses(&self) -> u32 {
        1
    }

    /// Retrieves information about this item to be displayed
    /// on screen, coupled with the item's unique identifier,
    /// which will allow for it to be specifically referred to
    /// later on.
    fn get_display_info(&self, price_factor: f32) -> ItemDisplayInfo {
        ItemDisplayInfo {
            item_id: self.get_id(),
            info: format!(
                "{}\n  * Type: {}\n  * Price: {}g",
                self.get_name(),
                self.get_type(),
                self.get_adjusted_price(price_factor)
            ),
        }
    }
}

/// A derivable trait which can clone Atomics and Mutexes.
/// Not really viable for most external use cases.
pub trait AtomicClone: Clone {}

/// Derivable methods for `Item`.
pub trait ItemTools: Any + Send + Sync {
    fn clone_box(&self) -> Box<Item>;

    fn as_any(&self) -> &Any;
}

/// Originally intended to provide additional methods for
/// items that functioned as weapons. Still needs expansions.
pub trait Weapon: Item {
    fn set_damage(&self, _val: u32) {}

    fn get_damage(&self) -> u32 {
        5
    }

    fn get_repair_price(&self) -> u32 {
        self.get_price() / 2
    }
}

/// //////////////////////////////////////////////////////
///                     # Shops
/// //////////////////////////////////////////////////////

/// The result of selling an item to a shop. Should
/// probably be moved elsewhere.
pub enum AttemptedSale {
    StoreFull(Box<Item>),
    Sale(usize),
}

/// The result of purchasing an item a shop. Should
/// probably be moved elsewhere.
pub enum AttemptedPurchase {
    NotFound,
    CantAfford,
    CantHold,
    Purchase,
}

/// These are not stored as consistently as the other types,
/// and thus temporarily require use of raw pointers.
pub trait Shop: Send + Sync {
    /// Borrows a reference to this shops `Inventory`.
    fn borrow_inventory(&self) -> &Inventory;

    /// A temporary method used for retrieving a permanent
    /// reference to this shop. It is not possible to use
    /// reference counters in this context, due to the fact
    /// that shops can be stored in many different ways,
    /// and thus I am looking for a better solution.
    fn get_ptr(&self) -> *const Shop;

    /// Attempts to sell an item to the shop, returning an
    /// `AttemptedSale` containing the result.
    fn sell(&self, item: Box<Item>) -> AttemptedSale {
        let inventory = self.borrow_inventory();

        if inventory.can_add_item(&*item) {
            let payback = item.get_price() as f32 * self.sell_to_rate();
            inventory.add_item(item, None);
            Sale(payback as usize)
        } else {
            StoreFull(item)
        }
    }

    /// The rate at which this shop is willing to purchase
    /// items.
    fn sell_to_rate(&self) -> f32;

    /// The rate at which this shop will sell its items.
    fn buy_from_rate(&self) -> f32;

    /// Attempts to purchase an item from this shop. Returns
    /// the result in the form of an `AttemptedPurchase`.
    fn buy(&self, player: &PlayerMeta, item_id: usize, price_factor: f32) -> AttemptedPurchase {
        let inventory = self.borrow_inventory();
        let slot_num = inventory.get_slot_num(item_id);
        if let None = slot_num {
            return NotFound;
        }
        let slot_num = slot_num.unwrap();

        let (price, can_afford, can_hold) = inventory.get_item_info(slot_num, 0, |item| {
            access::entity(player.get_accessor(), |player| {
                let price = item.get_adjusted_price(price_factor);
                (
                    price,
                    player.can_afford(price),
                    player.get_inventory().unwrap().can_add_item(item),
                )
            })
            .expect("Area no longer contains entity.")
        });

        if !can_afford {
            CantAfford
        } else if !can_hold {
            CantHold
        } else {
            // Placement avoids borrow errors with item use.
            access::entity(player.get_accessor(), |entity| {
                entity.give_item(inventory.take_item(slot_num, None));
                entity.take_money(price);
            });

            if self.should_restock() {
                self.restock();
            }

            Purchase
        }
    }

    /// Whether this shop should currently replace its inventory.
    fn should_restock(&self) -> bool {
        self.borrow_inventory().current_size() == 0
    }

    /// An event that fires when it's time for the shop to
    /// restock its inventory.
    fn restock(&self);

    /// Retrieves the dialogue used by players for interacting
    /// with this shop.
    fn get_dialogue(&self, player: &PlayerMeta, allow_sales: bool, price_factor: f32) -> Dialogue {
        let inventory: &Inventory = self.borrow_inventory();
        let info = inventory.get_display_info(price_factor);
        let mut responses = Vec::new();
        let mut commands = Vec::new();

        self.get_responses(player, &info, allow_sales, &mut responses);
        self.get_commands(player, &info, allow_sales, price_factor, &mut commands);

        Dialogue {
            title: String::from("Trades"),
            info: Some(Inventory::format_display_info(&info)),
            responses,
            commands,
            player_id: player.get_player_id(),
            ..Dialogue::default()
        }
    }

    fn get_responses(&self, _player: &PlayerMeta, _items: &Vec<ItemDisplayInfo>, _allow_sales: bool, responses: &mut Vec<Response>) {
        responses.push(Response::text_only("Leave."));
    }

    fn get_commands(&self, _player: &PlayerMeta, items: &Vec<ItemDisplayInfo>, allow_sales: bool, price_factor: f32, commands: &mut Vec<Command>) {
        let mut item_ids = Vec::new();
        items.iter().for_each(|i| item_ids.push(i.item_id));

        commands.push(Command {
            name: String::from("buy"),
            input_desc: String::from("buy #"),
            output_desc: String::from("Buy item #."),
            run: self.process_buy(item_ids, price_factor),
            next_dialogue: Generate(self.refresh_dialogue(allow_sales, price_factor)),
        });

        if allow_sales {
            commands.push(Command::manual_desc(
                "sell", "sell #", "Sell item # from inventory.",
                |_args, player| {
                    player.send_short_message("Let's just pretend you sold that. ;)");
                },
            ));
        }
    }

    // Stylistic improvements needed for the dialogue.
    fn process_buy(&self, item_ids: Vec<usize>, price_factor: f32, ) -> Box<Fn(&Vec<&str>, &PlayerMeta)> {
        let ptr = self.get_ptr();

        Box::new(move |args: &Vec<&str>, player: &PlayerMeta| {
            if args.len() == 0 {
                return;
            }
            if item_ids.len() == 0 {
                player.send_short_message("There are no items to buy.");
                return;
            }
            let shop = unsafe {
                match ptr.as_ref() {
                    Some(s) => s,
                    None => {
                        player.add_short_message("The shop seems to have moved away.");
                        return;
                    }
                }
            };
            let item_num: usize = match args[0].parse() {
                Ok(num) => num,
                Err(_) => {
                    player.add_short_message("Not sure which item you're looking for.");
                    return;
                }
            };
            if item_ids.len() < item_num || item_num < 1 {
                player.add_short_message("I'm afraid I can't tell what you're looking for.");
                return;
            }

            let item_id: usize = item_ids[item_num - 1];

            match shop.buy(player, item_id, price_factor) {
                NotFound => {
                    player.add_short_message("Looks like someone already bought that item.");
                }
                CantAfford => {
                    player.add_short_message("You can't afford that.");
                }
                CantHold => {
                    player.add_short_message("You don't have enough room.");
                }
                Purchase => {
                    player.add_short_message("Purchase successful.");
                }
            };
        })
    }

    fn refresh_dialogue(&self, allow_sales: bool, price_factor: f32, ) -> Box<Fn(&PlayerMeta) -> Dialogue> {
        let ptr = self.get_ptr();

        Box::new(move |player: &PlayerMeta| {
            access::area(player.get_coordinates(), move |area| unsafe {
                match ptr.as_ref() {
                    Some(ref shop) => shop.get_dialogue(player, allow_sales, price_factor),
                    None => area.get_dialogue(player),
                }
            })
            .expect("Area no longer exists.")
        })
    }
}
