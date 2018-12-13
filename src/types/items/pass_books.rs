use crate::traits::Item;
use crate::types::items::display_info::ItemDisplayInfo;

use std::cell::RefCell;

use rand::random;

pub const MAX_NUM_PASSES: u8 = 5;

#[derive(Clone, ItemTools)]
pub struct PassBook {
    pub id: usize,
    pub passes: RefCell<Vec<TrainPass>>,
}

impl PassBook {
    pub fn new() -> PassBook {
        PassBook {
            id: random(),
            passes: RefCell::new(Vec::new()),
        }
    }

    pub fn can_hold_more(&self) -> bool {
        let passes = self.passes.borrow();

        passes.len() < MAX_NUM_PASSES as usize // Not sure why
    }

    pub fn add_pass(&self, town_num: usize, num_uses: u32) {
        let mut passes = self.passes.borrow_mut();

        passes.push(TrainPass { town_num, num_uses });
    }

    pub fn has_pass(&self, town_num: usize) -> bool {
        let passes = self.passes.borrow();

        for pass in passes.iter() {
            if pass.town_num == town_num {
                return true;
            }
        }
        false
    }

    /**
     * Decrements the number of uses for town_num
     * and removes the pass if num_uses <= 0;
     */
    pub fn use_pass(&self, town_num: usize) {
        let mut passes = self.passes.borrow_mut();
        let index = passes.iter().position(|p| p.town_num == town_num);

        match index {
            Some(num) => {
                let mut delete = false;

                if let Some(ref mut pass) = passes.get_mut(num) {
                    pass.num_uses -= 1;

                    delete = pass.num_uses <= 0
                }

                if delete {
                    passes.remove(num);
                }
            }
            None => {}
        }
    }
}

impl Item for PassBook {
    fn get_id(&self) -> usize {
        self.id
    }

    fn get_price(&self) -> u32 {
        10
    }

    fn max_stack_size(&self) -> u32 {
        1
    }

    fn get_type(&self) -> &'static str {
        "pass_book"
    }

    fn get_display_info(&self, _price_factor: f32) -> ItemDisplayInfo {
        let passes = self.passes.borrow();
        let mut info = String::new();

        info += "Travel Booklet";

        for pass in passes.iter() {
            info += &format!(
                "\n  * Town #{}; Remaining uses: {}",
                pass.town_num, pass.num_uses
            );
        }

        ItemDisplayInfo {
            item_id: self.id,
            info,
        }
    }
}

#[derive(Clone)]
pub struct TrainPass {
    pub town_num: usize,
    pub num_uses: u32,
}
