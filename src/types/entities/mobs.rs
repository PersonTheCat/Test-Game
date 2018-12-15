use crate::traits::Entity;

use atomic::Ordering::*;
use atomic::Atomic;
use rand::random;
pub struct Mob {
    id: usize,
    name: String,
    health: Atomic<u32>,
    base_damage: Atomic<u32>,
}

impl Mob {
    pub fn new() -> Mob {
        Mob {
            id: random(),
            name: String::from("Ordinary Spider"),
            health: Atomic::new(5),
            base_damage: Atomic::new(5),
        }
    }
}

impl Entity for Mob {
    fn get_id(&self) -> usize {
        self.id
    }

    fn get_name(&self) -> &String {
        &self.name
    }

    fn set_health(&self, health: u32) {
        self.health.store(health, SeqCst);
    }

    fn get_health(&self) -> u32 {
        self.health.load(SeqCst)
    }

    fn kill_entity(&self) {}

    fn as_mob(&self) -> Option<&Mob> {
        Some(self)
    }

    fn get_type(&self) -> &str {
        "mob"
    }
}
