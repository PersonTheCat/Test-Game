use crate::traits::{ Weapon, Item };
use crate::types::items::{ self, display_info::ItemDisplayInfo };

use std::cell::Cell;


/**
 * There's only one of each item type.
 * Use multiple constructors, instead.
 */
#[derive(Clone, ItemTools)]
pub struct Bow
{
    pub id: usize,
    pub name: String,
    pub level: u32,
    damage: Cell<u32>,
    pub piercing: u32,
    pub speed: u32,
    pub price: u32,
    num_repairs: Cell<u32>,
    num_uses: Cell<u32>,
    pub max_uses: u32
}

impl Bow
{
    pub fn new(_town_num: usize) -> Box<Item>
    {
        Box::new(Bow
        {
            id: rand::random(),
            name: String::from("to-do"),
            level: 1,
            damage: Cell::new(5),
            piercing: 0,
            speed: 15,
            price: 500,
            num_repairs: Cell::new(0),
            num_uses: Cell::new(100),
            max_uses: 100
        })
    }
}

impl Weapon for Bow
{
    fn set_damage(&self, val: u32) { self.damage.set(val); }

    fn get_damage(&self) -> u32 { self.damage.get() }

    fn get_repair_price(&self) -> u32
    {
        let base = self.get_price() / 2;

        base + ((base as f32 / 2.0).ceil() as u32 * self.num_repairs.get())
    }
}

impl Item for Bow
{
    fn get_id(&self) -> usize { self.id }

    fn get_name(&self) -> &String { &self.name }

    fn get_level(&self) -> u32 { self.level }

    fn is_weapon(&self) -> bool { true }

    fn get_price(&self) -> u32 { self.price }

    fn max_stack_size(&self) -> u32 { 1 } //Test value. Should be 1.

    fn get_type(&self) -> &'static str { "bow" }

    fn as_bow(&self) -> Option<&Bow> { Some(&self) }

    fn get_max_uses(&self) -> u32 { self.max_uses }

    fn set_num_uses(&self, val: u32) { self.num_uses.set(val); }

    fn get_num_uses(&self) -> u32 { self.num_uses.get() }

    fn get_display_info(&self, price_factor: f32) -> ItemDisplayInfo
    {
        ItemDisplayInfo
        {
            item_id: self.get_id(),
            info: format!(
                "{}\n  * Type: lvl {} {}\n  * Dps: ({} / {})\n  * Piercing: {}\n  * Uses: ({})\n  * Price: {}g",
                self.name,
                self.level,
                self.get_type(),
                self.get_damage(),
                self.speed,
                self.piercing,
                items::format_num_uses(self.num_uses.get(), self.max_uses),
                self.get_adjusted_price(price_factor),
            )
        }
    }
}