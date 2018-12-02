use types::items::{ self, display_info::ItemDisplayInfo };
use types::effects::{ Effect, EffectType::* };
use traits::{ Weapon, Item, Entity, Area };

use std::cell::{ Cell, RefCell };

use rand::{ random, thread_rng, Rng };

/**
 * The only way to ensure that hold_effect always stays
 * accurate is to let entities use attribute modifiers
 * instead of directly manipulating their stats.
 * That would require a bit too much effort to change
 * for this project.
 */

#[derive(Clone, ItemTools)]
pub struct Sword
{
    pub id: usize,
    pub name: String,
    pub level: u32,
    damage: Cell<u32>,
    sharpness: Cell<i32>,
    pub max_sharpness: i32,
    pub speed: u32,
    pub price: u32,
    num_repairs: Cell<u32>,
    num_uses: Cell<u32>,
    pub max_uses: u32,
    pub hold_effect: RefCell<Option<Effect>>,
    pub use_effect: RefCell<Option<Effect>>
}

const DAMAGE_PER_LEVEL: f32 = 4.5;
const SHARPNESS_PER_LEVEL: f32 = 3.2;
const USE_EFFECT_CHANCE: f32 = 3.75;// Per-level
const HOLD_EFFECT_CHANCE: f32 = 3.75;// Per-level
const USES_PER_LEVEL: u32 = 85;
const SPEED_PER_LEVEL: i32 = -250;
const BASE_SPEED: u32 = 9_000;
const MIN_SPEED: u32 = 2_000;
const DULL_CHANCE: f32 = 0.15;

impl Sword
{
    /**
     * +1 level per 2 * town_num
     * +1 variability per 3 * town_num
     */
    pub fn new(town_num: usize) -> Box<Item>
    {
        let base_level = (town_num / 2) + 1; // Start at level = 1
        let variability = town_num / 3; // Start at variability = 0;

        let level = thread_rng().gen_range(base_level - variability, base_level + variability + 1);

        Self::from_level(level as u32)
    }

    pub fn from_level(level: u32) -> Box<Item>
    {
        let max_sharpness = calc_sharpness(level);
        let damage = calc_damage(level);
        let sharpness = thread_rng().gen_range(0, max_sharpness);
        let speed = calc_speed(level);
        let num_uses = calc_uses(level);
        let hold_effect = calc_hold_effect(level);
        let use_effect = calc_use_effect(level);
        let price = calc_price(damage, sharpness, use_effect.is_some(), hold_effect.is_some(), num_uses, speed);

        Box::new(Sword
        {
            id: random(),
            name: String::from("to-do"),
            level,
            damage: Cell::new(damage),
            sharpness: Cell::new(sharpness),
            max_sharpness,
            speed,
            price,
            num_repairs: Cell::new(0),
            num_uses: Cell::new(num_uses),
            max_uses: num_uses,
            hold_effect: RefCell::new(hold_effect),
            use_effect: RefCell::new(use_effect)
        })
    }

    pub fn get_sharpness(&self) -> i32
    {
        self.sharpness.get()
    }

    pub fn set_sharpness(&self, val: i32)
    {
        self.sharpness.set(val);
    }

    pub fn get_min_sharpness(&self) -> i32
    {
        -1 * self.max_sharpness // Assume this to be positive, for now.
    }
}

fn calc_damage(level: u32) -> u32
{
    (DAMAGE_PER_LEVEL * level as f32) as u32
}

fn calc_sharpness(level: u32) -> i32
{
    (SHARPNESS_PER_LEVEL * level as f32) as i32
}

fn calc_use_effect(level: u32) -> Option<Effect>
{
    if random::<f32>() <= (USE_EFFECT_CHANCE * level as f32)
    {
        None // to-do
    }
    else { None }
}

fn calc_hold_effect(level: u32) -> Option<Effect>
{
    if random::<f32>() <= (HOLD_EFFECT_CHANCE * level as f32)
    {
        None // to-do
    }
    else { None }
}

/**
 * +100 per 2 * level
 * +20 variability per 3 * level
 */
fn calc_uses(level: u32) -> u32
{
    let base_level = ((level / 2) + 1) * 50; // Start at level = 1
    let variability = (level / 3) * 10; // Start at variability = 0;

    thread_rng().gen_range(base_level - variability, base_level + variability + 1)
}

fn calc_speed(level: u32) -> u32
{
    let mut speed = (BASE_SPEED as i32 + (SPEED_PER_LEVEL * level as i32)) as u32;
    if speed < MIN_SPEED { speed = MIN_SPEED };
    speed
}

fn calc_price(damage: u32, sharpness: i32, has_use: bool, has_hold: bool, max_uses: u32, speed: u32) -> u32
{
    let mut price = 0;

    price += damage;
    price += sharpness as u32;
    if has_use { price += 100; }
    if has_hold { price += 100; }
    price += max_uses;
    price += (BASE_SPEED - speed) / 100;

    price
}

impl Weapon for Sword
{
    fn set_damage(&self, val: u32) { self.damage.set(val); }

    fn get_damage(&self) -> u32 { (self.damage.get() as i32 + self.get_sharpness()) as u32 }

    fn get_repair_price(&self) -> u32
    {
        let base = self.get_price() / 2;

        base + ((base as f32 / 2.0).ceil() as u32 * self.num_repairs.get())
    }
}

impl Item for Sword
{
    fn get_id(&self) -> usize { self.id }

    fn get_name(&self) -> &String { &self.name }

    fn get_level(&self) -> u32 { self.level }

    fn is_weapon(&self) -> bool { true }

    fn as_weapon(&self) -> Option<&Weapon> { Some(self) }

    fn get_price(&self) -> u32 { self.price }

    fn max_stack_size(&self) -> u32 { 1 }

    fn get_type(&self) -> &'static str { "sword" }

    fn as_sword(&self) -> Option<&Sword> { Some(self) }

    fn has_entity_effect(&self) -> bool { self.hold_effect.borrow().is_some() }

    /**
     * To-do: Possibly allow weapons to apply
     * effects to the user on use.
     */
    fn use_item(&self, _user: Option<&Entity>, use_on: Option<&Entity>, _area: &Area) -> Option<String>
    {
        if let Some(entity) = use_on
        {
            entity.add_health(-1 * self.get_damage() as i32);

            if let Some(ref effect) = *self.use_effect.borrow()
            {
                effect.apply(entity);
            }
            None
        }
        else { Some(String::from("This item has no effect here.")) } // Bug: Num uses will still decrement.
    }

    fn on_equip(&self, entity: &Entity)
    {
        if let Some(ref effect) = *self.hold_effect.borrow()
        {
            effect.apply(entity);
        }
    }

    fn on_unequip(&self, entity: &Entity)
    {
        let effect_cell = self.hold_effect.borrow();

        let effect = match *effect_cell
        {
            Some(ref e) => e,
            None => return
        };

        if let Permanent = effect.effect_type // See notes about accuracy.
        {
            effect.get_opposite_effect().apply(entity);
        }
    }

    fn get_max_uses(&self) -> u32 { self.max_uses }

    fn set_num_uses(&self, val: u32) { self.num_uses.set(val); }

    fn decrement_uses(&self)
    {
        if random::<f32>() <= DULL_CHANCE
        {
            self.set_sharpness(self.get_sharpness() - 1);
            if self.get_sharpness() < self.get_min_sharpness()
            {
                self.set_sharpness(self.get_min_sharpness());
            }
        }
        self.set_num_uses(self.get_num_uses().checked_sub(1).unwrap_or(0));
    }

    fn get_num_uses(&self) -> u32 { self.num_uses.get() }

    fn get_display_info(&self, price_factor: f32) -> ItemDisplayInfo
    {
        let mut info = format!
        (
            "{}\n  * Type: lvl {} {}\n  * Dps: ({})\n  * Sharpness: ({} / {})\n  * Uses: ({})\n  * Price: {}g",
            self.name,
            self.level,
            self.get_type(),
            items::format_damage(self.get_damage(), self.speed),
            self.sharpness.get(),
            self.max_sharpness,
            items::format_num_uses(self.num_uses.get(), self.max_uses),
            self.get_adjusted_price(price_factor),
        );

        if let Some(ref effect) = *self.hold_effect.borrow()
        {
            info += &format!("\n  * When equipped: {}", effect.name);
        }
        if let Some(ref effect) = *self.use_effect.borrow()
        {
            info += &format!("\n  * Attack effect: {}", effect.name);
        }

        ItemDisplayInfo
        {
            item_id: self.get_id(),
            info
        }
    }
}