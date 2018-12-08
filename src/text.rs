use crate::util::player_options::{ Response, Dialogue, TextHandler };
use crate::types::classes::Class::{self, *};
use crate::types::entities::players::Player;
use crate::traits::Entity;
use crate::util::access;
use crate::*;

use rand::{ thread_rng, Rng };

/**
 * This class is for holding a bunch of miscellaneous
 * dialogue to keep it away from the code inside of
 * other classes. For smaller classes, it may be
 * cleaner just to keep all related text and code in
 * one location.
 *
 * ## Text formatting info:
 *
 * The `§` character will handle automatically inserting
 * new lines, whereas the `∫` character splits the text up
 * and sends it to the user in sections. `∫` can be followed
 * by a floating point number to indicate the speed with
 * which each next section should come. This is a
 * multiplier which defaults to 1.00. In some cases,
 * new lines have been manually inserted even when using
 * `§`. This is because using `∫` to send text in sections
 * results in messages that are difficult to keep up with.
 * Thus, an extra `\n` will improve readability for the
 * user.
 */

/** To-do: Create and move this to common_functions.rs */
pub fn choose<T>(a: &[T]) -> &T
{
    thread_rng().choose(a)
        .expect("You need to use thread_rng().choose() for arrays where len < 1.")
}

pub fn auto_break(text: &str) -> String
{
    let mut chars: Vec<char> = text.chars().collect();
    if chars.len() <= LINE_LENGTH { return text.to_string(); }

    let mut start_at = 0;
    while start_at <= chars.len() - LINE_LENGTH
    {
        let end = end_of_line(start_at, &chars);
        chars[end] = '\n';
        start_at = end + 1;
    }
    chars.into_iter().collect()
}

fn end_of_line(start_at: usize, text: &Vec<char>) -> usize
{
    let max_line = &text[start_at..(start_at + LINE_LENGTH)];
    let mut final_space = max_line.len();

    //Iterate forward in case of an early new line.
    for (i, c) in max_line.iter().enumerate()
    {
        match *c
        {
            '\n' => return start_at + i,
            ' ' => final_space = i,
            _ => {}
        };
    }
    start_at + final_space
}

fn get_last_char(ch: char, text: &[char]) -> usize
{
    for (i, c) in text.iter().enumerate().rev()
    {
        if *c == ch { return i; }
    }
    text.len()
}

///** Faster; does not support special characters. */
//pub fn auto_break_old(text: &str) -> String
//{
//    let mut ret = text.to_string();
//    if ret.len() <= ::LINE_LENGTH { return ret; }
//
//    let mut start_at = 0;
//    while start_at < text.len() - ::LINE_LENGTH
//    {
//        let final_space = end_of_line(start_at,&ret);
//        ret.insert(final_space + 1, '\n');
//        start_at += final_space;
//    }
//    ret
//}
//
//fn end_of_line(start_at: usize, text: &str) -> usize
//{
//    let line = &text[start_at..(start_at + ::LINE_LENGTH)];
//    get_last_char(' ', line)
//}
//
//fn get_last_char(ch: char, text: &str) -> usize
//{
//    let chars = text
//        .char_indices()
//        .rev();
//
//    for (i, c) in chars
//    {
//        if c == ch { return i; }
//    }
//    text.len()
//}

pub fn apply_replacements(text: &str, replacements: &[(&str, String)]) -> String
{
    let mut ret = text.to_string();

    for (find, replace) in replacements
    {
        ret = ret.replace(find, replace);
    }
    ret
}

pub fn convert_to_vec(array: &[&str]) -> Vec<String>
{
    let mut ret = Vec::new();

    for text in array
    {
        ret.push(text.to_string());
    }
    ret
}

fn tuples_to_vec(tuples: &[(&str, &str)]) -> Vec<String>
{
    let mut ret = Vec::new();

    for (name, _description) in tuples
    {
        ret.push(name.to_string());
    }
    ret
}

/**
 * ***************
 *      Gods
 * ***************
 */

pub const CELTIC_GODS: [(&str, &str); 6] =
[
    ("Danu", "Matriarch of the Tuatha Dé Danann;\ncaretaker of the Earth."),
    ("Ogma", "God of eloquence and learning, master of\nspeech and language."),
    ("Epona", "Goddess of fertility, maternity, protector\nof horses, horse breeding, prosperity, dogs,\nhealing springs, crops."),
    ("Arwan", "god of the underground; kingdom of the dead.\nEvoker of revenge, terror, and war."),
    ("Scathach", "Goddess of shadows and destruction,\npatroness of blacksmiths, healing, magic, prophecy,\nand martial arts."),
    ("Merlin", "The great sorcerer, druid, and magician;\nmaster of illusion, shape-shifting, healing,\nnature, and counseling.")
];

pub const CELTIC_GODS_WHO_FLY: [&str; 4] =
[
    "Latobius", "Taranis", "Brigid", "Nuada"
];

pub const OTHER_GODS_WHO_FLY: [&str; 1] =
[
    "Horagalles"
];

pub const HINDU_GODS: [(&str, &str); 6] =
[
    ("Durga", "Goddess of victory; bane of evil."),
    ("Kali", "Goddess of time, creation, destruction, and power; mother of the universe and bestower of moksha."), // Mounts a fox.
    ("Ganesha", "Please provide text."), // Mounts a deer
    ("Vishnu", "I'm gonna need some text."),
    ("Surya", "How's about some text for this guy?"),
    ("Krishna", "Yo, shoot me some info on this guy.")
];

pub const BABYLONIAN_GODS: [(&str, &str); 2] =
[
    ("Ereshkigal", "Queen of the underworld and lady of\nthe great below."),
    ("Gilgamesh", "Someone please tell me something\ninteresting about Gilgamesh.")
];

/**
 * There are a lot of redundant functions here.
 * I'll decide which ones to keep later on.
 */

pub fn gods_for_class(class: Class) -> Vec<String>
{
    match class
    {
        Melee => tuples_to_vec(&BABYLONIAN_GODS),
        Ranged => tuples_to_vec(&CELTIC_GODS),
        Magic => tuples_to_vec(&HINDU_GODS)
    }
}

pub fn rand_god(class: Class) -> &'static str { rand_god_info(class).0 }

pub fn rand_god_info(class: Class) -> (&'static str, &'static str)
{
    match class
    {
        Melee => rand_babylonian_god_info(),
        Ranged => rand_celtic_god_info(),
        Magic => rand_hindu_god_info()
    }
}

pub fn rand_celtic_god() -> &'static str
{
    choose(&CELTIC_GODS).0
}

pub fn rand_celtic_god_info() -> (&'static str, &'static str)
{
    *choose(&CELTIC_GODS)
}

pub fn rand_hindu_god() -> &'static str
{
    choose(&HINDU_GODS).0
}

pub fn rand_hindu_god_info() -> (&'static str, &'static str)
{
    *choose(&HINDU_GODS)
}

pub fn rand_babylonian_god() -> &'static str
{
    choose(&BABYLONIAN_GODS).0
}

pub fn rand_babylonian_god_info() -> (&'static str, &'static str)
{
    *choose(&BABYLONIAN_GODS)
}

pub fn get_info_for_god(god: &str, class: Class) -> &'static str
{
    let gods: &[(&'static str, &'static str)] = match class
    {
        Melee => &BABYLONIAN_GODS,
        Ranged => &CELTIC_GODS,
        Magic => &HINDU_GODS
    };

    for (god2, info) in gods
    {
        if god == *god2
        {
            return *info;
        }
    }
    ""
}

const SAME_GOD: [&str; 5] =
[
    "§What's that? You also worship <god>? I might have something else to show you.",
    "§What's that? You also worship <god>? Maybe there's something else I can do for you...",
    "§I see you're a follower of <god>. Praise be. Let me help you with something good.",
    "§I see you've found light in the path of <god>. Let us share in this blessing.",
    "§Ahh. Another acolyte of <god>, greatness be. Let us share in this blessing."
];

pub fn generic_same_god_message(name: &String, god: &String) -> String
{
    let text = choose(&SAME_GOD);

    let mut ret = String::new();
    ret += name;
    ret += ": ";

    let body = apply_replacements(&text.to_string(), &vec![("<god>", god.clone())]);

    ret += &body;
    ret
}

pub const DONATION_REJECTED: [&str; 3] =
[
    "The gods accept your offering, but do not\n\
    believe in your faith.",
    "The gods smile upon you, but expect further\n\
    praise on your behalf.",
    "The gods welcome your sacrifice, but still\n\
    question your devotion."
];

pub fn rand_donation_rejected() -> &'static str
{
    *choose(&DONATION_REJECTED)
}

/**
 * ****************
 *      Towns
 * ****************
 */

pub const PATH_ADJECTIVES: [&str; 7] =
[
    "Overgrown", "Worn", "Fragile",
    "Lone", "Wallowing", "Mysterious",
    "Tumbling"
];

pub const PATH_NOUNS: [&str; 6] =
[
    "Forest", "Groves", "Forest",
    "Stones", "Fallow", "Elm"
];

pub fn rand_path_name() -> String
{
    let adj = choose(&PATH_ADJECTIVES);
    let noun = choose(&PATH_NOUNS);

    format!("{} {}", adj, noun)
}

/**
 * ****************
 *      Stores
 * ****************
 */

pub const PUB_KEEPER_TITLES: [&str; 0] =
[

];

pub const BLACKSMITH_KEEPER_TITLES: [&str; 0] =
[

];

pub const ARMORY_KEEPER_TITLES: [&str; 0] =
[

];

pub const WEAPONSMITH_KEEPER_TITLES: [&str; 0] =
[

];

pub const CHURCH_KEEPER_TITLES: [&str; 0] =
[

];

pub const TICKETSTATION_KEEPER_TITLES: [&str; 0] =
[

];

/**
 * ***************
 *      NPCs
 * ***************
 */

const MALE: u8 = 0;
const FEMALE: u8 = 1;
const UNKNOWN: u8 = 2;
const CREATURE: u8 = 3;

pub const NPC_NAMES_FEMALE: [&str; 11] =
[
    "Naomi", "Mable", "Beatrice",
    "Ava", "Nora", "Cait",
    "Cara", "Bláthnaid", "Aoife",
    "Alannah", "Máire"
];

pub const NPC_NAMES_MALE: [&str; 11] =
[
    "Silas", "Oliver", "Alexander",
    "James", "Edgar", "Enoch",
    "Elijah", "Eli", "Lemuel",
    "Arthur", "Abe"
];

pub const NPC_NAMES_NEUTRAL: [&str; 11] =
[
    "Per", "Nils", "Freja",
    "Elias", "Jere", "Aatami",
    "Kai", "Kyösti", "Deva",
    "Khursh", "Sutra"
];

pub const NPC_NAMES_CREATURE: [&str; 11] =
[
    "Havarr", "Sindri", "Cyrus",
    "Emil", "Dyra", "Kolli",
    "Ása", "Eydís", "Aurora",
    "Draven", "Reznor"
];

pub const NPC_DESCRIPTIONS_FEMALE: [&str; 9] =
[
    "elegant lady",
    "elderly woman",
    "enigmatic caricature of a female",
    "somewhat bitter broad",
    "restless dame",
    "dark mistress",
    "tall, graceful mistress",
    "confident and powerful woman",
    "peaceful townswoman"
];

pub const NPC_DESCRIPTIONS_MALE: [&str; 9] =
[
    "strange, towering man",
    "fairly tall gentleman",
    "young swain",
    "suspiciously quiet man",
    "very taciturn fellow",
    "rather well-kept, proud gentleman",
    "strange old man",
    "tranquil townsman",
    "nice old man"
];

pub const NPC_DESCRIPTIONS_NEUTRAL: [&str; 9] =
[
    "ordinary citizen",
    "fair singleton",
    "surprisingly radiant character",
    "well-kept denizen",
    "friendly commoner",
    "seemingly important civilian",
    "fairly short individual",
    "nearby occupant",
    "fellow voyager"
];

pub const NPC_DESCRIPTIONS_CREATURE: [&str; 9] =
[
    "tall, dark entity",
    "rather unpleasant figure",
    "disfigured creature",
    "mysterious humanoid figure",
    "tall, menacing beast",
    "stunningly vibrant being",
    "ghastly personage",
    "somewhat shocking brute",
    "swift, graceful character"
];

pub fn rand_npc_name() -> String // This usually needs to be owned.
{
    let slice = choose(&[NPC_NAMES_FEMALE, NPC_NAMES_MALE]);

    choose(slice).to_string()
}

pub fn rand_npc_details() -> (&'static str, &'static str)
{
    let (name, description);

    match thread_rng().gen_range(MALE, CREATURE + 1)
    {
        MALE =>
        {
            name = choose(&NPC_NAMES_MALE);
            description = choose(&NPC_DESCRIPTIONS_MALE);
        },
        FEMALE =>
        {
            name = choose(&NPC_NAMES_FEMALE);
            description = choose(&NPC_DESCRIPTIONS_FEMALE);
        },
        UNKNOWN =>
        {
            name = choose(&NPC_NAMES_NEUTRAL);
            description = choose(&NPC_DESCRIPTIONS_NEUTRAL);
        },
        _ =>
        {
            name = choose(&NPC_NAMES_CREATURE);
            description = choose(&NPC_DESCRIPTIONS_CREATURE);
        }
    };
    (name, description)
}

/**
 * ***************
 *     Dialogue
 * ***************
 */

pub const NEW_SENDER: [&str; 5] =
[
    "§Hello? Is there someone there? ∫0.2.∫0.2.∫0.2.∫0.5\n\
    Oh, yes. That's right. I've been expecting you.∫\n\
    Go ahead and sit down. We have much to discuss. But \
    before we begin, please, remind me your name.",

    "§Hello? Who's there? ∫0.2.∫0.2.∫0.2.∫0.5\n\
    Ah, yes. Very good. I was hoping you would find me here.∫\n\
    Please, sit down. I have big plans for us to discus, but \
    before we can get started, do remind me your name.",

    "§What's that? Is someone there? ∫0.2.∫0.2.∫0.2.∫0.5\n\
    Ah, I see. Good day. I'm glad you found me here.∫\n\
    If you don't mind, please have a seat. I can't wait to \
    share my plans with you. Now, before we begin, \
    please remind me your name.",

    "§Hello? Is someone there? ∫0.2.∫0.2.∫0.2.∫0.5\n\
    Ah, I see. I'm glad to see you made it all the way here.∫\n\
    If you don't mind, you should go ahead and sit down. \
    This might take us a few minutes.∫\n\
    Now, then. Let me just ask... Who do you think you are?",

    "§What's that? Who goes there? ∫0.2.∫0.2.∫0.2.∫0.5\n\
    Ah. Good day, there. I'm glad to see you arrived safely.∫\n\
    Now, please, do have a seat. This won't take long.∫\n\
    Let me start by asking: who exactly do you think you are?"
];

pub fn rand_new_sender() -> &'static str
{
    *choose(&NEW_SENDER)
}

pub fn new_player_name(player_id: usize) -> Dialogue
{
    let text_handler = TextHandler
    {
        text: String::from("Enter your name:"),
        execute: Box::new(move | args: &str |
        {
            access::player_meta(player_id, | meta |
            {
                meta.name = args.to_string()
            });
        }),
        next_dialogue: gen_dialogue(move || { new_player_name_confirm(player_id, 0) })
    };

    Dialogue::handle_text
    (
        String::from("New Player"),
        &Vec::new(),
        Vec::new(),
        text_handler,
        player_id
    )
}

pub const NAME_LEARNED: [&str; 5] =
[
    "\"<name>.\" Is that right?",
    "\"<name>,\" you say. Is that so?",
    "You say your name is \"<name>?\"",
    "Ohh, I see. \"<name>,\" you say?",
    "So, \"<name>\" is what you remember?"
];

pub fn new_player_name_confirm(player_id: usize, num_corrections: u8) -> Dialogue
{
    let confirm = Response::goto_dialogue("Confirm", move ||
    {
        new_player_class(player_id)
    });

    let different_name = TextHandler
    {
        text: String::from("Enter a different name:"),
        execute: Box::new(move | input: &str |
        {
            access::player_meta(player_id, |meta |
            {
                if num_corrections > 0
                {
                    meta.name = rand_npc_name()
                }
                else { meta.name = input.to_string(); }
            });
        }),
        next_dialogue: Generate(Box::new(move ||
        {
            if num_corrections > 0
            {
                new_player_class(player_id)
            }
            else { new_player_name_confirm(player_id, num_corrections + 1) }
        }))
    };

    let manual_substitutions = vec![("<name>", get_entered_name(player_id))];

    Dialogue::new
    (
        String::from("New Player"),
        &Vec::new(),
        Vec::new(),
        Some(apply_replacements(&choose(&NAME_LEARNED).to_string(), &manual_substitutions)),
        vec![confirm],
        Vec::new(),
        Some(different_name),
        player_id
    )
}

fn get_entered_name(player_id: usize) -> String
{
    access::player_meta(player_id, |meta |
    {
        meta.name.clone()
    })
    .expect("Player data no longer exists.")
}

pub fn new_player_class(player_id: usize) -> Dialogue
{
    let mut responses = Vec::new();

    lazy_static!
    {
        static ref text: Vec<&'static str> = vec!
        [
            "§Ahh, yes. \"<name>.\" I remember it well, but \
            you see, it's been a long time.∫\n\
            After all these years, you may have forgotten \
            my face, but yours is not one I could forget.∫\n\
            Now, <name>, I want you to tell me: \
            What is it that defines you?",

            "§Ahh, that's right. \"<name>.\" As I expected. \
            There was a time when we knew each other so well.∫\n\
            But, after all these years, I suppose some memories \
            fade. You may have forgotten me, <name>, \
            but I still know who you are.∫\n\
            Now, the only question is: do you?",

            "§Very well. You are just as I remember, <name>. \
            You see, there was a time when we knew each other \
            so well.∫\n\
            But, I suppose some memories do fade. It's curious, \
            <name>, seeing as you've changed so much and yet you \
            are completely unaware.∫\n\
            I want you to think, <name>. Tell me what it is that \
            defines you.",

            "§I see, then. \"<name>.\" Very well. \
            It's a shame to discover just how much you've forgotten. \
            There was a time when we knew each other so well \
            but some memories just don't last.∫\n\
            Let us try an exercise, <name>. I want you to try and \
            think about what it is that makes you who you are."
        ];
    }

    responses.push(Response
    {
        text: String::from("Melee"),
        execute: Some(Box::new(move | player |
        {
            access::player_meta(player, |meta |
            {
                meta.class = Melee;
            });
        })),
        next_dialogue: gen_dialogue(move || { new_player_god(player_id, Melee) })
    });

    responses.push(Response
    {
        text: String::from("Ranged"),
        execute: Some(Box::new(move | player |
        {
            access::player_meta(player, |meta |
            {
                meta.class = Ranged;
            });
        })),
        next_dialogue: gen_dialogue(move || { new_player_god(player_id, Ranged) })
    });

    responses.push(Response
    {
        text: String::from("Magic"),
        execute: Some(Box::new(move | player |
        {
            access::player_meta(player, |meta |
            {
                meta.class = Magic;
            });
        })),
        next_dialogue: gen_dialogue(move || { new_player_god(player_id, Magic) })
    });

    Dialogue::new
    (
        String::from("New Player"),
        &text,
        vec![("<name>", get_entered_name(player_id))],
        Some(String::from("Choose a class:")),
        responses,
        Vec::new(),
        None,
        player_id
    )
}

pub fn new_player_god(player_id: usize, class: Class) -> Dialogue
{
    lazy_static!
    {
        static ref melee: Vec<&'static str> = vec!
        [
            "Ahh, yes of course. A warrior; one\n\
            who acts with courage and vigilance.",
            "A master of face to face combat and\n\
            purveyor of blades. A true warrior."
        ];
        static ref ranged: Vec<&'static str> = vec!
        [
            "Ahh, of course. An archer; one who\n\
            calculates his actions at range and\n\
            thrives on stealth.",
            "A master of stealth and ranged combat\n\
            A true archer."
        ];
        static ref magic: Vec<&'static str> = vec!
        [
            "Yes, of course. A mage; conductor of\n\
            darkness and conjurer of the mysterious.",
            "A master of illusions and evoker of\n\
            the mysterious. A true wizard.",
            "An illusionist and conjurer of the\n\
            the mysterious. A veritable wizard."
        ];
    }

    let text: &Vec<&'static str> = match class
    {
        Melee => &melee,
        Ranged => &ranged,
        Magic => &magic
    };

    let mut responses = Vec::new();

    for god in gods_for_class(class)
    {
        responses.push(Response
        {
            text: god.clone(),
            execute: Some(Box::new(move |player: usize |
            {
                access::player_meta(player, |meta |
                {
                    meta.god = god.clone();
                });
            })),
            next_dialogue: gen_dialogue(move || { new_player_ready(player_id) })
        });
    }

    Dialogue::new
    (
        String::from("New Player"),
        text,
        Vec::new(),
        Some(format!("Choose a god from the {} class:", class)),
        responses,
        Vec::new(),
        None,
        player_id
    )
}

fn new_player_ready(player_id: usize) -> Dialogue
{
    let (god, class) =
    access::player_meta(player_id, |player |
    {
        (player.god.clone(), player.class)
    })
    .expect("Player data no longer exists.");

    let info = get_info_for_god(&god, class);

    let responses = vec!
    [
        Response::goto_dialogue("Start game.", move ||
        {
            new_player_finished(player_id)
        }),
    ];

    Dialogue::simple
    (
        String::from("New Player"),
        &vec![info],
        Vec::new(),
        responses,
        player_id
    )
}

fn new_player_finished(player_id: usize) -> Dialogue
{
    let rand_starting_town = thread_rng().gen_range(1, 4);

    access::starting_area(rand_starting_town, move |area |
    {
        let name = access::player_meta(player_id, |player |
        {
            player.name.clone()
        })
        .expect("Player data no longer exists.");

        let player = Box::new(Player::new(player_id, name));
        player.give_money(1000);

        area.add_entity(player);
        area.get_dialogue(player_id)
    })
}