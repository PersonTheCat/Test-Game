use crate::types::{
    effects::Effect,
    items::{
        self,
        swords::Sword,
        bows::Bow,
        display_info::ItemDisplayInfo,
        inventories::Inventory
    },
    entities::{
        players::Player,
        mobs::Mob,
        npcs::NPC
    }
};

use crate::util::player_options::{ Dialogue, Response, Command };
use crate::util::access::{ self, EntityAccessor };
use crate::player_data::PlayerMeta;
use crate::text;
use crate::*;

use self::AttemptedPurchase::*;
use self::AttemptedSale::*;

use std::cell::Ref;
use std::any::Any;

use rand::random;

pub trait Area: EntityHolder + AreaTools
{
    fn get_formatted_title(&self) -> String
    {
        format!("Town #{}; Area #{}: {}", self.get_town_num(), self.get_area_num(), self.get_title())
    }

    fn get_type(&self) -> &'static str;

    fn get_map_icon(&self) -> &'static str;

    fn can_enter(&self, _player: &Player) -> bool { true }

    fn get_entrance_message(&self) -> Option<String> { None }

    fn get_title(&self) -> String;

    fn should_mobs_spawn(&self) -> bool { false }

    fn can_use_item(&self, _item: &Item) -> bool { true }

    fn can_leave(&self, _player: &Player) -> bool
    {
        !self.contains_mobs()
    }

    fn get_guaranteed_item(&self) -> Option<Box<Item>> { None }

    fn set_guaranteed_item(&self, _item: Box<Item>) {  }

    fn get_dialogue_info(&self, player: &mut PlayerMeta) -> Option<String>
    {
        Some(get_new_map(self.get_coordinates().0, player))
    }

    /**
     * To-do
     */
    fn fight_sequence(&self, player: &mut PlayerMeta) -> Dialogue
    {
        Dialogue::empty(player.player_id)
    }

    fn get_movements(&self, _player_id: usize, responses: &mut Vec<Response>)
    {
        let current = self.get_coordinates();
        let connections = self.get_connections();
        let num_connections = connections.len();

        for coordinates in connections
        {
            let direction = get_direction_label(current, coordinates)
                .expect("get_direction_label() did not error correctly.");

            let text = if num_connections == 1
            {
                format!("Walk away from the {}", self.get_title())
            }
            else { format!("Go {}: {}", direction, get_new_area_title(coordinates)) };

            responses.push(Response::_simple(text, move | called_player: usize |
            {
                access::area(current, | current_area |
                {
                    access::area(coordinates, | new_area |
                    {
                        current_area.transfer_to_area(called_player, new_area);
                    })
                });
            }));
        }
    }

    fn get_entity_interactions(&self, player: &mut PlayerMeta, responses: &mut Vec<Response>)
    {
        let entities = self.borrow_entities_ref();

        for entity in entities.iter()
        {
            if entity.get_id() == player.player_id { continue; }

            if let Some(text) = entity.get_response_text(player)
            {
                let accessor = entity.get_accessor();

                responses.push(Response::_get_entity_dialogue(text, accessor, player.player_id));
            }

            if entity.get_type() == "player"
            {
                let sender = player.name.clone();
                let receiver = entity.get_id();
                let wave = Response::_action_only(format!("Wave to {}.", entity.get_name()), move | p |
                {
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

                    let formatted = text::apply_replacements(msg, &[("<name>", sender.clone())]);

                    add_short_message(receiver, &formatted);

                    if !try_refresh_options(receiver)
                    {
                        send_short_message(p, *choose(&[
                            "They were too busy to notice you, but heard your message.",
                            "They didn't see you there, but got your message."
                        ]));
                    }
                    else { send_current_options(p); } // Manually trigger refresh. There is a very strange bug associated.
                });
                let trade = Response::_text_only(format!("Trade with {}", entity.get_name()));

                responses.push(wave);
                responses.push(trade);
            }
        }
    }

    fn get_specials(&self, _player: &mut PlayerMeta, _responses: &mut Vec<Response>) {}

    fn get_commands(&self, player_id: usize, commands: &mut Vec<Command>)
    {
        commands.push( Command::goto_dialogue("i", "View your inventory", move ||
        {
            access::player_context(player_id, | player, _, _, entity |
            {
                entity.get_inventory()
                    .expect("Player does not have an inventory.")
                    ._get_dialogue(player)
            })
            .expect("Player data no longer exists.")
        }));

        if access::player(player_id, | e | e.get_secondary() != "None")
            .expect("Player data no longer exists.")
        {
            commands.push( Command::simple("s", "Use your secondary item.", | _, p |
            {
                access::player(p, |e | e.use_secondary())
                    .expect("Player data no longer exists.");
            }));
        }
    }

    fn get_dialogue(&self, player_id: usize) -> Dialogue
    {
        access::player_meta(player_id, |player |
        {
            self._get_dialogue(player)
        })
        .expect("Player data no longer exists.")
    }

    fn _get_dialogue(&self, player: &mut PlayerMeta) -> Dialogue
    {
        if self.contains_mobs() { return self.fight_sequence(player); }

        let mut responses = Vec::new();
        let mut commands = Vec::new();

        self.get_movements(player.player_id, &mut responses);
        self.get_specials(player, &mut responses);
        self.get_entity_interactions(player, &mut responses);
        self.get_commands(player.player_id, &mut commands);

        let coordinates = self.get_coordinates();

        let entrance_message = if !player.player_has_visited(coordinates)
        {
            //To-do: find a better place for this.
            player.add_record_book(coordinates);
            self.get_entrance_message()
        }
        else { None };

        Dialogue
        {
            title: self.get_formatted_title(),
            text: entrance_message,
            info: self.get_dialogue_info(player),
            responses,
            commands,
            text_handler: None,
            player_id: player.player_id,
            id: random()
        }
    }
}

/**
 * To-do: Work on all of these a bit.
 */

fn get_new_area_title(coords: (usize, usize, usize)) -> String
{
    match access::area(coords, |a | a.get_title())
    {
        Some(title) => title,
        None => String::new()
    }
}

fn get_new_map(town_num: usize, player: &mut PlayerMeta) -> String
{
    access::town(town_num, | t | t._get_map_for_player(player))
}

/**
 * To-do: Possibly just use "next" / "previous."
 * Would have to add on world gen. This is
 * mildly bad.
 */
fn get_direction_label(from: (usize, usize, usize), to: (usize, usize, usize)) -> Option<&'static str>
{
    if to.2 == from.2
    {
        if to.1 == from.1 + 1
        {
            return Some("forward");
        }
        else if to.1 == from.1 - 1
        {
            return Some("backward");
        }
        panic!("Error: Indirect connections are not yet implemented. Tried to skip a z coordinate.");
    }
    else if to.1 == from.1
    {
        if to.2 == from.2 + 1
        {
            return Some("right");
        }
        else if to.2 == from.2 - 1
        {
            return Some("left");
        }
        panic!("Error: Indirect connections are not yet implemented. Tried to skip an x coordinate.");
    }
    panic!("Error: Indirect connections are not yet implemented. Tried to move diagonally.");
}

/**
 * These rarely change and thus can be derived.
 */
pub trait AreaTools
{
    fn get_area_num(&self) -> usize;

    fn get_town_num(&self) -> usize;

    fn get_coordinates(&self) -> (usize, usize, usize);

    fn add_connection(&self, connection: (usize, usize, usize));

    fn get_connections(&self) -> Vec<(usize, usize, usize)>;

    fn as_entity_holder(&self) -> &EntityHolder;

    fn as_any(&self) -> &Any;
}

pub trait EntityHolder
{
    fn contains_type(&self, typ: &'static str) -> bool;

    fn add_entity(&self, entity: Box<Entity>);

    fn remove_entity(&self, id: usize) -> Option<Box<Entity>>;

    fn transfer_entity(&self, id: usize, to: &EntityHolder);

    fn contains_entity(&self, id: usize) -> bool;

    fn get_entity_index(&self, id: usize) -> Option<usize>;

    fn take_entity_by_index(&self, index: usize) -> Box<Entity>;

    fn borrow_entities_ref(&self) -> Ref<Vec<Box<Entity>>>;

    fn contains_mobs(&self) -> bool
    {
        self.contains_type("mob")
    }

    fn contains_players(&self) -> bool
    {
        self.contains_type("player")
    }

    fn contains_npcs(&self) -> bool
    {
        self.contains_type("npc")
    }

    // This should look nicer in-use.
    fn transfer_to_area(&self, id: usize, area: &Area)
    {
        self.transfer_entity(id, area.as_entity_holder());
    }
}

/**
 * Generic speed caps.
 */
pub const ATTACK_SPEED_MIN: i32 = -5000;
pub const ITEM_SPEED_MIN: i32 = -8000;

pub trait Entity
{
    fn get_id(&self) -> usize;

    fn get_name(&self) -> &String;

    fn get_title(&self) -> Option<&String> { None }

    fn get_description(&self) -> Option<&String> { None }

    fn set_max_health(&self, _val: u32) {}

    fn get_max_health(&self) -> u32 { 15 }

    fn set_health(&self, health: u32);

    fn get_health(&self) -> u32;

    fn get_health_bar(&self) -> String
    {
        format!
        (
            "HP: ({} / {}); Dps: ({}); Gold: {}g\n\
            Prim: {}; Sec: {}",
            self.get_health(), self.get_max_health(), items::format_damage_2(self.get_base_damage(), self.get_attack_speed()), self.get_money(),
            self.get_primary(), self.get_secondary()
        )
    }

    fn update_health_bar(&self) {}

    fn add_health(&self, health: i32)
    {
        let prior = self.get_health();

        self.set_health((prior as i32 + health) as u32);

        let adjusted = self.get_health();

        let max = self.get_max_health();

        if adjusted > max
        {
            self.set_health(max);
        }
    }

    fn remove_health(&self, health: u32)
    {
        let prior = self.get_health();

        self.set_health(prior - health);

        if self.get_health() == 0
        {
            self.kill_entity()
        }
    }

    fn set_base_damage(&self, _val: u32) {}

    fn get_base_damage(&self) -> u32 { 5 }

    fn set_attack_speed(&self, _val: i32) {}

    fn add_attack_speed(&self, val: i32)
    {
        let current = self.get_attack_speed();
        let new = current + val;
        if new < ATTACK_SPEED_MIN
        {
            self.set_attack_speed(ATTACK_SPEED_MIN);
        }
        else { self.set_attack_speed(new); }
    }

    fn get_attack_speed(&self) -> i32 { 0 }

    fn set_item_speed(&self, _val: i32) {}

    fn add_item_speed(&self, val: i32)
    {
        let current = self.get_item_speed();
        let new = current + val;
        if new < ITEM_SPEED_MIN
        {
            self.set_item_speed(ITEM_SPEED_MIN);
        }
        else { self.set_item_speed(new); }
    }

    fn get_item_speed(&self) -> i32 { 0 }

    fn get_inventory(&self) -> Option<&Inventory> { None }

    fn get_response_text(&self, _player: &mut PlayerMeta) -> Option<String> { None }

    fn get_dialogue(&self, player_id: usize) -> Option<Dialogue>
    {
        access::player_meta(player_id, | player |
        {
            self._get_dialogue(player)
        })
        .expect("Player data no longer exists.")
    }

    /**
     * These duplicates were added because access to the player's
     * EntityKnowledge will generally be necessary. It doesn't make
     * much sense to provide direct access to that instead, however,
     * because player metadata is also generally needed and there
     * can only be one mutable reference to _player at a time.
     */
    fn _get_dialogue(&self, _player: &mut PlayerMeta) -> Option<Dialogue> { None }

    fn goto_dialogue(&self, marker: u8, player_id: usize) -> Option<Dialogue>
    {
        access::player_meta(player_id, | player |
        {
            self._goto_dialogue(marker, player)
        })
        .expect("Player data no longer exists.")
    }

    fn _goto_dialogue(&self, _marker: u8, _player: &mut PlayerMeta) -> Option<Dialogue> { None }

    fn get_trades(&self, _player_id: usize, _trades: &mut Vec<Response>) {}

    fn give_item(&self, _item: Box<Item>) {}

    fn take_item_id(&self, _id: usize) -> Option<Box<Item>> { None }

    fn equip_item(&self, _slot_num: usize) {}

    fn unequip_item(&self, _id: usize) {}

    fn use_item(&self, _item_num: usize, _use_on: Option<&Entity>) {}

    fn use_primary(&self) {}

    fn use_secondary(&self) {}

    fn get_primary(&self) -> String { String::from("None") }

    fn get_secondary(&self) -> String { String::from("None") }

    fn give_money(&self, _amount: u32) {}

    fn take_money(&self, _amount: u32) {}

    fn get_money(&self) -> u32 { 0 }

    fn can_afford(&self, amount: u32) -> bool { self.get_money() >= amount }

    fn has_effect(&self, _name: &str) -> bool { false }

    fn give_effect(&self, _effect: Effect) {}

    fn apply_effect(&self, _name: &str) {}

    fn remove_effect(&self, _name: &str) {}

    fn clear_effects(&self) {}

    fn kill_entity(&self);

    fn as_player(&self) -> Option<&Player> { None }

    fn as_mob(&self) -> Option<&Mob> { None }

    fn as_npc(&self) -> Option<&NPC> { None }

    fn set_coordinates(&self, _coords: (usize, usize, usize)) {}

    fn get_coordinates(&self) -> (usize, usize, usize) { (0, 0, 0) }

    fn on_enter_area(&self, _coords: (usize, usize, usize)) {}

    fn get_type(&self) -> &str;

    fn get_accessor(&self) -> EntityAccessor
    {
        EntityAccessor
        {
            coordinates: self.get_coordinates(),
            entity_id: self.get_id(),
            is_player: self.get_type() == "player"
        }
    }
}

lazy_static!
{
    static ref NO_NAME: String = String::from("");
}

pub trait Item: ItemTools
{
    fn get_id(&self) -> usize;

    fn get_name(&self) -> &String { &NO_NAME }

    fn get_level(&self) -> u32 { 0 }

    fn is_tradable(&self) -> bool { true }

    fn is_weapon(&self) -> bool { false }

    fn as_weapon(&self) -> Option<&Weapon> { None }

    fn get_price(&self) -> u32 { 10 }

    fn get_adjusted_price(&self, factor: f32) -> u32
    {
        (self.get_price() as f32 * factor) as u32
    }

    fn max_stack_size(&self) -> u32 { 4 }

    fn get_type(&self) -> &'static str;

    fn as_sword(&self) -> Option<&Sword> { None }

    fn as_bow(&self) -> Option<&Bow> { None }

    fn has_entity_effect(&self) -> bool { false }

    fn can_use_item(&self, _area: &Area) -> bool { true }

    /** Returning as string is unnecessary. Remove? */
    fn use_item(&self, _user: Option<&Entity>, _use_on: Option<&Entity>, _area: &Area) -> Option<String> { None }

    fn on_get(&self, _entity: Option<&Entity>) {}

    fn on_lose(&self, _entity: Option<&Entity>) {}

    fn on_equip(&self, _entity: &Entity) {}

    fn on_unequip(&self, _entity: &Entity) {}

    fn get_max_uses(&self) -> u32 { 1 }

    fn set_num_uses(&self, _val: u32) {}

    fn decrement_uses(&self) { self.set_num_uses(self.get_num_uses().checked_sub(1).unwrap_or(0)); }

    fn get_num_uses(&self) -> u32 { 1 }

    fn get_display_info(&self, price_factor: f32) -> ItemDisplayInfo
    {
        ItemDisplayInfo
        {
            item_id: self.get_id(),
            info: format!(
                "{}\n  * Type: {}\n  * Price: {}g",
                self.get_name(),
                self.get_type(),
                self.get_adjusted_price(price_factor)
            )
        }
    }
}

pub trait ItemTools: Any
{
    fn clone_box(&self) -> Box<Item>;

    fn as_any(&self) -> &Any;
}

pub trait Weapon: Item
{
    fn set_damage(&self, _val: u32) {}

    fn get_damage(&self) -> u32 { 5 }

    fn get_repair_price(&self) -> u32 { self.get_price() / 2 }
}

pub enum AttemptedSale
{
    StoreFull(Box<Item>),
    Sale(usize)
}

pub enum AttemptedPurchase
{
    NotFound,
    CantAfford,
    CantHold,
    Purchase
}

/**
 * These are not stored as consistently as the other types,
 * and thus currently require use of raw pointers.
 */
pub trait Shop
{
    fn borrow_inventory(&self) -> &Inventory;
    fn get_ptr(&self) -> *const Shop;

    fn sell(&self, item: Box<Item>) -> AttemptedSale
    {
        let inventory = self.borrow_inventory();

        if inventory.can_add_item(&*item)
        {
            let payback = item.get_price() as f32 * self.sell_to_rate();
            inventory.add_item(item, None);
            Sale(payback as usize)
        }
        else { StoreFull(item) }
    }

    fn sell_to_rate(&self) -> f32;
    fn buy_from_rate(&self) -> f32;

    fn buy(&self, player_id: usize, item_id: usize, price_factor: f32) -> AttemptedPurchase
    {
        let inventory = self.borrow_inventory();
        let slot_num = inventory.get_slot_num(item_id);

        if let None = slot_num { return NotFound; }

        let slot_num = slot_num.unwrap();

        let (price, can_afford, can_hold) =
        inventory.get_item_info(slot_num, 0, | item |
        {
            access::player_meta(player_id, move | meta |
            {
                access::entity(meta.get_accessor(), | player |
                {
                    let price = item.get_adjusted_price(price_factor);
                    (price, player.can_afford(price), player.get_inventory().unwrap().can_add_item(item))
                })
                .expect("Area no longer contains entity.")
            })
            .expect("Player data no longer exists.")
        });

        if !can_afford { CantAfford }
        else if !can_hold { CantHold }
        else
        {
            // Placement avoids borrow errors with item use.
            access::player(player_id, | entity |
            {
                entity.give_item(inventory.take_item(slot_num, None));
                entity.take_money(price);
            });

            if self.should_restock() { self.restock(); }

            Purchase
        }
    }

    fn should_restock(&self) -> bool
    {
        self.borrow_inventory().current_size() == 0
    }

    fn restock(&self);

    fn get_dialogue(&self, player: &mut PlayerMeta, allow_sales: bool, price_factor: f32) -> Dialogue
    {
        let inventory: &Inventory = self.borrow_inventory();

        let info = inventory.get_display_info(price_factor);
        let mut responses = Vec::new();
        let mut commands = Vec::new();

        self.get_responses(player, &info, allow_sales, &mut responses);
        self.get_commands(player, &info, allow_sales, price_factor, &mut commands);

        Dialogue::new
        (
            String::from("Trades"),
            &Vec::new(),
            Vec::new(),
            Some(Inventory::format_display_info(&info)),
            responses,
            commands,
            None,
            player.player_id
        )
    }

    fn get_responses(&self, _player: &mut PlayerMeta, _items: &Vec<ItemDisplayInfo>, _allow_sales: bool, responses: &mut Vec<Response>)
    {
        responses.push(Response::text_only("Leave."));
    }

    fn get_commands(&self, player: &mut PlayerMeta, items: &Vec<ItemDisplayInfo>, allow_sales: bool, price_factor: f32, commands: &mut Vec<Command>)
    {
        let mut item_ids = Vec::new();
        items.iter().for_each(| i |{ item_ids.push(i.item_id) });

        commands.push(Command
        {
            name: String::from("buy"),
            input_desc: String::from("buy #"),
            output_desc: String::from("Buy item #."),
            run: self.process_buy(item_ids, price_factor),
            next_dialogue: Generate(self.refresh_dialogue(player.player_id, allow_sales, price_factor))
        });

        if allow_sales
        {
            commands.push(Command::manual_desc(
            "sell", "sell #", "Sell item # from inventory.",
            | _args, player_id |
            {
                send_short_message(player_id, "Let's just pretend you sold that. ;)");
            }));
        }
    }

    /**
     * Stylistic improvements needed for the dialogue.
     */
    fn process_buy(&self, item_ids: Vec<usize>, price_factor: f32) -> Box<Fn(&Vec<&str>, usize)>
    {
        let ptr = self.get_ptr();

        Box::new(move | args: &Vec<&str>, player_id: usize |
        {
            if args.len() == 0 { return; }
            if item_ids.len() == 0
            {
                send_short_message(player_id, "There are no items to buy.");
                return;
            }
            let shop = unsafe { match ptr.as_ref()
            {
                Some(s) => s,
                None =>
                {
                    add_short_message(player_id, "The shop seems to have moved away.");
                    return;
                }
            }};
            let item_num: usize = match args[0].parse()
            {
                Ok(num) => num,
                Err(_) =>
                {
                    add_short_message(player_id, "Not sure which item you're looking for.");
                    return;
                }
            };
            if item_ids.len() < item_num || item_num < 1
            {
                add_short_message(player_id, "I'm afraid I can't tell what you're looking for.");
                return;
            }

            let item_id: usize = item_ids[item_num - 1];

            match shop.buy(player_id, item_id, price_factor)
            {
                NotFound => { add_short_message(player_id, "Looks like someone already bought that item."); }
                CantAfford => { add_short_message(player_id, "You can't afford that."); },
                CantHold => { add_short_message(player_id, "You don't have enough room."); },
                Purchase => { add_short_message(player_id, "Purchase successful."); }
            };
        })
    }

    fn refresh_dialogue(&self, player_id: usize, allow_sales: bool, price_factor: f32) -> Box<Fn() -> Dialogue>
    {
        let ptr = self.get_ptr();

        Box::new(move ||
        {
            access::player_context(player_id, move | player, _, area, _ |
            {
                unsafe { match ptr.as_ref()
                {
                    Some(ref shop) => shop.get_dialogue(player, allow_sales, price_factor),
                    None => area._get_dialogue(player)
                }}
            })
            .expect("Player somehow moved.")
        })
    }
}