/**
 * This class was for testing a potential
 * redesign of Dialogue that avoids
 * almost all unnecessary boxing, with
 * the exception being that the Dialogue
 * itself has to be boxed. It is currently
 * in limbo, as what benefit it would bring
 * might be too small to matter.
 */

pub enum OptionResult
{
    Success,
    InvalidNumber,
    NoneFound,
    NoArgs
}

pub trait DialogueHolder
{
    fn run(&self, args: &String) -> OptionResult;

    fn run_as_user(&self, args: &String, player_id: usize) -> OptionResult;

    fn get_display(&self) -> String;

    fn get_id(&self) -> usize;

    //pub fn add_replacement_flag(&mut self) -> Dialogue {}
}

pub type BoxedDialogue = Box<DialogueHolder>;
pub type OptionalDialogue = Option<BoxedDialogue>;

pub struct TextHandler<F1, F2>
    where F1: Fn(&str), F2: Fn() -> OptionalDialogue
{
    pub text: String,
    pub execute: F1,
    pub next_dialogue: F2
}

pub fn test()
{
    let test = TextHandler
    {
        text: String::from(""),
        execute: | msg | { println!("I got: {}.", msg); },
        next_dialogue: || { None }
    };

    (test.execute)("success");
}

