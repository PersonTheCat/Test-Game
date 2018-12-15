use crate::types::{
    areas::area_settings::PathPreference::*,
    areas::area_settings::AREA_REGISTRY,
    areas::gates::Gate,
    areas::paths::Path,
    classes::{self, Class},
};

use crate::player_data::PlayerMeta;
use crate::traits::Area;
use crate::*;

use self::Direction::*;

use rand::{random, thread_rng, Rng};
use lazy_static::lazy_static;
use array_init::array_init;
use hashbrown::HashMap;
use parking_lot::RwLock;
use atomic::Atomic;

use std::sync::atomic::Ordering::*;
use std::sync::Arc;

/// Width / Depth
pub const W: usize = 11; // -> z; ew
pub const D: usize = 10; // -> x; ns

/// Center
pub const C: usize = W / 2;
pub const CD: usize = (D - 1) / 2;

pub const STARTING_COORDS: (usize, usize) = (0, C);

/// 0-1 chance to go straight instead of turning.
/// 0 => diagonal lines.
/// 1 => exactly straight.
const STRAIGHTNESS_BIAS: f32 = 0.4;

/// How empty rooms will appear on the map.
const EMPTY_ROOM_PAT: &str = " · ";

const CURRENT_ROOM_PAT: &str = "(X)";

/// Towns are mapped to their index instead of being
/// stored in an array for two reasons:
/// - They can be registered and generated out of
///   order.
/// - They may at some point be mapped to something
///   other than indices, i.e. strings.
type TownRegistry = HashMap<usize, Arc<Town>>;

/// A convenience type generated from the the size
/// values above.
pub type Map = [[Option<Box<Area>>; W]; D];

/// A registry stored by each town that maps where
/// each type of area is stored. Certainly slightly
/// faster than searching all areas, but this would
/// not be true if towns for some reason became much
/// larger at some point in the future.
pub type Locations = Vec<(&'static str, (usize, usize))>;

lazy_static! {
    /// All towns are loaded statically.
    pub static ref TOWN_REGISTRY: RwLock<TownRegistry> = RwLock::new(HashMap::new());
}

pub fn setup_town_registry() {}

fn register_town(town_num: usize, town: Town) {
    TOWN_REGISTRY.write().insert(town_num, Arc::new(town));
}

pub struct Town {
    pub name: String, // Might remove.
    pub town_num: usize,
    pub areas: Map,
    pub coords: Locations, // Might remove; probably no benefit.
    pub key_found: Atomic<bool>,
    pub unlocked: Atomic<bool>,
    pub class: Class,
}

impl Town {
    pub fn generate(town_num: usize) {
        let class = classes::random_class();
        let (map, coords) = generate_map(town_num, class);

        register_town(town_num, Town {
            name: String::from(""),
            town_num,
            areas: map,
            coords,
            key_found: Atomic::new(false),
            unlocked: Atomic::new(false),
            class,
        });
    }

    /// Access the registry to locate the
    /// name of a town.
    pub fn find_name(town: usize) -> Option<String> {
        TOWN_REGISTRY.read()
            .get(&town)
            .and_then(|t| Some(t.name.clone()))
    }

    /// Access the registry to locate the
    /// class of a town.
    pub fn find_class(town: usize) -> Option<Class> {
        TOWN_REGISTRY.read()
            .get(&town)
            .and_then(|t| Some(t.class))
    }

    pub fn find_map(town: usize, player: &PlayerMeta) -> Option<String> {
        TOWN_REGISTRY.read()
            .get(&town)
            .and_then(|t| Some(t.get_map(player)))
    }

    pub fn get_name(&self) -> &String {
        &self.name
    }

    pub fn get_areas(&self) -> &Map {
        &self.areas
    }

    /// Find an area that matches the specified
    /// type identifier, specified by the area's
    /// author.
    pub fn locate_area(&self, typ: &str) -> Option<(usize, usize, usize)> {
        for (area, (x, z)) in &self.coords {
            if *area == typ {
                return Some((*&self.town_num, *x, *z));
            }
        }
        None
    }

    /// Shorthand for calling `locate_area("gate")`.
    /// This will panic if the area does not exist,
    /// as this implies there was an error generating
    /// the map which needs to be fixed.
    pub fn end_gate(&self) -> (usize, usize, usize) {
        self.locate_area("gate")
            .expect("A gate was not placed for this map.")
    }

    pub fn set_key_found(&self, b: bool) {
        self.key_found.store(b, SeqCst);
    }

    pub fn key_found(&self) -> bool {
        self.key_found.load(SeqCst)
    }

    pub fn set_unlocked(&self, b: bool) {
        self.unlocked.store(b, SeqCst);
    }

    pub fn unlocked(&self) -> bool {
        self.unlocked.load(SeqCst)
    }

    pub fn get_class(&self) -> Class {
        self.class
    }

    /// Generates a formatted map for the player.
    pub fn get_map(&self, player: &PlayerMeta) -> String {
        let mut ret = String::new();
        let horizontal_border = "-".repeat((W * 3) + 1);;

        ret += &horizontal_border;
        ret += "\n";

        for (x, z_axis) in self.areas.iter().enumerate().rev() {
            ret += "|";
            for (z, area) in z_axis.iter().enumerate() {
                match area {
                    Some(a) if player.player_has_visited((self.town_num, x, z)) => {
                        if area_coords_match(x, z, player.get_coordinates()) {
                            ret += CURRENT_ROOM_PAT;
                        } else {
                            ret += &format!("{}", a.get_map_icon());
                        }
                    }
                    _ => ret+= EMPTY_ROOM_PAT
                }
            }
            ret += "|";
            if x > 0 {
                ret += "\n";
            }
        }
        ret += "\n";
        ret + &horizontal_border
    }
}

#[derive(Copy, Clone)]
enum Direction {
    Forward,
    Left,
    Right,
}

fn generate_map(town_num: usize, class: Class) -> (Map, Locations) {
    let mut map = empty_map();
    let mut coords = Vec::new();

    // Maps are generated on the basis of which
    // direction was previously generated and
    // direction is being used now. This is a
    // bit silly and can definitely be improved,
    // but as a very quick and dry implementation,
    // it does ensure that areas are always
    // connected.
    let mut previous_dir; // = Forward; <- unused assignment
    let mut current_dir = Forward;
    let mut next_dir = Forward;

    // Keep records of which coordinates are
    // generating and count the areas as
    // they're placed.
    let mut current_x = 0;
    let mut current_z = C;
    let mut area_num = 1;

    // Generate the first two areas manually.
    // They'll always be the same.
    gen_starting_areas(
        class,
        town_num,
        &mut area_num,
        &mut current_x,
        current_z,
        &mut map,
    );

    // Only connect forward. We need to make sure these
    // connections are listed in a consistent order.
    connect_forward(0, C, 1, C, &map);

    while current_x < D - 1 { // < Max depth index
        // Cycle the directions backward, recalculate next_dir.
        previous_dir = current_dir;
        current_dir = next_dir;
        next_dir = get_next_dir(current_dir, previous_dir);

        // Update the coordinates
        let previous_x = current_x;
        let previous_z = current_z;

        update_coords(&mut current_x, &mut current_z, &mut next_dir);
        add_next_path(town_num, &mut area_num, current_x, current_z, &mut map);
        connect_forward(previous_x, previous_z, current_x, current_z, &map);
    }

    // Relatively inefficient way to go back through
    // and connect areas. Must happen in this order.
    modify_path(class, town_num, &mut coords, &mut map);
    trace_connect_backward(&mut current_x, &mut current_z, &map);
    add_branches(class, town_num, &mut area_num, &mut coords, &mut map);

    (map, coords)
}

fn empty_map() -> Map {
    array_init(|_| array_init(|_| None))
}

fn gen_starting_areas(class: Class, town_num: usize, area_num: &mut usize, current_x: &mut usize, current_z: usize, map: &mut Map) {
    map[*current_x][current_z] = Some(Gate::new(class, *area_num, (town_num, *current_x, current_z)));
    *area_num += 1;
    *current_x += 1;
    map[*current_x][current_z] = Some(Path::new(*area_num, (town_num, *current_x, current_z)));
}

/// Updates `current_x` and `current_z` on the
/// basic of which direction is being generated.
fn update_coords(current_x: &mut usize, current_z: &mut usize, next_dir: &mut Direction) {
    match *next_dir {
        Forward => {
            *current_x += 1;
        }
        Right => { // Leave >= 1 area margin.
            if *current_z < W - 2 {
                *current_z += 1;
            } else {
                *current_x += 1;
                *next_dir = Forward;
            }
        }
        Left => { // !!
            if *current_z > 1 {
                *current_z -= 1;
            } else {

                *current_x += 1;
                *next_dir = Forward;
            }
        }
    }
}

fn add_next_path(town_num: usize, area_num: &mut usize, current_x: usize, current_z: usize, map: &mut Map) {
    *area_num += 1;
    let next_area = Path::new(*area_num, (town_num, current_x, current_z));
    map[current_x][current_z] = Some(next_area);
}

/// Literally finds all existing areas in reverse order
/// and places connections between them in that order.
/// Not very efficient at all. Needs work.
fn trace_connect_backward(current_x: &mut usize, current_z: &mut usize, map: &Map) {
    let mut previous_x = *current_x;
    let mut previous_z = *current_z;

    while *current_x > 0 {
        if let Some(ref _area) = map[*current_x - 1][*current_z] {
            *current_x -= 1;
        } else if let Some(ref _area) = map[*current_x][*current_z - 1] {
            *current_z -= 1;
        } else {
            *current_z += 1;
        }
        connect_forward(previous_x, previous_z, *current_x, *current_z, &map);
        previous_x = *current_x;
        previous_z = *current_z;
    }
}

fn modify_path(class: Class, town_num: usize, coords: &mut Locations, map: &mut Map) {
    let registry = AREA_REGISTRY.lock();
    let areas_on_path = registry
        .iter()
        .filter(|s| s.path_pref == OnPath && random::<f32>() <= s.chance);

    for settings in areas_on_path {
        let (mut x, mut z);
        while { // Do-while
            x = thread_rng().gen_range(settings.min_x, settings.max_x + 1);
            z = get_z_of_path(x, &map);
            !is_replaceable(x, z, &map)
        } {}

        // Forward connections would be lost.
        let previous_connections = get_previous_connections(x, z, &map);
        let area_num = get_area_num(x, z, &map);
        let new_area = (settings.constructor)(class, area_num, (town_num, x, z));

        for connection in previous_connections {
            new_area.add_connection(connection);
        }

        coords.push((new_area.get_type(), (x, z)));
        map[x][z] = Some(new_area);
    }
}

fn add_branches(class: Class, town_num: usize, area_num: &mut usize, coords: &mut Locations, map: &mut Map) {
    let registry = AREA_REGISTRY.lock();
    let areas_off_path = registry
        .iter()
        .filter(|s| s.path_pref == OffPath && random::<f32>() <= s.chance);

    for settings in areas_off_path {
        let mut x;
        let mut on_off = None;
        while let None = on_off {
            x = thread_rng().gen_range(settings.min_x, settings.max_x + 1);
            on_off = get_coords_beside_path(x, &map);
        }

        let ((on_x, on_z), (off_x, off_z)) = on_off.unwrap();
        *area_num += 1;

        let new_area = (settings.constructor)(class, *area_num, (town_num, off_x, off_z));
        coords.push((new_area.get_type(), (off_x, off_z)));
        map[off_x][off_z] = Some(new_area);

        connect_paths(on_x, on_z, off_x, off_z, &map);
    }
}

/// Randomly picks a direction and locates the last
/// empty spot. Redundant code is used to avoid
/// unnecessarily calculating a second. Could
/// probably be cleaned up a bit, or at least
/// ignored.
fn get_coords_beside_path(x: usize, map: &Map) -> Option<((usize, usize), (usize, usize))> {
    if random() { // Start on the left.
        if let Some(coords) = get_coords_to_left(x, &map) {
            return Some(((coords.0, coords.1 + 1), coords));
        } else if let Some(coords) = get_coords_to_right(x, &map) {
            return Some(((coords.0, coords.1 - 1), coords));
        }
    } else { // Start on the right.
        if let Some(coords) = get_coords_to_right(x, &map) {
            return Some(((coords.0, coords.1 - 1), coords));
        } else if let Some(coords) = get_coords_to_left(x, &map) {
            return Some(((coords.0, coords.1 + 1), coords));
        }
    }
    return None;
}

fn get_coords_to_left(x: usize, map: &Map) -> Option<(usize, usize)> {
    for z in 0..map[x].len() {
        if let Some(ref area) = map[x][z] {
            if area.get_type() == "path"
            // Possibly unnecessary. Recheck this.
            {
                return Some((x, z - 1));
            } else {
                return None;
            }
        }
    }
    panic!("Error: Expected a path at ({}, Z). Found nothing.", x);
}

fn get_coords_to_right(x: usize, map: &Map) -> Option<(usize, usize)> {
    for z in (0..map[x].len()).rev() {
        if let Some(ref area) = map[x][z] {
            if area.get_type() == "path" {
                return Some((x, z + 1));
            } else {
                return None;
            }
        }
    }
    panic!("Error: Expected a path at ({}, Z). Found nothing.", x);
}
fn get_z_of_path(x: usize, map: &Map) -> usize {
    for z in 0..map[x].len() {
        if let Some(ref _area) = map[x][z] {
            return z;
        }
    }
    panic!("Error: Expected a path at ({}, Z). Found nothing.", x);
}

fn is_replaceable(x: usize, z: usize, map: &Map) -> bool {
    if let Some(ref area) = map[x][z] {
        if area.get_type() == "path" {
            return true;
        }
    }
    false
}

fn get_area_num(x: usize, z: usize, map: &Map) -> usize {
    if let Some(ref area) = map[x][z] {
        return area.get_area_num();
    }
    panic!("Error: An existing area was somehow removed.");
}

/// This is somewhat of a silly algorithm, but essentially
/// all it's doing is following these rules:
/// - If we're aren't currently going forward, we cannot
///   go any direction *but* forward. This is because
///   next_dir should never go horizontally > 1x.
/// - Generate a random number. If the probability to
///   go forward regardless of `previous_dir` is met,
///   it will go forward anyway. Higher probability, ->
///   higher chance of going straight.
/// - If we've previously gone horizontally, we will need
///   to repeat this direction. We do this to avoid creating
///   a loop that circles around. This was an aesthetic
///   choice that has no practical significance.
/// - If we previously went forward, we can go in any
///   direction at random, as it does not matter.
fn get_next_dir(current_dir: Direction, previous_dir: Direction) -> Direction {
    match current_dir {
        Forward => {
            let rand_f32: f32 = thread_rng().gen_range(0.0, 1.0);

            if rand_f32 <= STRAIGHTNESS_BIAS {
                Forward
            } else {
                match previous_dir {
                    Forward => *choose(&[Left, Right]),
                    Left => Left,
                    Right => Right,
                }
            }
        }
        _ => Forward,
    }
}

fn get_previous_connections(x: usize, z: usize, map: &Map) -> Vec<(usize, usize, usize)> {
    if let Some(ref area) = map[x][z] {
        return area.get_connections();
    }
    panic!("The referenced area was somehow lost...");
}

fn connect_forward(x1: usize, z1: usize, x2: usize, z2: usize, map: &Map) {
    if let Some(ref area1) = map[x1][z1] {
        if let Some(ref area2) = map[x2][z2] {
            area1.add_connection(area2.get_coordinates());
        }
    }
}

fn connect_paths(x1: usize, z1: usize, x2: usize, z2: usize, map: &Map) {
    if let Some(ref area1) = map[x1][z1] {
        if let Some(ref area2) = map[x2][z2] {
            area1.add_connection(area2.get_coordinates());
            area2.add_connection(area1.get_coordinates());
        }
    }
}

/// Generates the formatted map without hiding
/// areas that haven't been explored by the
/// the user. This was mostly used for debugging
/// purposes, but now could probably be removed.
fn format_map(map: &Map) -> String {
    let mut ret = String::new();

    for (x, z_axis) in map.iter().enumerate() {
        for area in z_axis.iter() {
            match area {
                Some(a) => ret += format!("{}", a.get_map_icon()).as_str(),
                None => ret += EMPTY_ROOM_PAT
            };
        }
        if x > 0 {
            ret += "\n";
        }
    }
    ret
}

fn area_coords_match(x: usize, z: usize, coords: (usize, usize, usize)) -> bool {
    x == coords.1 && z == coords.2
}