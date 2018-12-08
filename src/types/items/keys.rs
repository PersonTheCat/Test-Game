use crate::traits::{ Item };

#[derive(Clone, ItemTools)]
pub struct TownKey
{
    pub id: usize
}

impl Item for TownKey
{
    fn get_id(&self) -> usize { self.id }

    fn get_type(&self) -> &'static str { "town_key" }
}