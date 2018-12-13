use crate::types::items::inventories::Inventory;
use crate::util::timed_events::DelayHandler;
use crate::messages::MessageComponent::*;
use crate::types::{effects::Effect};
use crate::traits::{Entity, Item};
use crate::util::access;
use crate::*;

use std::cell::{Cell, RefCell};
use std::sync::Arc;

pub struct Player {
    name: String,
    metadata: Arc<PlayerMeta>,
    health: Cell<u32>,
    base_damage: Cell<u32>,
    max_health: Cell<u32>,
    health_bonus: Cell<u32>, //to-do: convert this into armor points.
    attack_speed: Cell<i32>,
    item_speed: Cell<i32>,
    pub main_inventory: Inventory,
    money: Cell<u32>,
    weapon_slot: Inventory,
    offhand_slot: Inventory,
    current_effects: RefCell<Vec<Effect>>,
}

impl Player {
    pub const MIN_DAMAGE: u32 = 5;
    pub const MAX_HEALTH: u32 = 100;
    pub const MIN_HEALTH: u32 = 5;
    pub const MAX_ATK_SPEED: i32 = 10000;
    pub const MIN_ATK_SPEED: i32 = -10000;
    pub const MAX_ITEM_SPEED: i32 = 10000;
    pub const MIN_ITEM_SPEED: i32 = -10000;

    pub fn new(meta: Arc<PlayerMeta>) -> Player {
        Player {
            name: meta.get_name(),
            metadata: meta,
            health: Cell::new(20),
            base_damage: Cell::new(5),
            max_health: Cell::new(20),
            health_bonus: Cell::new(0),
            attack_speed: Cell::new(0),
            item_speed: Cell::new(0),
            main_inventory: Inventory::new(15),
            money: Cell::new(0),
            weapon_slot: Inventory::new(1),
            offhand_slot: Inventory::new(1),
            current_effects: RefCell::new(Vec::new()),
        }
    }

    pub fn send_message(&self, typ: MessageComponent, msg: &str, ms_speed: u64) -> DelayHandler {
        self.metadata.send_message(typ, msg, ms_speed)
    }

    pub fn send_short_message(&self, msg: &str) {
        self.metadata.send_short_message(msg);
    }

    /// This is used to correct effect values so that removing
    /// the effect will properly restore the original levels.
    pub fn update_effect<F>(&self, name: &str, callback: F) -> bool
        where F: FnOnce(&mut Effect)
    {
        let mut effects = self.current_effects.borrow_mut();
        let index = effects.iter().position(|e| e.name == name);
        if let Some(num) = index {
            if let Some(ref mut effect) = effects.get_mut(num) {
                callback(effect);
                return true;
            }
        }
        false
    }

    pub fn has_special_item(&self, typ: &str, _info: Option<&str>) -> bool {
        self.main_inventory.for_each_item(|item| {
            if item.get_type() == typ {
                Some(true)
            } else {
                None
            }
        })
        .is_some()
    }
}

impl Entity for Player {
    fn get_id(&self) -> usize {
        self.metadata.get_player_id()
    }

    fn get_name(&self) -> &String {
        &self.name
    }

    fn set_max_health(&self, val: u32) {
        self.max_health.set(val);
        let new = self.max_health.get();

        if new > Self::MAX_HEALTH {
            self.max_health.set(Self::MAX_HEALTH);
        } else if new < Self::MIN_HEALTH {
            self.max_health.set(Self::MIN_HEALTH);
        }
    }

    fn get_max_health(&self) -> u32 {
        self.max_health.get()
    }

    fn set_health(&self, health: u32) {
        self.health.set(health);
        self.update_health_bar();
    }

    fn get_health(&self) -> u32 {
        self.health.get() + self.health_bonus.get()
    }

    fn update_health_bar(&self) {
        self.metadata.update_message(HealthBar, &self.get_health_bar());
    }

    fn set_base_damage(&self, val: u32) {
        self.base_damage.set(val);
        let new = self.base_damage.get();

        if new < Self::MIN_DAMAGE {
            self.base_damage.set(Self::MIN_DAMAGE);
        }
    }

    fn get_base_damage(&self) -> u32 {
        self.base_damage.get()
    }

    fn set_attack_speed(&self, val: i32) {
        self.attack_speed.set(val);
        let new = self.attack_speed.get();

        if new > Self::MAX_ATK_SPEED {
            self.attack_speed.set(Self::MAX_ATK_SPEED);
        } else if new < Self::MIN_ATK_SPEED {
            self.attack_speed.set(Self::MIN_ATK_SPEED);
        }
    }

    fn get_attack_speed(&self) -> i32 {
        self.attack_speed.get()
    }

    fn set_item_speed(&self, val: i32) {
        self.item_speed.set(val);
        let new = self.item_speed.get();

        if new > Self::MAX_ITEM_SPEED {
            self.item_speed.set(Self::MAX_ITEM_SPEED);
        } else if new < Self::MIN_ITEM_SPEED {
            self.item_speed.set(Self::MIN_ITEM_SPEED);
        }
    }

    fn get_item_speed(&self) -> i32 {
        self.item_speed.get()
    }

    fn get_inventory(&self) -> Option<&Inventory> {
        Some(&self.main_inventory)
    }

    fn give_item(&self, item: Box<Item>) {
        self.main_inventory.add_item(item, Some(self));
    }

    fn take_item_id(&self, id: usize) -> Option<Box<Item>> {
        if let Some(item) = self.main_inventory.take_item_id(id, Some(self)) {
            return Some(item);
        }
        if let Some(item) = self.weapon_slot.take_item_id(id, Some(self)) {
            return Some(item);
        }
        self.offhand_slot.take_item_id(id, Some(self))
    }

    fn equip_item(&self, slot_num: usize) {
        if slot_num > self.main_inventory.current_size() {
            return;
        }

        let is_weapon = self.main_inventory.get_item_info(slot_num - 1, 0, |item| {
            item.on_equip(self);
            item.is_weapon()
        });

        let slot = if is_weapon {
            &self.weapon_slot
        } else {
            &self.offhand_slot
        };

        if slot.current_size() > 0 {
            slot.get_item_info(0, 0, |item| {
                item.on_unequip(self);
            });

            slot.transfer(0, &self.main_inventory, None, None);
        }
        self.main_inventory.transfer(slot_num - 1, slot, None, None);

        self.update_health_bar();
    }

    fn use_item(&self, item_num: usize, use_on: Option<&Entity>) {
        if self.main_inventory.current_size() < item_num {
            temp_send_short_message(self.get_id(), "Invalid item #.");
            return;
        }

        access::area(self.get_coordinates(), |area| {
            self.main_inventory
                .on_use_item(item_num - 1, Some(self), use_on, area);
        })
        .expect("The player's current area could not be found.");
    }

    fn use_primary(&self) {
        if self.weapon_slot.current_size() < 1 {
            self.metadata.send_short_message("This item no longer exists.");
            return;
        }

        access::area(self.get_coordinates(), |area| {
            self.weapon_slot.on_use_item(0, Some(self), None, area);
        })
        .expect("The player's current area could not be found.");
    }

    fn use_secondary(&self) {
        if self.offhand_slot.current_size() < 1 {
            self.metadata.send_short_message("This item no longer exists.");
            return;
        }

        access::area(self.get_coordinates(), |area| {
            self.offhand_slot.on_use_item(0, Some(self), None, area);
        })
        .expect("The player's current area could not be found.");
    }

    fn get_primary(&self) -> String {
        if self.weapon_slot.current_size() > 0 {
            return self.weapon_slot
                .get_item_info(0, 0, |item| item.get_name().clone());
        }
        String::from("None")
    }

    fn get_secondary(&self) -> String {
        if self.offhand_slot.current_size() > 0 {
            return self.offhand_slot
                .get_item_info(0, 0, |item| item.get_name().clone());
        }
        String::from("None")
    }

    fn give_money(&self, amount: u32) {
        let current = self.money.get();
        self.money.set(current + amount);
        self.update_health_bar();
    }

    fn take_money(&self, amount: u32) {
        let current = self.money.get();
        self.money.set(current.checked_sub(amount).unwrap_or(0));
        self.update_health_bar();
    }

    fn get_money(&self) -> u32 {
        self.money.get()
    }

    fn has_effect(&self, name: &str) -> bool {
        self.current_effects.borrow()
            .iter()
            .find(|e| e.name == name)
            .is_some()
    }

    fn give_effect(&self, effect: Effect) {
        self.current_effects.borrow_mut().push(effect);
        self.update_health_bar();
    }

    fn apply_effect(&self, name: &str) {
        self.current_effects.borrow()
            .iter()
            .find(|e| e.name == name)
            .and_then(|e| Some(e.apply(self)));
    }

    fn remove_effect(&self, name: &str) {
        let mut effects = self.current_effects.borrow_mut();

        effects.iter()
            .position(|e| e.name == name)
            .and_then(|i| {
                let effect = effects.remove(i);
                Some(effect.remove(self))
            });
    }

    fn clear_effects(&self) {
        self.current_effects.borrow_mut().clear();
    }

    fn kill_entity(&self) {
        self.metadata.area(|current| {
            let current_town = current.get_coordinates().0;
            access::starting_area(current_town, |new| {
                current.transfer_to_area(self.get_id(), new)
            });
        });
    }

    fn as_player(&self) -> Option<&Player> {
        Some(self)
    }

    fn set_coordinates(&self, coords: (usize, usize, usize)) {
        self.metadata.set_coordinates(coords);
    }

    fn get_coordinates(&self) -> (usize, usize, usize) {
        self.metadata.get_coordinates()
    }

    fn on_enter_area(&self, coords: (usize, usize, usize)) {
        self.set_coordinates(coords);
    }

    fn get_type(&self) -> &str {
        "player"
    }
}