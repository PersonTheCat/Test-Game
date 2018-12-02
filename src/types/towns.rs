use types::{
    classes::{ self, Class },
    areas::paths::Path,
    areas::gates::Gate,
    areas::area_settings::AREA_REGISTRY,
    areas::area_settings::PathPreference::*
};

use player_data::PlayerMeta;
use traits::Area;
use var_access;
use self::Direction::*;

use std::cell::Cell;
use rand::{ Rng, thread_rng, random };
use array_init::array_init;
use hashbrown::HashMap;

/** Width / Depth */
pub const W: usize = 11; // -> z; ew
pub const D: usize = 10; // -> x; ns

/** Center */
pub const C: usize = W / 2;
pub const CD: usize = (D - 1) / 2;

pub const STARTING_COORDS: (usize, usize) = (0, C);

/**
 * 0-1 chance to go straight instead of turning.
 * 0 => diagonal lines.
 * 1 => exactly straight.
 */
const STRAIGHTNESS_BIAS: f32 = 0.4;

/**
 * How empty rooms will appear on the map.
 */
const EMPTY_ROOM_PAT: &str = " · ";

/** All towns are loaded statically */
pub static mut TOWN_REGISTRY: Option<HashMap<usize, Town>> = None;

pub unsafe fn setup_town_registry()
{
    TOWN_REGISTRY = Some(HashMap::new());
}

pub fn get_registry_size() -> usize
{
    unsafe { if let Some(ref registry) = TOWN_REGISTRY
    {
        return registry.len();
    }}
    panic!("Error: Registry was not loaded in time. Cannot view size.");
}

fn register_town(town_num: usize, town: Town)
{
    unsafe { if let Some(ref mut registry) = TOWN_REGISTRY
    {
        registry.insert(town_num, town);
    }
    else { panic!("Error: Registry has not been setup. Cannot register new town."); }}
}

pub type Map = [[Option<Box<Area>>; W]; D];
pub type Locations = Vec<(&'static str, (usize, usize))>;

pub struct Town
{
    pub name: String, // Might remove.
    pub town_num: usize,
    pub areas: Map,
    pub coords: Locations, // Might remove; probably no benefit.
    pub key_found: Cell<bool>,
    pub unlocked: Cell<bool>,
    pub class: Class
}

impl Town
{
    pub fn new(town_num: usize)
    {
        let class = classes::random_class();

        let (map, coords) = generate_map(town_num, class);

        //println!("{}", format_map(&map));

        let town = Town
        {
            name: String::from(""),
            town_num,
            areas: map,
            coords,
            key_found: Cell::new(false),
            unlocked: Cell::new(false),
            class
        };

        register_town(town_num, town);
    }

    pub fn get_name(&self) -> &String
    {
        &self.name
    }

    pub fn find_name(town: usize) -> Option<String>
    {
        unsafe { if let Some(ref registry) = TOWN_REGISTRY
        {
            match registry.get(&(town - 1))
            {
                Some(t) => Some(t.name.clone()),
                None => None
            };
        }}
        panic!("Something went wrong loading town #{}.", town);
    }

    pub fn get_areas(&self) -> &Map
    {
        &self.areas
    }

    pub fn locate_area(&self, typ: &str) -> Option<(usize, usize, usize)>
    {
        for (area, (x, z)) in &self.coords
        {
            if *area == typ { return Some((*&self.town_num, *x, *z)); }
        }
        None
    }

    pub fn end_gate(&self) -> (usize, usize, usize)
    {
        self.locate_area("gate")
            .expect("A gate was not placed for this map.")
    }

    pub fn set_key_found(&self, b: bool)
    {
        self.key_found.set(b);
    }

    pub fn key_found(&self) -> bool
    {
        self.key_found.get()
    }

    pub fn set_unlocked(&self, b: bool)
    {
        self.unlocked.set(b);
    }

    pub fn unlocked(&self) -> bool
    {
        self.unlocked.get()
    }

    pub fn get_class(&self) -> Class
    {
        self.class
    }

    pub fn find_class(town: usize) -> Option<Class>
    {
        unsafe { if let Some(ref registry) = TOWN_REGISTRY
        {
            match registry.get(&(town - 1))
            {
                Some(t) => Some(t.class),
                None => None
            };
        }}
        panic!("Something went wrong loading town #{}.", town);
    }

    pub fn get_map_for_player(&self, player_id: usize) -> String
    {
        var_access::access_player_meta(player_id, | player |
        {
            self._get_map_for_player(player)
        })
        .unwrap()
    }

    pub fn _get_map_for_player(&self, player: &mut PlayerMeta) -> String
    {
        let mut ret = String::new();
        let mut horizontal_border = String::new();

        for _ in 0..= (W * 3) + 1
        {
            horizontal_border += "-";
        }

        ret += horizontal_border.as_str();
        ret += "\n";

        for x in (0..self.areas.len()).rev()
        {
            let z_axis = &self.areas[x];

            ret += "|";

            for z in 0..z_axis.len()
            {
                let z_coord = &z_axis[z];

                match z_coord
                {
                    Some(coord) =>
                    {
                        if player.player_has_visited((self.town_num, x, z))
                        {
                            if area_coords_match(x, z, player.coordinates)
                            {
                                ret += "(X)";
                            }
                            else { ret += format!("{}", coord.get_map_icon()).as_str(); }
                        }
                        else { ret += EMPTY_ROOM_PAT; }
                    },
                    None => ret += EMPTY_ROOM_PAT
                };
            }
            ret += "|";

            if x > 0 { ret += "\n"; }
        }
        ret += "\n";
        ret + horizontal_border.as_str()
    }
}

#[derive(Copy, Clone)]
enum Direction
{
    Forward,
    Left,
    Right
}

fn generate_map(town_num: usize, class: Class) -> (Map, Locations)
{
    let mut map= empty_map();
    let mut coords = Vec::new();

    let mut previous_dir; // = Forward; <- unused assignment
    let mut current_dir = Forward;
    let mut next_dir = Forward;

    let mut current_x = 0;
    let mut current_z = C;

    let mut area_num = 1;

    /**
     * Generate the first two areas manually.
     * They'll always be the same.
     */
    gen_starting_areas(class, town_num, &mut area_num, &mut current_x, current_z, &mut map);

    /**
     * Only connect forward. We need to make sure these
     * connections are listed in a consistent order.
     */
    connect_forward(0, C, 1, C, &map);

    while current_x < D - 1 // < Max depth index
    {
        /**
         * Cycle the directions backward, recalculate next_dir.
         */
        previous_dir = current_dir;
        current_dir = next_dir;
        next_dir = get_next_dir(current_dir, previous_dir);

        let previous_x = current_x;
        let previous_z = current_z;

        update_coords(&mut current_x, &mut current_z, &mut next_dir);
        add_next_path(town_num, &mut area_num, current_x, current_z, &mut map);
        connect_forward(previous_x, previous_z, current_x, current_z, &map);
    }

    /**
     * Relatively inefficient way to go back through
     * and connect areas. Must happen in this order.
     */
    modify_path(class, town_num, &mut coords, &mut map);
    trace_connect_backward(&mut current_x, &mut current_z, &map);
    add_branches(class, town_num, &mut area_num, &mut coords, &mut map);

    (map, coords)
}

fn empty_map() -> Map
{
    array_init(| _ |
    {
        array_init(| _ | { None })
    })
}

fn gen_starting_areas(class: Class, town_num: usize, area_num: &mut usize, current_x: &mut usize, current_z: usize, map: &mut Map)
{
    map[*current_x][current_z] = Some(Gate::new(class, *area_num,(town_num, *current_x, current_z)));

    *area_num += 1;
    *current_x += 1;

    map[*current_x][current_z] = Some(Path::new(*area_num,(town_num, *current_x, current_z)));
}

fn update_coords(current_x: &mut usize, current_z: &mut usize, next_dir: &mut Direction)
{
    match *next_dir
    {
        Forward => { *current_x += 1; },
        Right =>
        {
            if *current_z < W - 2
            {
                *current_z += 1;
            }
            else // Leave >= 1 area margin.
            {
                *current_x += 1;
                *next_dir = Forward;
            }
        },
        Left =>
        {
            if *current_z > 1
            {
                *current_z -= 1;
            }
            else // !!
            {
                *current_x += 1;
                *next_dir = Forward;
            }
        }
    }
}

fn add_next_path(town_num: usize, area_num: &mut usize, current_x: usize, current_z: usize, map: &mut Map)
{
    *area_num += 1;

    let next_area = Path::new(*area_num, (town_num, current_x, current_z));

    map[current_x][current_z] = Some(next_area);
}

fn trace_connect_backward(current_x: &mut usize, current_z: &mut usize, map: &Map)
{
    let mut previous_x = *current_x;
    let mut previous_z = *current_z;

    while *current_x > 0
    {
        if let Some(ref _area) = map[*current_x - 1][*current_z]
        {
            *current_x -= 1;
        }
        else if let Some(ref _area) = map[*current_x][*current_z - 1]
        {
            *current_z -= 1;
        }
        else if let Some(ref _area) = map[*current_x][*current_z + 1]
        {
            *current_z += 1;
        }
        else { panic!("Tried to trace backward to impossible coordinates."); }

        connect_forward(previous_x, previous_z, *current_x, *current_z, &map);

        previous_x = *current_x;
        previous_z = *current_z;
    }
}

fn modify_path(class: Class, town_num: usize, coords: &mut Locations, map: &mut Map)
{
    unsafe { if let Some(ref registry ) = AREA_REGISTRY
    {
        let areas_on_path = registry
            .iter()
            .filter(| s |
                s.path_pref == OnPath && random::<f32>() <= s.chance
            );

        for settings in areas_on_path
        {
            let mut x = thread_rng().gen_range(settings.min_x, settings.max_x + 1);
            let mut z = get_z_of_path(x, &map);

            while !is_replaceable(x, z, &map)
            {
                x = thread_rng().gen_range(settings.min_x, settings.max_x + 1);
                z = get_z_of_path(x, &map);
            }

            let area_num = get_area_num(x, z, &map);

            /**
             * Forward connections would be lost.
             */
            let previous_connections = get_previous_connections(x, z, &map);

            let new_area = (settings.constructor)(class, area_num, (town_num, x, z));

            for connection in previous_connections
            {
                new_area.add_connection(connection);
            }

            coords.push((new_area.get_type(), (x, z)));
            map[x][z] = Some(new_area);
        }
    }
        else { panic!("Area registry not setup in time."); }
    };
}

fn add_branches(class: Class, town_num: usize, area_num: &mut usize, coords: &mut Locations, map: &mut Map)
{
    unsafe { if let Some(ref registry) = AREA_REGISTRY
    {
        let areas_off_path = registry
            .iter()
            .filter(| s |
                s.path_pref == OffPath && random::<f32>() <= s.chance
            );

        for settings in areas_off_path
        {
            let mut x = thread_rng().gen_range(settings.min_x, settings.max_x + 1);
            let mut on_off = get_coords_beside_path(x, &map);

            while let None = on_off
            {
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
        else { panic!("Area registry not setup in time."); }
    };
}

fn get_coords_beside_path(x: usize, map: &Map) -> Option<((usize, usize),(usize, usize))>
{
    if random() // Start on the left.
    {
        if let Some(coords) = get_coords_to_left(x, &map)
        {
            return Some(((coords.0, coords.1 + 1), coords));
        }
        else if let Some(coords) = get_coords_to_right(x, &map)
        {
            return Some(((coords.0, coords.1 - 1), coords));
        }
    }
    else // Start on the right.
    {
        if let Some(coords) = get_coords_to_right(x, &map)
        {
            return Some(((coords.0, coords.1 - 1), coords));
        }
        else if let Some(coords) = get_coords_to_left(x, &map)
        {
            return Some(((coords.0, coords.1 + 1), coords));
        }
    }
    return None;
}

fn get_coords_to_left(x: usize, map: &Map) -> Option<(usize, usize)>
{
    for z in 0..map[x].len()
    {
        if let Some(ref area) = map[x][z]
        {
            if area.get_type() == "path" // Possibly unnecessary. Recheck this.
            {
                return Some((x, z - 1));
            }
            else { return None; }
        }
    }
    panic!("Error: Expected a path at ({}, Z). Found nothing.", x);
}

fn get_coords_to_right(x: usize, map: &Map) -> Option<(usize, usize)>
{
    for z in (0..map[x].len()).rev()
    {
        if let Some(ref area) = map[x][z]
        {
            if area.get_type() == "path"
            {
                return Some((x, z + 1));
            }
            else { return None;}
        }
    }
    panic!("Error: Expected a path at ({}, Z). Found nothing.", x);
}
fn get_z_of_path(x: usize, map: &Map) -> usize
{
    for z in 0..map[x].len()
    {
        if let Some(ref _area) = map[x][z]
        {
            return z;
        }
    }
    panic!("Error: Expected a path at ({}, Z). Found nothing.", x);
}

fn is_replaceable(x: usize, z: usize, map: &Map) -> bool
{
    if let Some(ref area) = map[x][z]
    {
        if area.get_type() == "path"
        {
            return true;
        }
    }
    false
}

fn get_area_num(x: usize, z: usize, map: &Map) -> usize
{
    if let Some(ref area) = map[x][z]
    {
        return area.get_area_num();
    }
    panic!("Error: An existing area was somehow removed.");
}

fn get_next_dir(current_dir: Direction, previous_dir: Direction) -> Direction
{
    match current_dir
    {
        Forward =>
        {
            let rand_f32: f32 = thread_rng().gen_range(0.0, 1.0);

            if rand_f32 <= STRAIGHTNESS_BIAS
            {
                Forward
            }
            else { match previous_dir
            {
                Forward =>
                {
                    *::choose(&[Left, Right])
                },
                Left => Left,
                Right => Right
            }}
        },
        _ => Forward
    }
}

fn get_previous_connections(x: usize, z: usize, map: &Map) -> Vec<(usize, usize, usize)>
{
    if let Some(ref area) = map[x][z]
    {
        return area.get_connections();
    }
    panic!("The referenced area was somehow lost...");
}

fn connect_forward(x1: usize, z1: usize, x2: usize, z2: usize, map: &Map)
{
    if let Some(ref area1) = map[x1][z1]
    {
        if let Some(ref area2) = map[x2][z2]
        {
//            println!("Connecting ({},{}) to ({},{}).", x1, z1, x2, z2);

            area1.add_connection(area2.get_coordinates());
        }
    }
}

fn connect_paths(x1: usize, z1: usize, x2: usize, z2: usize, map: &Map)
{
    if let Some(ref area1) = map[x1][z1]
    {
        if let Some(ref area2) = map[x2][z2]
        {
//            println!("Connecting ({},{}) to ({},{}).", x1, z1, x2, z2);
//            println!("Connecting ({},{}) to ({},{}).", x2, z2, x1, z1);
            area1.add_connection(area2.get_coordinates());
            area2.add_connection(area1.get_coordinates());
        }
    }
}

fn format_map(map: &Map) -> String
{
    let mut ret = String::new();

    for x in (0..map.len()).rev()
    {
        let z_axis = &map[x];

        for z in 0..z_axis.len()
        {
            let z_coord = &z_axis[z];

            match z_coord
            {
                Some(coord) => ret += format!("{}", coord.get_map_icon()).as_str(),
                None => ret += EMPTY_ROOM_PAT
//                Some(coord) => ret += format!("(X,X)",).as_str(),
//                None => ret += format!("({},{})", x, z).as_str()
            };
        }

        if x > 0 { ret += "\n"; }
    }

    ret
}

fn area_coords_match(x: usize, z: usize, coords: (usize, usize, usize)) -> bool
{
    x == coords.1 && z == coords.2
}