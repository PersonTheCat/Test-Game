use crate::player_data::PlayerMeta;
use crate::text;
use crate::traits::{Area, Entity};
use crate::types::classes::Class;
use crate::types::effects::Effect;
use crate::util::access;
use crate::util::player_options::Response;

use std::cell::RefCell;

#[derive(EntityHolder, AreaTools)]
pub struct Altar {
    pub god_info: (&'static str, &'static str),
    entrance_message: String,
    pub area_title: String,
    pub area_num: usize,
    entities: RefCell<Vec<Box<Entity>>>,
    coordinates: (usize, usize, usize),
    connections: RefCell<Vec<(usize, usize, usize)>>,
}

impl Altar {
    pub fn new(class: Class, area_num: usize, coordinates: (usize, usize, usize)) -> Box<Area> {
        let god_info = text::rand_god_info(class);
        let entrance_message = format!("Monument to {}", god_info.0);

        Box::new(Altar {
            god_info,
            entrance_message,
            area_title: String::from("Altar"),
            area_num,
            coordinates,
            entities: RefCell::new(Vec::new()),
            connections: RefCell::new(Vec::new()),
        })
    }

    fn god(&self) -> &str {
        self.god_info.0
    }

    fn god_desc(&self) -> &str {
        self.god_info.1
    }
}

impl Area for Altar {
    fn get_type(&self) -> &'static str {
        "altar"
    }

    fn get_map_icon(&self) -> &'static str {
        " A "
    }

    fn get_entrance_message(&self) -> Option<String> {
        Some(self.entrance_message.clone())
    }

    fn get_title(&self) -> String {
        self.area_title.clone()
    }

    fn get_dialogue_info(&self, _player: &PlayerMeta) -> Option<String> {
        Some(format!(
            "The inscription upon the altar reads:\n\
             \"    Hallowed {},\n{}\"",
            self.god(),
            self.god_desc()
        ))
    }

    fn get_specials(&self, player: &PlayerMeta, responses: &mut Vec<Response>) {
        let num_uses = player.get_record(self.get_coordinates(), "num_uses");

        if num_uses != 0 {
            responses.push(Response::text_only(
                "You have already prayed here (do nothing).",
            ));
            return;
        }

        if player.get_god() == self.god() {
            responses.push(Response::simple("Pray to the god", |player| {
                access::entity(player.get_accessor(), |entity| {
                    let blessing = Effect::positive_altar_effect();
                    blessing.apply(entity);

                    player.incr_record(player.get_coordinates(), "num_uses");
                })
                .expect("Player data no longer exists.");
            }));
        } else {
            responses.push(Response::simple("Pray to the god", |player| {
                access::entity(player.get_accessor(), |entity| {
                    let (blessing, curse) = Effect::normal_altar_effect();
                    blessing.apply(entity);
                    curse.apply(entity);

                    player.incr_record(player.get_coordinates(), "num_uses");
                })
                .expect("Player data no longer exists.");
            }));
        }
    }
}
