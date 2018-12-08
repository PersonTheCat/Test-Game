use crate::types::items::display_info::ItemDisplayInfo;
use crate::traits::{Item, ItemTools, Entity, Area };
use crate::types::effects::Effect;

use std::cell::Cell;
use std::any::Any;

use rand::random;

#[derive(Clone)]
pub struct Consumable
{
    pub id: usize,
    pub name: String,
    pub level: u32,
    pub effect: Effect,
    pub stack_size: u32,
    pub price: u32,
    pub num_uses: Cell<u32>
}

impl Consumable
{
    /**
     * Test consumable.
     */
    pub fn poisonous_potato() -> Consumable
    {
        Consumable
        {
            id: random(),
            name: String::from("Poisonous Potato (Test Item)"),
            level: 1,
            effect: Effect::generic_damage(5),
            stack_size: 4,
            price: 25,
            num_uses: Cell::new(0)
        }
    }
}

impl Item for Consumable
{
    fn get_id(&self) -> usize { self.id }

    fn get_name(&self) -> &String { &self.name }

    fn get_level(&self) -> u32 { self.level }

    fn get_price(&self) -> u32 { self.price }

    fn get_type(&self) -> &'static str { "consumable" }

    fn use_item(&self, user: Option<&Entity>, use_on: Option<&Entity>, _area: &Area) -> Option<String>
    {
        if let Some(entity) = use_on
        {
            self.effect.apply(entity);
            Some(format!("A {} effect was applied to {}.", self.effect.name, entity.get_name()))
        }
        else if let Some(entity) = user
        {
            self.effect.apply(entity);
            None // Some(format!("A {} effect was applied.", self.effect.name)) // Already happens if the effect is permanent.
        }
        else { None }
    }

    fn set_num_uses(&self, val: u32) { self.num_uses.set(val); }

    fn get_num_uses(&self) -> u32 { self.num_uses.get() }

    fn get_display_info(&self, price_factor: f32) -> ItemDisplayInfo
    {
        ItemDisplayInfo
        {
            item_id: self.get_id(),
            info: format!(
                "{}\n  * Type: lvl {} {}\n  * Price: {}g",
                self.get_name(),
                self.level,
                self.get_type(),
                self.get_adjusted_price(price_factor)
            )
        }
    }
}

impl ItemTools for Consumable
{
    fn clone_box(&self) -> Box<Item> { Box::new(self.clone()) }

    fn as_any(&self) -> &Any { self }
}