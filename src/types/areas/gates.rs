use crate::player_data::PlayerMeta;
use crate::traits::{Area, Entity};
use crate::types::classes::Class;
use crate::util::access;
use crate::util::player_options::Response;

use parking_lot::RwLock;
use parking_lot::Mutex;

#[derive(EntityHolder, AreaTools)]
pub struct Gate {
    area_num: usize,
    coordinates: (usize, usize, usize),
    entities: RwLock<Vec<Box<Entity>>>,
    connections: Mutex<Vec<(usize, usize, usize)>>,
}

impl Gate {
    pub fn new(_class: Class, area_num: usize, coordinates: (usize, usize, usize)) -> Box<Area> {
        Box::new(Gate {
            area_num,
            coordinates,
            entities: RwLock::new(Vec::new()),
            connections: Mutex::new(Vec::new()),
        })
    }

    fn is_end_gate(&self) -> bool {
        self.coordinates.1 > 0
    }

    fn is_starting_town(&self) -> bool {
        self.coordinates.0 <= 1
    }
}

impl Area for Gate {
    fn get_type(&self) -> &'static str {
        "gate"
    }

    fn get_map_icon(&self) -> &'static str {
        "[G]"
    }

    /**
     * To-do: add variations.
     */
    fn get_entrance_message(&self) -> Option<String> {
        if self.is_end_gate() {
            Some(String::from(
                "§You notice that your path concludes at a familiar gate and \
                 begin to wonder if there is some sort of key.",
            ))
        } else if self.is_starting_town() {
            Some(String::from(
                "§As you gaze upon the sealed grounds that mark the beginning \
                 of your journey, you reflect upon your new life which has \
                 forever changed.",
            ))
        } else {
            Some(String::from(
                "§You arrive in front a tall, locked gate, wondering only \
                 if you can return from whence you came.",
            ))
        }
    }

    fn get_title(&self) -> String {
        if access::town(self.get_town_num()).unlocked() {
            String::from("Gate")
        } else {
            String::from("Locked Gate")
        }
    }

    fn get_specials(&self, _player: &PlayerMeta, responses: &mut Vec<Response>) {
        let current_area = self.coordinates;

        if self.is_end_gate() {
            let next_town = self.get_town_num() + 1;

            responses.push(Response::goto_dialogue(
                "Test going to the next area",
                move |player| {
                    access::area(current_area, |old_area| {
                        access::starting_area(next_town, |new_area| {
                            old_area.transfer_to_area(player.get_player_id(), new_area);
                            new_area.get_dialogue(player)
                        })
                    })
                    .expect("The player's current area could not be relocated.")
                },
            ))
        } else if !self.is_starting_town() {
            let previous_town = self.get_town_num() - 1;

            responses.push(Response::goto_dialogue(
                "Test going to the previous area",
                move |player| {
                    access::area(current_area, |old_area| {
                        let town = access::town(previous_town);
                        access::area(town.end_gate(), |new_area| {
                            old_area.transfer_to_area(player.get_player_id(), new_area);
                            new_area.get_dialogue(player)
                        })
                        .expect("Invalid town # or gate coordinates.")
                    })
                    .expect("The player's current area could not be relocated.")
                },
            ))
        }
    }
}
