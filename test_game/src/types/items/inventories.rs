use crate::player_data::PlayerMeta;
use crate::traits::{Area, Entity, Item};
use crate::types::items::display_info::ItemDisplayInfo;
use crate::util::access;
use crate::util::player_options::{Command, Dialogue, Response};
use crate::*;

use parking_lot::RwLock;

use std::boxed::Box;

pub struct ItemSlot {
    stack: RwLock<Vec<Box<Item>>>,
    kind: &'static str,
    pub max_count: u32,
}

impl ItemSlot {
    /// Constructs a raw ItemSlot used for storing items
    /// in an inventory. Could be used independently from
    /// `Inventory`, but this operation would require too
    /// many features to be reimplemented to be worth it.
    pub fn new(item: Box<Item>) -> ItemSlot {
        let max_count = item.max_stack_size();
        let mut stack = Vec::with_capacity(max_count as usize);
        let kind = item.get_type();

        stack.push(item);

        ItemSlot {
            stack: RwLock::new(stack),
            kind,
            max_count,
        }
    }

    /// Reports the maximum number of items this slot
    /// can hold, as originally determined by the item
    /// used to construct it.
    pub fn get_max_count(&self) -> u32 {
        self.max_count
    }

    /// Determines whether this slot is at capacity.
    pub fn can_hold_more(&self) -> bool {
        let stack = self.stack.read();
        stack.len() < stack.capacity()
    }

    /// Variant of `can_hold_more()` which determines
    /// whether the slot can hold an item of the
    /// specified type.
    pub fn can_add_item(&self, item: &Item) -> bool {
        if self.can_hold_more() {
            item.get_type() == self.kind
        } else {
            false
        }
    }

    /// Reports the current number of items in the slot.
    pub fn current_size(&self) -> usize {
        self.stack.read().len()
    }

    /// Used for adding new items to the slot.
    pub fn add_item(&self, item: Box<Item>) {
        self.stack.write().push(item);
    }

    /// Gets formatted information about the item and
    /// couples it with its ID, used for more specific
    /// referencing later on.
    pub fn get_display_info(&self, price_factor: f32) -> ItemDisplayInfo {
        let items = self.stack.read();
        let item = items.get(0)
            .expect("A slot existed, but there were no items in it.");

        let mut info = item.get_display_info(price_factor);
        info.info = format!("({}x) {}", self.current_size(), info.info);
        info
    }
}

pub struct Inventory {
    slots: RwLock<Vec<ItemSlot>>,
    pub max_size: usize,
}

impl Inventory {
    pub fn new(max_size: usize) -> Inventory {
        Inventory {
            slots: RwLock::new(Vec::new()),
            max_size,
        }
    }

    /// Performs an operation for each slot in the inventory.
    /// Does not allow any return information.
    pub fn for_each_slot<F>(&self, callback: F) where F: Fn(usize, &ItemSlot) {
        self.slots.read()
            .iter()
            .enumerate()
            .for_each(|(index, slot)| {
                callback(index, slot)
            });
    }

    // This is probably a little confusing.
    // Consider replacing it.
    pub fn for_each_item<T, F>(&self, mut callback: F) -> Option<T> where F: FnMut(&Item) -> Option<T> {
        let slots = self.slots.read();
        for slot in slots.iter() {
            let items = slot.stack.read();
            for item in items.iter() {
                if let Some(response) = callback(&**item) {
                    return Some(response);
                }
            }
        }
        None
    }

    /// Reports the current number of slots that are occupied.
    pub fn current_size(&self) -> usize {
        self.slots.read().len()
    }

    /// Adds an additional slot to the inventory. Could be private,
    /// but might also allow for some interesting item effects.
    pub fn add_slot(&self, slot: ItemSlot) {
        self.slots.write().push(slot);
    }

    /// Determines whether the inventory can hold any further items
    /// of any kind.
    pub fn can_hold_more(&self) -> bool {
        self.current_size() < self.max_size
    }

    /// Determines whether the inventory can hold the specified
    /// item.
    pub fn can_add_item(&self, item: &Item) -> bool {
        if self.can_hold_more() {
            return true;
        }

        for slot in self.slots.read().iter() {
            if slot.can_add_item(item) {
                return true;
            }
        }
        false
    }

    /// Adds an item to the inventory. Does not fail if no
    /// space is available.
    pub fn add_item(&self, item: Box<Item>, entity: Option<&Entity>) {
        item.on_get(entity);

        for slot in self.slots.read().iter() {
            if slot.can_add_item(&*item) {
                slot.add_item(item);
                return;
            }
        }
        self.add_slot(ItemSlot::new(item));
    }

    /// Returns an item from the specified `slot_num` and
    /// reports the updated space in that slot.
    fn get_owned_item(&self, slot_num: usize) -> (Box<Item>, usize) {
        let slots = self.slots.write();
        let slot = slots.get(slot_num).expect("Invalid slot #.");

        let mut items = slot.stack.write();
        let item = items.pop()
            .expect("Tried to pull an item from a slot which became empty.");

        (item, items.len())
    }

    /// Takes an item based on its `id`. This ensures that the
    /// correct item be removed from the inventory.
    pub fn take_item_id(&self, id: usize, from: Option<&Entity>) -> Option<Box<Item>> {
        match self.get_slot_num(id) {
            Some(slot_num) => Some(self.take_item(slot_num, from)),
            None => None,
        }
    }

    /// Variant of `take_item_id()` which pulls an item from
    /// the top of the specified `slot_num`. Uses `from` to
    /// determine whether to apply or remove effects from the
    /// specified entity.
    pub fn take_item(&self, slot_num: usize, from: Option<&Entity>) -> Box<Item> {
        let (item, slot_size) = self.get_owned_item(slot_num);
        let mut slots = self.slots.write();

        item.on_lose(from);

        // Make sure no slot is left empty.
        if slot_size < 1 {
            slots.remove(slot_num);
        }
        item
    }

    /// Allows temporary access into the inventory for retrieving
    /// information about an item. Items are not reference counted,
    /// and thus each of the four calls found in this function must
    /// both be present and remain in scope while the reference can
    /// be in use. Can return any information *except* for a reference
    /// to the item.
    pub fn get_item_info<T, F>(&self, slot_num: usize, item_num: usize, callback: F) -> T
        where F: Fn(&Item) -> T
    {
        let slots = self.slots.read();
        let slot = slots.get(slot_num).expect("Invalid slot #.");

        let items = slot.stack.read();
        let item = items.get(item_num).expect("Invalid item #.");

        callback(&**item)
    }

    /// Variant of `get_item_info()` which instead retrieves
    /// information about the specified `slot_num`, ignoring
    /// the particular items in that slot.
    pub fn get_slot_info<T, F>(&self, slot_num: usize, callback: F) -> T
        where F: Fn(&mut Vec<Box<Item>>) -> T
    {
        let slots = self.slots.read();
        let slot = slots.get(slot_num).expect("Invalid slot #.");

        let mut items = slot.stack.write();

        callback(&mut items)
    }

    // Looks like this is unable to check beyond
    // the first item in any slot.
    pub fn get_slot_num(&self, id: usize) -> Option<usize> {
        let slots = self.slots.read();

        slots.iter().position(|slot| {
            let items = slot.stack.read();
            let item = items.get(0)
                .expect("A slot existed, but there were no items in it.");

            item.get_id() == id
        })
    }

    /// Used for handling events related to using the item
    /// in this slot. Will ensure that num_uses is decremented
    /// and that the updated information is refreshed for the
    /// user.
    pub fn on_use_item(&self, slot_num: usize, user: Option<&Entity>, use_on: Option<&Entity>, area: &Area) {
        let (num_uses, response) = self.get_item_info(slot_num, 0, |item| {
            item.decrement_uses();
            (item.get_num_uses(), item.use_item(user, use_on, area))
        });

        if let Some(usr) = user {
            if num_uses <= 0 {
                self.take_item(slot_num, user);
                usr.update_health_bar();
            }
            if let Some(ref msg) = response {
                if let Some(player) = usr.as_player() {
                    player.send_short_message(msg);
                }
            }
        }
    }

    pub fn get_item_price(&self, slot_num: usize, item_num: usize) -> u32 {
        self.get_item_info(slot_num, item_num, |i| i.get_price())
    }

    /// Returns whether the transfer was successful.
    pub fn transfer(&self, from_slot: usize, other: &Inventory, from: Option<&Entity>, to: Option<&Entity>) -> bool {
        let can_add = self.get_item_info(from_slot, 0, |i| other.can_add_item(i));

        if can_add {
            let item = self.take_item(from_slot, from);
            other.add_item(item, to);
        }
        can_add
    }

    /// Transfers the item with `id` to the specified inventory.
    /// Uses `from` and `to` to apply effects to entities on
    /// pickup / drop.
    pub fn transfer_id(&self, id: usize, other: &Inventory, from: Option<&Entity>, to: Option<&Entity>) -> bool {
        self.get_slot_num(id)
            .and_then(|num| Some(self.transfer(num, other, from, to)))
            .is_some()
    }

    /// Gets the information used for displaying each item on
    /// screen. Contains their IDs such that specific items--
    /// and not just slot numbers--can be referenced later on.
    pub fn get_display_info(&self, price_factor: f32) -> Vec<ItemDisplayInfo> {
        let mut info = Vec::new();
        let slots = self.slots.read();

        for slot in slots.iter() {
            info.push(slot.get_display_info(price_factor));
        }

        info
    }

    /// Converts the display info from `get_display_info()` into
    /// something a little bit more appealing.
    pub fn format_display_info(info: &Vec<ItemDisplayInfo>) -> String {
        let mut ret = String::new();
        let mut index = 0;

        for item in info {
            index += 1;
            ret += &format!("#{}: {}", index, item.info);
            if index != info.len() {
                ret += "\n";
            }
        }
        ret
    }

    /// Generates the dialogue used for interacting with the
    /// inventory's contents.
    pub fn get_dialogue(&self, player: &PlayerMeta) -> Dialogue {
        let info = self.get_display_info(1.0);
        let mut responses = Vec::new();
        let mut commands = Vec::new();

        self.get_responses(player, &info, &mut responses);
        self.get_commands(player, &info, &mut commands);

        Dialogue {
            title: String::from("Inventory"),
            info: Some(Self::format_display_info(&info)),
            responses,
            commands,
            player_id: player.get_player_id(),
            ..Dialogue::default()
        }
    }

    pub fn get_responses(&self, _player: &PlayerMeta, _items: &Vec<ItemDisplayInfo>, responses: &mut Vec<Response>) {
        responses.push(Response::text_only("Close inventory."))
    }

    pub fn get_commands(&self, _player: &PlayerMeta, _items: &Vec<ItemDisplayInfo>, commands: &mut Vec<Command>) {
        commands.push(Self::equip_command());
        commands.push(Self::use_command());
    }

    fn equip_command() -> Command {
        Command {
            input: String::from("e #"),
            output_desc: String::from("Equip item #."),
            run: Box::new(|args: &Vec<&str>, player: &PlayerMeta| {
                if args.len() < 1 {
                    player.add_short_message("You must specify the item #.");
                    return;
                }
                let slot_num: usize = match args[0].parse() {
                    Ok(num) => num,
                    Err(_e) => {
                        player.add_short_message("Not sure what you're trying to do, there.");
                        return;
                    }
                };

                player.entity(move |entity| {
                    let inventory = entity
                        .get_inventory()
                        .expect("Player does not have an inventory.");

                    if inventory.current_size() < slot_num || slot_num == 0 {
                        player.add_short_message("Invalid item #.");
                        return;
                    }
                    entity.equip_item(slot_num);
                })
            }),
            next_dialogue: Self::get_next_dialogue()
        }
    }

    fn use_command() -> Command {
        Command {
            input: String::from("u #"),
            output_desc: String::from("Use item #."),
            run: Box::new(|args: &Vec<&str>, player: &PlayerMeta| {
                if args.len() < 1 {
                    player.add_short_message("You must specify the item #.");
                    return;
                }
                let item_num = match args[0].parse::<usize>() {
                    Ok(num) if num > 0 => num - 1,
                    _ => {
                        player.add_short_message("Not sure what you're trying to do, there.");
                        return;
                    }
                };

                access::context(player, |_, a, e| {
                    let inventory = e
                        .get_inventory()
                        .expect("Player no longer has an inventory.");

                    if inventory.current_size() < item_num || item_num == 0 {
                        player.add_short_message("Invalid item #.");
                        return;
                    }
                    inventory.on_use_item(item_num, Some(e), None, a);
                })
                    .expect("Player data no longer exists.");
            }),
            next_dialogue: Self::get_next_dialogue()
        }
    }

    fn get_next_dialogue() -> DialogueOption {
        Generate(Box::new(move |player: &PlayerMeta| {
            player.entity(|entity: &Entity| {
                entity.get_inventory()
                    .expect("Player not longer has an inventory")
                    .get_dialogue(player)
            })
        }))
    }
}
