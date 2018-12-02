use types::entities::players::Player;
use types::effects::{ Effect, EffectType::* };
use player_options::Response;
use player_data::PlayerMeta;
use traits::{ Area, Entity };
use types::classes::Class;
use var_access;
use text;

use std::cell::RefCell;

use rand::random;

#[derive(EntityHolder, AreaTools)]
pub struct Fountain
{
    entrance_message: String,
    pub area_title: String,
    pub area_num: usize,
    entities: RefCell<Vec<Box<Entity>>>,
    coordinates: (usize, usize, usize),
    connections: RefCell<Vec<(usize, usize, usize)>>
}

impl Fountain
{
    pub fn new(_class: Class, area_num: usize, coordinates: (usize, usize, usize)) -> Box<Area>
    {
        Box::new(Fountain
        {
            entrance_message: String::from("Welcome to the test fountain."),
            area_title: String::from("Fountain"),
            area_num,
            coordinates,
            entities: RefCell::new(Vec::new()),
            connections: RefCell::new(Vec::new())
        })
    }
}

impl Area for Fountain
{
    fn get_type(&self) -> &'static str { "fountain" }

    fn get_map_icon(&self) -> &'static str { "[F]" }

    fn can_enter(&self, _player: &Player) -> bool { true }

    fn get_entrance_message(&self) -> Option<String> { Some(self.entrance_message.clone()) }

    fn get_title(&self) -> String { self.area_title.clone() }

    fn get_specials_for_player(&self, player: &mut PlayerMeta, responses: &mut Vec<Response>)
    {
        let coords = self.get_coordinates();
        let successful_donations = player.get_record(coords, "successful_donations");

        if successful_donations == 0
        {
            let num_donations = player.get_record(coords, "num_donations");
            let price = get_price(num_donations, self.get_town_num());
            let text = format!("Throw a coin into the fountain ({}g).", price);

            responses.push(Response::_simple(text, move | player_id |
            {
                var_access::access_player_context(player_id, | meta, town, _, entity |
                {
                    if entity.can_afford(price)
                    {
                        meta.incr_record(coords, "num_donations");
                        entity.take_money(price);

                        if random()
                        {
                            meta.incr_record(coords, "successful_donations");

                            let effect = Effect::get_fountain_effect(town.town_num);
                            effect.apply(entity);

                            if let Temporary(duration) = effect.effect_type
                            {
                                ::add_short_message(player_id, &format!("The gods have blessed you with\n{} {} for {} seconds.", effect.name, effect.level, duration / 1000));
                            }
                            else { ::add_short_message(player_id, &format!("The gods have blessed you with\n{} {}.", effect.name, effect.level)); }
                        }
                        else { ::add_short_message(player_id, text::rand_donation_rejected()); }
                    }
                    else { ::add_short_message(player_id, "You can't afford this offering."); }
                });
            }));
        }
        else { responses.push(Response::text_only("The gods have already spoken in your\nfavor (do nothing).")); }
    }
}

const BASE_PRICE: u32 = 10;
const LEVEL_RATE: f32 = 7.5;

fn get_price(num_donations: u8, level_num: usize) -> u32
{
    (BASE_PRICE + (level_num as f32 * LEVEL_RATE) as u32) * (num_donations as u32 + 1)
}