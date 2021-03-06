use crate::util::timed_events::{DelayHandler, DelayedEvent};
use crate::player_data::PLAYER_META;
use crate::util::access;
use crate::text;
use crate::*;

#[cfg(feature = "discord")]
use crate::util::discord_bot;
#[cfg(feature = "remote_clients")]
use crate::util::server_host;
#[cfg(feature = "discord")]
use serenity::model::id::{ChannelId, UserId};

use lazy_static::lazy_static;
use regex::Regex;

use std::io::{self, Write};

use self::ChannelInfo::*;

pub fn temp_send_message_to_player(id: usize, typ: MessageComponent, msg: &str) -> DelayHandler {
    access::player_meta(id).send_message(typ, msg)
}

pub fn temp_update_player_message(id: usize, typ: MessageComponent, msg: &str) {
    access::player_meta(id).update_message(typ, msg);
}

pub fn temp_send_blocking_message(id: usize, msg: &str) -> DelayHandler {
    access::player_meta(id).send_blocking_message(msg)
}

pub fn temp_add_short_message(id: usize, msg: &str) {
    access::player_meta(id).add_short_message(msg);
}

pub fn temp_send_short_message(id: usize, msg: &str) {
    access::player_meta(id).send_short_message(msg)
}

/// A function used for sending messages to all players.
pub fn send_global_message(msg: &str) {
    for player in PLAYER_META.lock().iter() {
        player.send_short_message(msg);
    }
}

pub fn send_message_to_channel(channel: &ChannelInfo, message: &mut ReusableMessage, ms_speed: u64) -> DelayHandler {
    separate_messages(channel);

    if ms_speed == 0 {
        return single_message(channel, message);
    }

    lazy_static! {
        static ref speed_pattern: Regex = Regex::new(r"^(\d{1,2}(\.\d{1,2})?)?").unwrap();
    }

    let mut delay_ms = 0;
    let general = message.get_general();

    if general.len() > 0 {
        let mut iter = general.split("∫");

        schedule_message(channel, &iter.next().unwrap().to_string(), delay_ms);

        for mut part in iter {
            let find = speed_pattern.find(part);
            let mut multiplier: f32 = 1.0;

            if let Some(ref mat) = find {
                let num = mat.end();

                multiplier = part[0..num].parse().unwrap_or(1.0);
                part = &part[num..];
            }
            delay_ms += (ms_speed as f32 * multiplier) as u64;
            schedule_message(channel, &part.to_string(), delay_ms);
        }
    }

    let mut main_info = String::new();
    correct_server_spacing(channel, &mut main_info);
    if message.options.len() > 0 {
        main_info += &message.options;
    }
    if message.health_bar.len() > 0 {
        main_info += "\n"; //To-do: Handle this better.
        main_info += &message.health_bar;
    }
    if main_info.len() > 0 {
        correct_server_spacing(channel, &mut main_info);
        main_info += "\n";
        delay_ms += ms_speed;
        schedule_message(channel, &main_info, delay_ms);
    }

    DelayHandler::new(delay_ms)
}

#[cfg(feature = "remote_clients")]
fn correct_server_spacing(channel: &ChannelInfo, msg: &mut String) {
    if let Remote(_) = channel {
        *msg += "\n";
    }
}

#[cfg(not(feature = "remote_clients"))]
fn correct_server_spacing(_channel: &ChannelInfo, _msg: &mut String) {}

fn single_message(channel: &ChannelInfo, message: &ReusableMessage) -> DelayHandler {
    match channel {
        Local => println!("{}", message.format()),
        #[cfg(feature = "remote_clients")]
        Remote(ref username) => {
            server_host::send_message_to_client(username, &(message.format() + "\n\n"));
        }
        // Calls a rudimentary function that just
        // determines whether to edit a previous
        // message or send a new one.
        #[cfg(feature = "discord")]
        Discord(channel_id, user_id) => {
            discord_bot::handle_discord_message(channel_id, user_id, &message.format());
        }
    };
    DelayHandler::new(0)
}

// Same as single message, but uses DelayedEvents.
fn schedule_message(channel: &ChannelInfo, message: &str, delay_ms: u64) {
    let owned = message.to_string();

    match channel {
        // Manually flush the output to allow for
        // better control over message formatting.
        Local => {
            DelayedEvent::no_flags(delay_ms, move || {
                io::stdout().write(owned.as_bytes()).unwrap();
                io::stdout().flush().unwrap();
            });
        }
        #[cfg(feature = "remote_clients")]
        Remote(ref username) => {
            let user_owned = username.clone();
            DelayedEvent::no_flags(delay_ms, move || {
                server_host::send_message_to_client(&user_owned, &owned);
            });
        }
        #[cfg(feature = "discord")]
        Discord(channel_id, user_id) => {
            DelayedEvent::no_flags(delay_ms, move || {
                discord_bot::handle_discord_message(channel_id, user_id, &owned);
            });
        }
    };
}

// Only print one string. Terminal animations make
// these print lines distractingly visible.
fn separate_messages(channel: &ChannelInfo) {
    match channel {
        // Manually print a bunch of lines until / unless
        // a terminal client is integrated.
        Local => {
            let mut print = String::new();
            for _ in 0..NUM_SPACES {
                print += "\n";
            }
            println!("{}", print);
        },
        // Handle remote users in the same way as local
        // users, but pass their info through the host.
        #[cfg(feature = "remote_clients")]
        Remote(ref username) => {
            let mut print = String::new();
            for _ in 0..NUM_SPACES {
                print += "\n";
            }
            server_host::send_message_to_client(username, &print);
        },
        // Find and delete the most recent message
        // if it was sent by the bot.
        #[cfg(feature = "discord")]
        Discord(channel_id, _) => {
            if let Ok(ref mut messages) = channel_id.messages(|get| get.most_recent()) {
                if messages.len() == 0 {
                    return;
                }
                if !messages[0].author.bot {
                    return;
                }
                if let Err(_) = messages[0].delete() { /* ignore */ }
            }
        }
    };
}

// Misleading: not actually reusable.
// Needs to be stored in mutable space.
pub struct ReusableMessage {
    pub health_bar: String,
    pub general: Vec<String>,
    pub options: String,
    pub last_input: String, //Not ready for use.
}

impl ReusableMessage {
    pub fn new() -> ReusableMessage {
        ReusableMessage {
            health_bar: String::new(),
            general: Vec::new(),
            options: String::new(),
            last_input: String::new(),
        }
    }

    pub fn set_general(&mut self, length: usize, message: &str) {
        self.general.clear();
        let fmt = if message.starts_with("§") {
            indent_general(&text::auto_break(0, length, &message[2..]))
        } else {
            indent_general(message)
        };
        self.general.push(fmt);
    }

    pub fn get_general(&self) -> String {
        let mut ret = String::new();

        for msg in self.general.iter() {
            ret += msg;
        }
        ret
    }

    pub fn add_to_general(&mut self, length: usize, mut message: String) {
        // This might be redundant. Need to verify.
        if message.starts_with("§") {
            message = text::auto_break(0, length, &message[2..]);
        }

        if self.general.len() > 0 {
            if self.general[0].starts_with(">") {
                self.general.clear();
            }
        }
        if self.general.len() >= MAX_SHORT_MESSAGES {
            self.general.remove(0);
        }
        self.general.push(message);
    }

    pub fn format(&self) -> String {
        lazy_static! {
            static ref full_speed_pattern: Regex =
                Regex::new(r"∫(\d{1,2}(\.\d{1,2})?)?").unwrap();
        }

        let mut ret = String::new();

        let general = self.get_general();

        if general.len() > 0 {
            ret += &general;
        }
        if self.options.len() > 0 {
            ret += &self.options;
            ret += "\n"; //To-do: Handle this better.
        }
        if self.health_bar.len() > 0 {
            ret += &self.health_bar;
        }
        full_speed_pattern.replace_all(&ret, "").to_string()
    }
}

fn indent_general(text: &str) -> String {
    let mut ret = String::new();

    for line in text.lines() {
        ret += "> ";
        ret += line;
        ret += "\n";
    }
    ret
}

#[derive(Copy, Clone)]
pub enum MessageComponent {
    HealthBar,
    General,
    Options,
}

#[derive(Clone, Eq, PartialEq)]
pub enum ChannelInfo {
    Local,

    #[cfg(feature = "remote_clients")]
    Remote(String),

    #[cfg(feature = "discord")]
    Discord(ChannelId, UserId),
}
