use crate::player_data::{PlayerMeta, PLAYER_META};
use crate::traits::{Area, Entity};
use crate::types::towns::{self, Town};
use crate::*;

use std::sync::Arc;

///   These are a bunch of functions I use for accessing
/// variables statically provided information about them.
/// It's relatively inefficient, and while sometimes it
/// can result in fairly pretty code, it also quickly
/// increases the number of lines in a way that can be
/// difficult to parse when used too often.
///   This solution is in many ways inferior to using
/// raw pointers. It is not a complete substitute (see
/// shops and trades) and of course it is much slower
/// as well as more difficult to use. As such, I may
/// wind up accepting defeat on living without normal
/// pointers and someday make the switch back.
///   ...Or not. It's just a text-based game!

#[derive(Copy, Clone)]
pub struct EntityAccessor {
    pub coordinates: (usize, usize, usize),
    pub entity_id: usize,
    pub is_player: bool,
}

/// Entities are not reference counted, and thus references
/// to them cannot be extracted from areas. One way to
/// ensure that these references are valid is to make sure
/// that all pointers to them stay in scope. This is why
/// callbacks are needed for this function; however, it's
/// possible that this will change in the future.
pub fn entity<T, F>(mut accessor: EntityAccessor, callback: F) -> Option<T>
    where F: FnOnce(&Entity) -> T
{
    // Refresh the accessor for players. Other entities won't move,
    // but should probably also be converted to reference counters
    // at some point in the future, as well.
    if accessor.is_player {
        accessor = player_meta(accessor.entity_id).get_accessor();
    }

    area(accessor.coordinates, |area| {
        area.borrow_entity_lock().iter()
            .find(|e| e.get_id() == accessor.entity_id)
            .and_then(|e| Some(callback(&**e)))
    })
    .expect("Area no longer exists.")
}

/// Clones a reference to this player's information from
/// the registry using their ID.
pub fn player_meta(player_id: usize) -> Arc<PlayerMeta> {
    PLAYER_META.lock()
        .iter()
        .find(|p| p.get_player_id() == player_id)
        .expect("Called tried access a player who was not registered.")
        .clone()
}

/// Retrieves information about the user associated with
/// this channel information, i.e. Discord channel,
/// local username, etc.
pub fn player_meta_sender(channel: &ChannelInfo) -> Option<Arc<PlayerMeta>> {
    PLAYER_META.lock()
        .iter()
        .find(|p| p.get_channel() == *channel)
        .and_then(|p| Some(p.clone()))
}

/// Retrieves information related to the specified player's
/// context, including their current town, area, and actual
/// entity.
pub fn context<T, F>(player: &PlayerMeta, callback: F) -> Option<T>
    where F: FnOnce(&Town, &Area, &Entity) -> T
{
    let coordinates = player.get_coordinates();
    let town = town(coordinates.0);
    let area = match &town.get_areas()[coordinates.1][coordinates.2] {
        Some(ref a) => a,
        None => return None,
    };

    let entities = area.borrow_entity_lock();

    let entity = entities
        .iter()
        .find(|e| e.get_id() == player.get_player_id())
        .expect("Area no longer contains entity.");

    Some(callback(&*town, &**area, &**entity))
}

/// Clones a reference to the specified town from the registry.
/// Generates towns that do not exist. As such, there is no
/// need to generate these manually.
pub fn town(num: usize) -> Arc<Town> {
    if let Some(t) = towns::TOWN_REGISTRY.read().get(&num) {
        return t.clone();
    }
    // Nothing was found. Drop the lock, generate, and try again.
    Town::generate(num);
    towns::TOWN_REGISTRY.read().get(&num).unwrap().clone()
}

pub fn area_exists(coords: (usize, usize, usize)) -> bool {
    match towns::TOWN_REGISTRY.read().get(&coords.0) {
        Some(t) => (&t.get_areas()[coords.1][coords.2]).is_some(),
        _ => false
    }
}

/// Used for borrowing a reference to the area located
/// at `coords`. See `entities()`.
pub fn area<F, T>(coords: (usize, usize, usize), callback: F) -> Option<T>
    where F: FnOnce(&Area) -> T
{
    // Need to make sure the data isn't moved.
    // Difficult to do functionally.
    match &town(coords.0).get_areas()[coords.1][coords.2] {
        Some(ref a) => Some(callback(&**a)),
        None => None
    }
}

/// Used for borrowing a reference to the starting area
/// in the specified `town_num`. Panics if no starting
/// area exists in the town, as this would be a bug and
/// should thus be fixed.
pub fn starting_area<F, T>(town_num: usize, callback: F) -> T
    where F: FnOnce(&Area) -> T
{
    let (x, z) = towns::STARTING_COORDS;
    let town = town(town_num);
    if let Some(a) = &town.get_areas()[x][z] {
        return callback(&**a);
    }
    panic!("Error: Starting area not generated for this town.");
}