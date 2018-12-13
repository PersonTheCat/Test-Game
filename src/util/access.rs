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

pub fn entity<T, F>(accessor: EntityAccessor, callback: F) -> Option<T>
    where F: FnOnce(&Entity) -> T
{
    area(accessor.coordinates, |area| {
        area.borrow_entities_ref().iter()
            .find(|e| e.get_id() == accessor.entity_id)
            .and_then(|e| Some(callback(&**e)))
    })
    .expect("Area no longer exists.")
}

pub fn player_meta(player_id: usize) -> Arc<PlayerMeta> {
    PLAYER_META.lock()
        .iter()
        .find(|p| p.get_player_id() == player_id)
        .expect("Called tried access a player who was not registered.")
        .clone()
}

pub fn player_meta_sender(channel: &ChannelInfo) -> Option<Arc<PlayerMeta>> {
    PLAYER_META.lock()
        .iter()
        .find(|p| p.get_channel() == *channel)
        .and_then(|p| Some(p.clone()))
}

pub fn context<T, F>(player: &PlayerMeta, callback: F) -> Option<T>
    where F: FnOnce(&Town, &Area, &Entity) -> T
{
    let coordinates = player.get_coordinates();

    return town(coordinates.0, |town| {
        let area = match &town.get_areas()[coordinates.1][coordinates.2] {
            Some(ref a) => a,
            None => return None,
        };

        let entities = area.borrow_entities_ref();

        let entity = entities
            .iter()
            .find(|e| e.get_id() == player.get_player_id())
            .expect("Area no longer contains entity.");

        Some(callback(town, &**area, &**entity))
    });
}

pub fn town<F, T>(num: usize, callback: F) -> T
    where F: FnOnce(&Town) -> T
{
    unsafe {
        if let Some(ref mut registry) = towns::TOWN_REGISTRY {
            match registry.get(&num) {
                Some(ref town) => return callback(town),
                None => {
                    towns::Town::new(num); // Registered automatically.
                    return town(num, callback); // Potential overflow errors; seems fairly safe.
                }
            }
        } else {
            panic!("Error: Town registry not setup in time.")
        }
    }
}

pub fn area_exists(coords: (usize, usize, usize)) -> bool {
    town(coords.0, |town| {
        town.get_areas()[coords.1][coords.2].is_some()
    })
}

pub fn area<F, T>(coords: (usize, usize, usize), callback: F) -> Option<T>
    where F: FnOnce(&Area) -> T
{
    town(coords.0, |town| {
        match &town.get_areas()[coords.1][coords.2] {
            Some(a) => Some(callback(&**a)),
            None => None,
        }
    })
}

pub fn starting_area<F, T>(town_num: usize, callback: F) -> T
    where F: FnOnce(&Area) -> T
{
    town(town_num, |town| {
        let (x, z) = towns::STARTING_COORDS;
        if let Some(ref a) = &town.get_areas()[x][z] {
            callback(&**a)
        } else {
            panic!("Error: Starting area not generated for town #{}.", town_num);
        }
    })
}
