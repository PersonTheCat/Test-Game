use crate::types::items::item_settings;
use crate::traits::{Item, Shop};
use crate::types::items::inventories::Inventory;

/// Persistent refers to the fact that
/// the same items are used on restock.
pub struct PersistentShop {
    pub inventory: Inventory,
    items: Vec<Box<Item>>,
}

impl PersistentShop {
    pub fn new(items: Vec<Box<Item>>) -> PersistentShop {
        let ret = PersistentShop {
            inventory: Inventory::new(items.len()),
            items,
        };
        ret.restock();
        ret
    }
}

impl Shop for PersistentShop {
    fn borrow_inventory(&self) -> &Inventory {
        &self.inventory
    }

    fn get_ptr(&self) -> *const Shop {
        self as *const PersistentShop
    }

    fn sell_to_rate(&self) -> f32 {
        0.0
    }

    fn buy_from_rate(&self) -> f32 {
        1.0
    }

    fn restock(&self) {
        for item in &self.items {
            self.inventory.add_item(item.clone_box(), None);
        }
    }
}

pub struct BlacksmithShop {
    pub inventory: Inventory,
    pub town_num: usize,
}

impl BlacksmithShop {
    pub fn new(town_num: usize) -> BlacksmithShop {
        let ret = BlacksmithShop {
            inventory: Inventory::new(5),
            town_num,
        };
        ret.restock();
        ret
    }
}

impl Shop for BlacksmithShop {
    fn borrow_inventory(&self) -> &Inventory {
        &self.inventory
    }

    fn get_ptr(&self) -> *const Shop {
        self as *const BlacksmithShop
    }

    fn sell_to_rate(&self) -> f32 {
        0.6
    }

    fn buy_from_rate(&self) -> f32 {
        1.0
    }

    /**
     * Will need some work when more
     * items get added.
     */
    fn restock(&self) {
        for _ in 0..self.inventory.max_size {
            self.inventory
                .add_item(item_settings::rand_weapon(None, self.town_num), None);
        }
    }
}
