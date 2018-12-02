use player_data::{ PlayerMeta, PLAYER_META };
use traits::{ Entity, Area };
use towns::{ self, Town };

/**
 *   These are a bunch of methods I use for accessing
 * variables statically provided information about them.
 * It's relatively inefficient, and while sometimes it
 * can result in fairly pretty code, it also quickly
 * increases the number of lines in a way that can be
 * difficult to parse when used too often.
 *   This solution is in many ways inferior to using
 * raw pointers. It is not a complete substitute (see
 * shops and trades) and of course it is much slower
 * as well as more difficult to use. As such, I may
 * wind up accepting defeat on living without normal
 * pointers and someday make the switch back.
 *   ...Or not. It's just a text-based game!
 */

#[derive(Copy, Clone)]
pub struct EntityAccessor
{
    pub coordinates: (usize, usize, usize),
    pub entity_id: usize,
    pub is_player: bool
}

impl EntityAccessor
{
    fn update(self) -> EntityAccessor
    {
        if self.is_player
        {
            access_player_meta(self.entity_id, | p | p.get_accessor())
                .expect("Player data no longer exists.")
        }
        else { self }
    }
}

pub fn access_entity<T, F>(accessor: EntityAccessor, callback: F) -> Option<T>
    where F: FnOnce(&Entity) -> T
{
    let accessor = accessor.update();

    access_area(accessor.coordinates, | area |
    {
        let entities = area.borrow_entities_ref();

        let entity = entities
            .iter()
            .find(| e | e.get_id() == accessor.entity_id );

        match entity
        {
            Some(ref e) => Some(callback(&***e)),
            None => None
        }
    })
    .expect("Area no longer exists.")
}

pub fn access_player_meta_sender<T, F>(channel: ::ChannelInfo, callback: F) -> Option<T>
    where F: FnOnce(&PlayerMeta) -> T
{
    unsafe { if let Some(ref registry) = PLAYER_META
    {
        let player = registry
            .iter()
            .find(| p | p.channel == channel);

        match player
        {
            Some(p) => return Some(callback(p)),
            None => return None
        }
    }}
    panic!("Error: Player meta registry not established in time.");
}

pub fn access_player_meta<T, F>(player_id: usize, callback: F) -> Option<T>
    where F: FnOnce(&mut PlayerMeta) -> T
{
    unsafe { if let Some(ref mut registry) = PLAYER_META
    {
        let player = registry
            .iter_mut()
            .find(| p | p.player_id == player_id);

        match player
        {
            Some(p) => return Some(callback(p)),
            None => return None
        }
    }}
    None
}

pub fn access_player_context<T, F>(player_id: usize, callback: F) -> Option<T>
    where F: FnOnce(&mut PlayerMeta, &Town, &Area, &Entity) -> T
{
    unsafe { if let Some(ref mut registry) = PLAYER_META
    {
        let player = registry
            .iter_mut()
            .find(| p | p.player_id == player_id);

        let player = match player
        {
            Some(p) => p,
            None => return None
        };

        return access_town(player.coordinates.0, | town |
        {
            let area = match &town.get_areas()[player.coordinates.1][player.coordinates.2]
            {
                Some(ref a) => a,
                None => return None
            };

            let entities = area.borrow_entities_ref();

            let entity = entities
                .iter()
                .find(| e | { e.get_id() == player_id })
                .expect("Area no longer contains entity.");

            Some(callback(player, town, &**area, &**entity))
        });
    }}
    None
}

/**
 * This still requires that all variables associated
 * with the player's context be borrowed, but is
 * more readable under some circumstances.
 */
pub fn access_player<T, F>(player_id: usize, callback: F) -> Option<T>
    where F: FnOnce(&Entity) -> T
{
    access_player_context(player_id, | _, _, _, e | callback(e))
}

pub fn get_player_for_sender(channel: ::ChannelInfo) -> Option<usize>
{
    access_player_meta_sender(channel, | p | p.player_id)
}

pub fn access_town<F, T>(num: usize, callback: F) -> T
    where F: FnOnce(&Town) -> T
{
    unsafe { if let Some(ref mut registry) = towns::TOWN_REGISTRY
    {
        match registry.get(&num)
        {
            Some(ref town) => { return callback(town) },
            None =>
            {
                towns::Town::new(num); // Registered automatically.
                return access_town(num, callback); // Potential overflow errors; seems fairly safe.
            }
        }
    }
    else { panic!("Error: Town registry not setup in time.") }}
}

pub fn area_exists(coords: (usize, usize, usize)) -> bool
{
    access_town(coords.0, | town |
    {
        town.get_areas()[coords.1][coords.2].is_some()
    })
}

pub fn access_area<F, T>(coords: (usize, usize, usize), callback: F) -> Option<T>
    where F: FnOnce(&Area) -> T
{
    access_town(coords.0, | town |
    {
        match &town.get_areas()[coords.1][coords.2]
        {
            Some(a) => Some(callback(&**a)),
            None => None
        }
    })
}

pub fn access_starting_area<F, T>(town_num: usize, callback: F) -> T
    where F: FnOnce(&Area) -> T
{
    access_town(town_num, | town |
    {
        let area = &town.get_areas()[towns::STARTING_COORDS.0][towns::STARTING_COORDS.1];

        if let Some(ref a) = area { callback(&**a) }

        else { panic!("Error: Starting area not generated for town #{}.", town_num); }
    })
}