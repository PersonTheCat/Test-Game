use crate::traits::Item;
use crate::types::classes::Class::{self, *};
use crate::types::items::{bows::Bow, swords::Sword};

use lazy_static::lazy_static;
use parking_lot::Mutex;
use rand::distributions::{Sample, Weighted, WeightedChoice};
use rand::thread_rng;

type ItemRegistry = Vec<ItemSettings>;

lazy_static! {
    /** Generic item constructors are registered statically */
    static ref ITEM_POOLS: Mutex<ItemPools> = Mutex::new(init_item_pools());
}

struct ItemPools {
    misc_consumables: ItemRegistry,
    potions: ItemRegistry,
    food: ItemRegistry,
    passives: ItemRegistry,
    weapons: ItemRegistry,
    weapons_unbreakable: ItemRegistry,
}

fn init_item_pools() -> ItemPools {
    ItemPools {
        misc_consumables: Vec::new(),
        potions: Vec::new(),
        food: Vec::new(),
        passives: Vec::new(),
        weapons: Vec::new(),
        weapons_unbreakable: Vec::new(),
    }
}

pub fn setup_item_pools() {}

pub struct ItemSettings {
    weight: u32,
    class_limits: Option<Vec<Class>>,
    constructor: fn(usize) -> Box<Item>,
}

pub fn rand_consumable(class: Option<Class>, town_num: usize) -> Box<Item> {
    rand_item(&ITEM_POOLS.lock().misc_consumables, class, town_num)
}

pub fn rand_potion(class: Option<Class>, town_num: usize) -> Box<Item> {
    rand_item(&ITEM_POOLS.lock().potions, class, town_num)
}

pub fn rand_food(class: Option<Class>, town_num: usize) -> Box<Item> {
    rand_item(&ITEM_POOLS.lock().food, class, town_num)
}

pub fn rand_passive(class: Option<Class>, town_num: usize) -> Box<Item> {
    rand_item(&ITEM_POOLS.lock().passives, class, town_num)
}

pub fn rand_weapon(class: Option<Class>, town_num: usize) -> Box<Item> {
    rand_item(&ITEM_POOLS.lock().weapons, class, town_num)
}

pub fn rand_weapon_unbreakable(class: Option<Class>, town_num: usize) -> Box<Item> {
    rand_item(&ITEM_POOLS.lock().weapons_unbreakable, class, town_num)
}

/**
 * Should panic if no item is registered.
 */
fn rand_item(registry: &ItemRegistry, class: Option<Class>, town_num: usize) -> Box<Item> {
    let mut choices: Vec<Weighted<fn(usize) -> Box<Item>>> = registry
        .iter()
        .filter(|s| is_class_allowed(class, &s.class_limits))
        .map(|s| Weighted {
            weight: s.weight,
            item: s.constructor,
        })
        .collect();

    WeightedChoice::new(&mut choices).sample(&mut thread_rng())(town_num)
}

fn is_class_allowed(class: Option<Class>, limits: &Option<Vec<Class>>) -> bool {
    let c = if let Some(clazz) = class {
        clazz
    } else {
        return true;
    };
    if let Some(ref vec) = limits {
        vec.contains(&c)
    } else {
        false
    }
}

pub fn register_consumable(item: ItemSettings) {
    ITEM_POOLS.lock().misc_consumables.push(item);
}

pub fn register_potion(item: ItemSettings) {
    ITEM_POOLS.lock().potions.push(item);
}

pub fn register_food(item: ItemSettings) {
    ITEM_POOLS.lock().food.push(item);
}

pub fn register_passive(item: ItemSettings) {
    ITEM_POOLS.lock().passives.push(item);
}

pub fn register_weapon(item: ItemSettings) {
    ITEM_POOLS.lock().weapons.push(item);
}

pub fn register_weapon_unbreakable(item: ItemSettings) {
    ITEM_POOLS.lock().weapons_unbreakable.push(item);
}

pub fn register_vanilla_settings() {
    let procedural_swords = ItemSettings {
        weight: 100,
        class_limits: Some(vec![Melee]),
        constructor: Sword::new,
    };

    let procedural_bows = ItemSettings {
        weight: 100,
        class_limits: Some(vec![Ranged]),
        constructor: Bow::new,
    };

    register_weapon(procedural_swords);
    register_weapon(procedural_bows);
}
