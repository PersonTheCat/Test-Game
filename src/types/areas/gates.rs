use player_options::Response;
use traits::{ Area, Entity };
use player_data::PlayerMeta;
use types::classes::Class;
use var_access;

use std::cell::RefCell;

#[derive(EntityHolder, AreaTools)]
pub struct Gate
{
    area_num: usize,
    coordinates: (usize, usize, usize),
    entities: RefCell<Vec<Box<Entity>>>,
    connections: RefCell<Vec<(usize, usize, usize)>>
}

impl Gate
{
    pub fn new(_class: Class, area_num: usize, coordinates: (usize, usize, usize)) -> Box<Area>
    {
        Box::new(Gate
        {
            area_num,
            coordinates,
            entities: RefCell::new(Vec::new()),
            connections: RefCell::new(Vec::new())
        })
    }

    fn is_end_gate(&self) -> bool { self.coordinates.1 > 0 }

    fn is_starting_town(&self) -> bool { self.coordinates.0 <= 1 }
}

impl Area for Gate
{
    fn get_type(&self) -> &'static str { "gate" }

    fn get_map_icon(&self) -> &'static str { "[G]" }

    /**
     * To-do: add variations.
     */
    fn get_entrance_message(&self) -> Option<String>
    {
        if self.is_end_gate()
        {
            Some(String::from(
                "You notice that your path concludes at a familiar gate and\n\
                 begin to wonder if there is some sort of key."))
        }
        else if self.is_starting_town()
        {
            Some(String::from(
                "As you gaze upon the sealed grounds that mark the beginning\n\
                of your journey, you reflect upon your new life which has\n\
                forever changed."))
        }
        else { Some(String::from(
            "You arrive in front a tall, locked gate, wondering only\n\
            if you can return from whence you came.")) }
    }

    fn get_title(&self) -> String
    {
        if var_access::access_town(self.get_town_num(), | t | t.unlocked() )
        {
            String::from("Gate")
        }
        else { String::from("Locked Gate") }
    }

    fn get_specials_for_player(&self, player: &mut PlayerMeta, responses: &mut Vec<Response>)
    {
        let player_id = player.player_id;
        let current_area = self.coordinates;

        if self.is_end_gate()
        {
            let next_town = self.get_town_num() + 1;

            responses.push(Response::goto_dialogue("Test going to the next area", move ||
            {
                var_access::access_area(current_area, | old_area |
                {
                    var_access::access_starting_area(next_town, | new_area |
                    {
                        old_area.transfer_to_area(player_id, new_area);
                        new_area.get_dialogue_for_player(player_id)
                    })
                })
                .expect("The player's current area could not be relocated.")
            }))
        }
        else if !self.is_starting_town()
        {
            let previous_town = self.get_town_num() - 1;

            responses.push(Response::goto_dialogue("Test going to the previous area", move ||
            {
                var_access::access_area(current_area, | old_area |
                {
                    var_access::access_town(previous_town, | town |
                    {
                        var_access::access_area(town.end_gate(), | new_area |
                        {
                            old_area.transfer_to_area(player_id, new_area);
                            new_area.get_dialogue_for_player(player_id)
                        })
                        .expect("Invalid town # or gate coordinates.")
                    })
                })
                .expect("The player's current area could not be relocated.")
            }))
        }
    }
}