use types::classes::Class::{ self, * };
use types::items::
{
    swords::Sword,
    bows::Bow
};
use traits::Item;

use rand::distributions::{ Weighted, WeightedChoice, Sample };
use rand::thread_rng;

type ItemRegistry = Option<Vec<ItemSettings>>;

/** Generic item constructors are registered statically */
static mut MISC_CONSUMABLES: ItemRegistry = None;
static mut POTIONS: ItemRegistry = None;
static mut FOOD: ItemRegistry = None;
static mut PASSIVES: ItemRegistry = None;
static mut WEAPONS: ItemRegistry = None;
static mut WEAPONS_UNBREAKABLE: ItemRegistry = None;

pub unsafe fn setup_item_pools()
{
    MISC_CONSUMABLES = Some(Vec::new());
    POTIONS = Some(Vec::new());
    FOOD = Some(Vec::new());
    PASSIVES = Some(Vec::new());
    WEAPONS = Some(Vec::new());
    WEAPONS_UNBREAKABLE = Some(Vec::new());
}

pub struct ItemSettings
{
    weight: u32,
    class_limits: Option<Vec<Class>>,
    constructor: fn(usize) -> Box<Item>
}

pub fn rand_consumable(class: Option<Class>, town_num: usize) -> Box<Item>
{
    unsafe { rand_item(&MISC_CONSUMABLES, class, town_num) }
}

pub fn rand_potion(class: Option<Class>, town_num: usize) -> Box<Item>
{
    unsafe { rand_item(&POTIONS, class, town_num) }
}

pub fn rand_food(class: Option<Class>, town_num: usize) -> Box<Item>
{
    unsafe { rand_item(&FOOD, class, town_num) }
}

pub fn rand_passive(class: Option<Class>, town_num: usize) -> Box<Item>
{
    unsafe { rand_item(&PASSIVES, class, town_num) }
}

pub fn rand_weapon(class: Option<Class>, town_num: usize) -> Box<Item>
{
    unsafe { rand_item(&WEAPONS, class, town_num) }
}

pub fn rand_weapon_unbreakable(class: Option<Class>, town_num: usize) -> Box<Item>
{
    unsafe { rand_item(&WEAPONS_UNBREAKABLE, class, town_num) }
}

/**
 * Should panic if no item is registered.
 */
unsafe fn rand_item(registry: &ItemRegistry, class: Option<Class>, town_num: usize) -> Box<Item>
{
    if let Some(ref registry) = registry
    {
        let mut choices = Vec::new();

        for settings in registry.iter()
            .filter(| s | is_class_allowed(class, &s.class_limits))
        {
            choices.push(Weighted { weight: settings.weight, item: settings.constructor });
        }

        let mut chooser = WeightedChoice::new(&mut choices);

        chooser.sample(&mut thread_rng())(town_num)
    }
    else { panic!("Item registry was not setup in time."); }
}

fn is_class_allowed(class: Option<Class>, limits: &Option<Vec<Class>>) -> bool
{
    let c = if let Some(clazz) = class { clazz } else { return true; };
    if let Some(ref vec) = limits { vec.contains(&c) } else { false }
}

pub fn register_consumable(item: ItemSettings)
{
    unsafe { register_item(&mut MISC_CONSUMABLES, item); }
}

pub fn register_potion(item: ItemSettings)
{
    unsafe { register_item(&mut POTIONS, item); }
}

pub fn register_food(item: ItemSettings)
{
    unsafe { register_item(&mut FOOD, item); }
}

pub fn register_passive(item: ItemSettings)
{
    unsafe { register_item(&mut PASSIVES, item); }
}

pub fn register_weapon(item: ItemSettings)
{
    unsafe { register_item(&mut WEAPONS, item); }
}

pub fn register_weapon_unbreakable(item: ItemSettings)
{
    unsafe { register_item(&mut WEAPONS_UNBREAKABLE, item); }
}

unsafe fn register_item(registry: &mut ItemRegistry, item: ItemSettings)
{
    if let Some(ref mut registry) = registry
    {
        registry.push(item);
    }
    else { panic!("Item registry was not setup in time."); }
}

pub fn register_vanilla_settings()
{
    let procedural_swords = ItemSettings
    {
        weight: 100,
        class_limits: Some(vec![Melee]),
        constructor: Sword::new
    };

    let procedural_bows = ItemSettings
    {
        weight: 100,
        class_limits: Some(vec![Ranged]),
        constructor: Bow::new
    };

    register_weapon(procedural_swords);
    register_weapon(procedural_bows);
}