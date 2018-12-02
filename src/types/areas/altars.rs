use traits::{ Area, Entity };
use player_options::Response;
use player_data::PlayerMeta;
use types::effects::Effect;
use types::classes::Class;
use var_access;
use text;

use std::cell::RefCell;

#[derive(EntityHolder, AreaTools)]
pub struct Altar
{
    pub god_info: (&'static str, &'static str),
    entrance_message: String,
    pub area_title: String,
    pub area_num: usize,
    entities: RefCell<Vec<Box<Entity>>>,
    coordinates: (usize, usize, usize),
    connections: RefCell<Vec<(usize, usize, usize)>>
}

impl Altar
{
    pub fn new(class: Class, area_num: usize, coordinates: (usize, usize, usize)) -> Box<Area>
    {
        let god_info = text::rand_god_info(class);
        let entrance_message = format!("Monument to {}", god_info.0);

        Box::new(Altar
        {
            god_info,
            entrance_message,
            area_title: String::from("Altar"),
            area_num,
            coordinates,
            entities: RefCell::new(Vec::new()),
            connections: RefCell::new(Vec::new())
        })
    }

    fn god(&self) -> &str
    {
        self.god_info.0
    }

    fn god_desc(&self) -> &str
    {
        self.god_info.1
    }
}

impl Area for Altar
{
    fn get_type(&self) -> &'static str { "altar" }

    fn get_map_icon(&self) -> &'static str { " A " }

    fn get_entrance_message(&self) -> Option<String> { Some(self.entrance_message.clone()) }

    fn get_title(&self) -> String { self.area_title.clone() }

    fn get_info_for_player(&self, _player: &mut PlayerMeta) -> Option<String>
    {
        Some(format!(
            "The inscription upon the altar reads:\n\
            \"    Hallowed {},\n{}\"",
            self.god(),
            self.god_desc()
        ))
    }

    fn get_specials_for_player(&self, player: &mut PlayerMeta, responses: &mut Vec<Response>)
    {
        let num_uses = player.get_record(self.get_coordinates(), "num_uses");

        if num_uses != 0
        {
            responses.push(Response::text_only("You have already prayed here (do nothing)."));
            return;
        }

        if player.god == self.god()
        {
            responses.push(Response::simple("Pray to the god", | player_id |
            {
                var_access::access_player_context(player_id, | meta, _, area, entity |
                {
                    let blessing = Effect::positive_altar_effect();
                    blessing.apply(entity);

                    meta.incr_record(area.get_coordinates(), "num_uses");
                })
                .expect("Player data no longer exists.");
            }));
        }
        else
        {
            responses.push(Response::simple("Pray to the god", | player_id |
            {
                var_access::access_player_context(player_id, | meta, _, area, entity |
                {
                    let (blessing, curse) = Effect::normal_altar_effect();

                    blessing.apply(entity);
                    curse.apply(entity);

                    meta.incr_record(area.get_coordinates(), "num_uses");
                })
                .expect("Player data no longer exists.");
            }));
        }
    }
}