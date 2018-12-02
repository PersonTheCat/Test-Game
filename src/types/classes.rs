extern crate rand;

use rand::{
    thread_rng,
    Rng
};

use std::fmt::{
    Display,
    Formatter,
    Result
};

use self::Class::*;

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum Class
{
    Melee,
    Ranged,
    Magic
}

impl Display for Class
{
    fn fmt(&self, f: &mut Formatter) -> Result
    {
        match self
        {
            Melee => write!(f, "Melee"),
            Ranged => write!(f, "Ranged"),
            Magic => write!(f, "Magic"),
        }
    }
}

pub fn random_class() -> Class
{
    match thread_rng().gen_range(0, 3)
    {
        0 => Melee,
        1 => Ranged,
        2 => Magic,
        _ => panic!("Error: Generated an impossible number.")
    }
}