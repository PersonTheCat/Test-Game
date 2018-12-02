use types::items::display_info::ItemDisplayInfo;
use player_options::{ Dialogue, Response, Command };
use player_data::PlayerMeta;
use traits::{ Item, Entity, Area };
use var_access;

use std::cell::RefCell;
use std::boxed::Box;

pub struct ItemSlot
{
    stack: RefCell<Vec<Box<Item>>>,
    kind: &'static str,
    pub max_count: u32
}

impl ItemSlot
{
    pub fn new(item: Box<Item>) -> ItemSlot
    {
        let max_count = item.max_stack_size();
        let mut stack = Vec::with_capacity(max_count as usize);
        let kind = item.get_type();

        stack.push(item);

        ItemSlot
        {
            stack: RefCell::new(stack),
            kind,
            max_count
        }
    }

    pub fn get_max_count(&self) -> u32
    {
        self.max_count
    }

    pub fn can_hold_more(&self) -> bool
    {
        let stack = self.stack.borrow();

        stack.len() < stack.capacity()
    }

    pub fn can_add_item(&self, item: &Item) -> bool
    {
        if self.can_hold_more()
        {
            item.get_type() == self.kind
        }
        else { false }
    }

    pub fn current_size(&self) -> usize
    {
        self.stack.borrow().len()
    }

    pub fn add_item(&self, item: Box<Item>)
    {
        self.stack.borrow_mut().push(item);
    }

    pub fn get_display_info(&self, price_factor: f32) -> ItemDisplayInfo
    {
        let items = self.stack.borrow();
        let item = items.get(0)
            .expect("A slot existed, but there were no items in it.");

        let mut info = item.get_display_info(price_factor);

        info.info = format!("({}x) {}", self.current_size(), info.info);

        info
    }
}

pub struct Inventory
{
    slots: RefCell<Vec<ItemSlot>>,
    pub max_size: usize
}

impl Inventory
{
    pub fn new(max_size: usize) -> Inventory
    {
        Inventory
        {
            slots: RefCell::new(Vec::new()),
            max_size
        }
    }

    pub fn access<T, F>(&self, callback: F) -> T
        where F: Fn(&Vec<ItemSlot>) -> T
    {
        let slots = self.slots.borrow();

        callback(&*slots)
    }

    pub fn for_each_slot<F>(&self, callback: F)
        where F: Fn(usize, &ItemSlot)
    {
        let slots = self.slots.borrow();
        let mut index = 0;

        for slot in slots.iter()
        {
            callback(index, &slot);
            index += 1;
        }
    }

    /**
     * This is probably a little confusing.
     * Consider replacing it.
     */
    pub fn for_each_item<T, F>(&self, callback: F) -> Option<T>
        where F: Fn(&Item) -> Option<T>
    {
        let slots = self.slots.borrow();

        for slot in slots.iter()
        {
            let items = slot.stack.borrow();

            for item in items.iter()
            {
                if let Some(response) = callback(&**item)
                {
                    return Some(response)
                }
            }
        }
        None
    }

    pub fn current_size(&self) -> usize
    {
        self.slots.borrow().len()
    }

    pub fn add_slot(&self, slot: ItemSlot)
    {
        self.slots.borrow_mut().push(slot);
    }

    pub fn can_hold_more(&self) -> bool
    {
        self.current_size() < self.max_size
    }

    pub fn can_add_item(&self, item: &Item) -> bool
    {
        if self.can_hold_more() { return true; }

        for slot in self.slots.borrow_mut().iter()
        {
            if slot.can_add_item(item) { return true; }
        }
        false
    }

    pub fn add_item(&self, item: Box<Item>, entity: Option<&Entity>)
    {
        item.on_get(entity);

        for slot in self.slots.borrow_mut().iter()
        {
            if slot.can_add_item(&*item)
            {
                slot.add_item(item);
                return;
            }
        }
        self.add_slot(ItemSlot::new(item));
    }

    fn get_owned_item(&self, slot_num: usize) -> (Box<Item>, usize)
    {
        let slots = self.slots.borrow();
        let slot = slots.get(slot_num)
            .expect("Invalid slot #.");

        let mut items = slot.stack.borrow_mut();
        let item = items.pop()
            .expect("Tried to pull an item from a slot which became empty.");

        (item, items.len())
    }

    pub fn take_item_id(&self, id: usize, from: Option<&Entity>) -> Option<Box<Item>>
    {
        match self.get_slot_num(id)
        {
            Some(slot_num) =>
            {
                Some(self.take_item(slot_num, from))
            },
            None => None
        }
    }

    pub fn take_item(&self, slot_num: usize, from: Option<&Entity>) -> Box<Item>
    {
        let (item, slot_size) = self.get_owned_item(slot_num);

        item.on_lose(from);

        let mut slots = self.slots.borrow_mut();

        if slot_size < 1
        {
            slots.remove(slot_num);
        }
        item
    }

    pub fn get_item_info<T, F>(&self, slot_num: usize, item_num: usize, callback: F) -> T
        where F: Fn(&Item) -> T
    {
        let slots = self.slots.borrow();
        let slot = slots.get(slot_num)
            .expect("Invalid slot #.");

        let items = slot.stack.borrow();
        let item = items.get(item_num)
            .expect("Invalid item #.");

        callback(&**item)
    }

    pub fn get_slot_info<T, F>(&self, slot_num: usize, callback: F) -> T
        where F: Fn(&mut Vec<Box<Item>>) -> T
    {
        let slots = self.slots.borrow();
        let slot = slots.get(slot_num)
            .expect("Invalid slot #.");

        let mut items = slot.stack.borrow_mut();

        callback(&mut items)
    }

    /**
     * Looks like this is unable to check beyond
     * the first item in any slot.
     */
    pub fn get_slot_num(&self, id: usize) -> Option<usize>
    {
        let slots = self.slots.borrow();

        slots.iter().position( | slot |
        {
            let items = slot.stack.borrow();
            let item = items.get(0)
                .expect("A slot existed, but there were no items in it.");

            item.get_id() == id
        })
    }

    pub fn on_use_item(&self, slot_num: usize, user: Option<&Entity>, use_on: Option<&Entity>, area: &Area)
    {
        let (num_uses, response) =
        self.get_item_info(slot_num, 0, | item |
        {
            item.decrement_uses();
            (item.get_num_uses(), item.use_item(user, use_on, area))
        });

        if let Some(usr) = user
        {
            if num_uses <= 0
            {
                self.take_item(slot_num, user);
                usr.update_health_bar();
            }
            if usr.get_type() == "player"
            {
                if let Some(ref msg) = response
                {
                    ::send_short_message(usr.get_id(), msg);
                }
            }
        }
    }

    pub fn get_item_price(&self, slot_num: usize, item_num: usize) -> u32
    {
        self.get_item_info(slot_num, item_num, | i | i.get_price())
    }

    /**
     * Returns whether the transfer was successful.
     */
    pub fn transfer(&self, from_slot: usize, other: &Inventory, from: Option<&Entity>, to: Option<&Entity>) -> bool
    {
        let can_add = self.get_item_info(from_slot, 0, | i | other.can_add_item(i));

        if can_add
        {
            let item = self.take_item(from_slot, from);
            other.add_item(item, to);
            true
        }
        else { false }
    }

    pub fn transfer_id(&self, id: usize, other: &Inventory, from: Option<&Entity>, to: Option<&Entity>) -> bool
    {
        let slot_num = match self.get_slot_num(id)
        {
            Some(num) => num,
            None => return false
        };

        self.transfer(slot_num, other, from, to)
    }

    pub fn get_display_info(&self, price_factor: f32) -> Vec<ItemDisplayInfo>
    {
        let mut info = Vec::new();

        let slots = self.slots.borrow();

        for slot in slots.iter()
        {
            info.push(slot.get_display_info(price_factor));
        }

        info
    }

    pub fn format_display_info(info: &Vec<ItemDisplayInfo>) -> String
    {
        let mut ret = String::new();
        let mut index = 0;

        for item in info
        {
            index += 1;
            ret += &format!("#{}: {}", index, item.info);
            if index != info.len() { ret += "\n"; }
        }
        ret
    }

    pub fn get_dialogue_for_player(&self, player_id: usize) -> Dialogue
    {
        var_access::access_player_meta(player_id, | player |
        {
            self._get_dialogue_for_player(player)
        })
        .expect("Player data no longer exists.")
    }

    pub fn _get_dialogue_for_player(&self, player: &mut PlayerMeta) -> Dialogue
    {
        let info = self.get_display_info(1.0);
        let mut responses = Vec::new();
        let mut commands = Vec::new();

        self.get_responses_for_player(player, &info, &mut responses);
        self.get_commands_for_player(player, &info, &mut commands);

        Dialogue::new
        (
            String::from("Inventory"),
            &Vec::new(),
            Vec::new(),
            Some(Self::format_display_info(&info)),
            responses,
            commands,
            None,
            player.player_id
        )
    }

    pub fn get_responses_for_player(&self, _player: &mut PlayerMeta, _items: &Vec<ItemDisplayInfo>, responses: &mut Vec<Response>)
    {
        responses.push(Response::text_only("Close inventory."))
    }

    pub fn get_commands_for_player(&self, player: &mut PlayerMeta, _items: &Vec<ItemDisplayInfo>, commands: &mut Vec<Command>)
    {
        let player_id = player.player_id;

        commands.push
        (
            Command
            {
                name: String::from("e"),
                input_desc: String::from("e #"),
                output_desc: String::from("Equip item #."),
                run: Box::new(| args, player |
                {
                    if args.len() < 1
                    {
                        ::add_short_message(player, "You must specify the item #.");
                        return;
                    }
                    let slot_num: usize = match args[0].parse()
                    {
                        Ok(num) => num,
                        Err(_e) =>
                        {
                            ::add_short_message(player, "Not sure what you're trying to do, there.");
                            return;
                        }
                    };

                    var_access::access_player(player, move | entity |
                    {
                        let inventory = entity.get_inventory()
                            .expect("Player does not have an inventory.");

                        if !(inventory.current_size() >= slot_num && slot_num > 0) // Potential clarity improvements needed.
                        {
                            ::add_short_message(player, "Invalid item #.");
                            return;
                        }
                        entity.equip_item(slot_num);
                    })
                    .expect("Player data no longer exists.");
                }),
                next_dialogue: ::Generate(Box::new(move ||
                {
                    var_access::access_player_context(player_id, | p, _, _, e |
                    {
                        e.get_inventory()
                            .expect("PLayer not longer has an inventory")
                            ._get_dialogue_for_player(p)
                    })
                    .expect("Player data no longer exists.")
                }))
            }
        );

        commands.push
        (
        Command
            {
                name: String::from("u"),
                input_desc: String::from("u #"),
                output_desc: String::from("Use item #."),
                run: Box::new(| args, player |
                {
                    if args.len() < 1
                    {
                        ::add_short_message(player, "You must specify the item #.");
                        return;
                    }
                    let item_num = match args[0].parse::<usize>()
                    {
                        Ok(num) => num - 1,
                        Err(_e) =>
                        {
                            ::add_short_message(player, "Not sure what you're trying to do, there.");
                            return;
                        }
                    };

                    var_access::access_player_context(player, | _, _, a, e |
                    {
                        e.get_inventory()
                            .expect("Player no longer has an inventory.")
                            .on_use_item(item_num, Some(e), None, a);
                    })
                    .expect("Player data no longer exists.");
                }),
                next_dialogue: ::Generate(Box::new(move ||
                {
                    var_access::access_player_context(player_id, | p, _, _, e |
                    {
                        e.get_inventory()
                            .expect("Player not longer has an inventory.")
                            ._get_dialogue_for_player(p)
                    })
                    .expect("Player data no longer exists.")
                }))
            }
        );
    }
}