use crate::traits::Area;
use crate::types::classes::Class;

use crate::types::areas::{
    altars::Altar, bosses::BossRoom, dungeons::Dungeon, fountains::Fountain,
    gambling_den::GamblingDen, gates::Gate, shop_areas::Pub, stations::Station,
};

// Center(deep), Depth
use crate::types::towns::{CD, D};

use self::PathPreference::*;

use lazy_static::lazy_static;
use parking_lot::Mutex;

lazy_static! {
    /** Area constructors are registered statically */
    pub static ref AREA_REGISTRY: Mutex<Vec<AreaSettings>> = Mutex::new(Vec::new());
}

pub fn setup_area_registry() {}

pub fn register(settings: AreaSettings) {
    AREA_REGISTRY.lock().push(settings);
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum PathPreference {
    OnPath,
    OffPath,
}

pub struct AreaSettings {
    pub min_x: usize,
    pub max_x: usize,
    pub chance: f32,
    pub class_limits: Option<Vec<Class>>,
    pub path_pref: PathPreference,
    pub constructor: fn(Class, usize, (usize, usize, usize)) -> Box<Area>,
}

pub fn register_vanilla_settings() {
    let gate = AreaSettings {
        min_x: D - 1, // Last area only.
        max_x: D - 1,
        chance: 1.0,
        class_limits: None,
        path_pref: OnPath,
        constructor: Gate::new,
    };
    let altar = AreaSettings {
        min_x: CD + 3, // Second half. Close to end.
        max_x: D - 2,
        chance: 1.0,
        class_limits: None,
        path_pref: OffPath,
        constructor: Altar::new,
    };
    let boss_room = AreaSettings {
        min_x: CD + 1, // Second half.
        max_x: D - 2,
        chance: 1.0,
        class_limits: None,
        path_pref: OnPath,
        constructor: BossRoom::new,
    };
    let dungeon = AreaSettings {
        min_x: 1, // Anywhere.
        max_x: D - 2,
        chance: 1.0,
        class_limits: None,
        path_pref: OffPath,
        constructor: Dungeon::new,
    };
    let fountain = AreaSettings {
        min_x: CD - 1, // Close to center.
        max_x: CD + 1,
        chance: 0.75,
        class_limits: None,
        path_pref: OnPath,
        constructor: Fountain::new,
    };
    let shops = AreaSettings {
        min_x: 1, // Anywhere.
        max_x: D - 2,
        chance: 1.0,
        class_limits: None,
        path_pref: OffPath,
        constructor: Pub::new, // Only one shop, for now.
    };
    let station = AreaSettings {
        min_x: 1, // First half. Close to beginning.
        max_x: CD - 2,
        chance: 1.0,
        class_limits: None,
        path_pref: OffPath,
        constructor: Station::new,
    };
    let gambling_den = AreaSettings {
        min_x: 3, // Away from edges.
        max_x: D - 3,
        chance: 0.35,
        class_limits: None,
        path_pref: OffPath,
        constructor: GamblingDen::new,
    };

    register(gate);
    register(altar);
    register(boss_room);
    register(dungeon);
    register(fountain);
    register(shops);
    register(station);
    register(gambling_den)
}
