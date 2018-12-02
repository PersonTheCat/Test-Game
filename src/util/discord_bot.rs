use serenity::model::id::{ ChannelId, UserId };
use serenity::client::{ Client, Context };
use serenity::model::channel::Message;
use serenity::prelude::EventHandler;
use serenity::model::user::User;

use std::sync::mpsc::Sender;
use std::fs::OpenOptions;
use std::io::prelude::*;
use parking_lot::Mutex;
use std::fs::File;
use std::io;

use messages::ChannelInfo::Discord;
use GameMessage;

/**
 * To-do: Use one file instead of two.
 */

const TOKEN_FILE: &'static str = "discord_token.txt";
const CHANNELS_FILE: &'static str = "discord_channels.txt";
const COMMAND_INDICATOR: &'static str = "!";

/**
 * This just determines whether to send a new message or
 * edit a previous message. Needs some work for clarity.
 */
pub fn handle_discord_message(channel: ChannelId, _user: UserId, message: &str)
{
    match channel.messages(| m | m.most_recent())
    {
        Ok(ref mut matches) =>
        {
            /**
             * No messages found -> send a new one.
             * Last message somehow came from a user -> same.
             */
            if matches.len() == 0 || !matches[0].author.bot
            {
                send_message(channel, message);
                return;
            }

            let mut formatted = String::new();
            let previous = matches[0].content.to_owned();

            /**
             * Following this pattern:
             * ```
             * previous contents
             * new contents
             * ```
             *
             * To-do: It may be simpler to just insert the
             * new message before the closing backticks.
             */
            formatted += "```";

            if previous.starts_with("```")
            {
                formatted += &previous[3..previous.len() - 3]
            }
            else { formatted += &previous; }

            formatted += message;
            formatted += "```";

            &mut matches[0].edit(| edit |
            {
                edit.content(formatted)
            });
        }
        Err(_) => send_message(channel, message)
    }
}

/**
 * To-do: Send these messages as embeds. Seeing as
 * the Discord functionality probably *cannot* get
 * very good, this may not happen.
 */
fn send_message(channel: ChannelId, message: &str)
{
    if let Err(_) = channel.say(standard_formatting(message))
    {
        /* To-do */
    }
}

fn standard_formatting(message: &str) -> String
{
    format!("```\n{}\n```", message)
}

pub struct Bot
{
    /**
     * These need to be individually stored in mutexes in
     * order for the Discord client to hold them correctly.
     */
    sender: Mutex<Sender<GameMessage>>,
    channels: Mutex<Vec<u64>>
}

impl Bot
{
    /**
     * Functions related to initializing the bot.
     */

    pub fn load(sender: Sender<GameMessage>) -> bool
    {
        let token = match Self::load_token()
        {
            Some(t) => t,
            None => return false
        };

        let channels = Self::load_channels();
        let handler = Bot::new(sender, channels);

        Client::new(&token, handler)
            .expect("Error creating Discord client.")
            .start()
            .expect("Error connecting to Discord's servers.");

        true
    }

    fn load_token() -> Option<String>
    {
        match get_file_contents(TOKEN_FILE)
        {
            Some(t) => if t.is_empty() { None } else { Some(t) },
            None =>
            {
                File::create(TOKEN_FILE)
                    .expect("Error creating token file for Discord client.");
                None
            }
        }
    }

    fn load_channels() -> Vec<u64>
    {
        match get_file_contents(CHANNELS_FILE)
        {
            Some(t) =>
            {
                let mut vec = Vec::new();
                let split = t.lines();
                for s in split
                {
                    if let Ok(num) = s.parse::<u64>()
                    {
                        vec.push(num);
                        continue;
                    }
                    println!("Error reading from {}. Ignoring.", CHANNELS_FILE);
                    break;
                }
                vec
            },
            None =>
            {
                File::create(CHANNELS_FILE)
                    .expect("Error creating channels file for Discord client.");
                Vec::new()
            }
        }
    }

    fn new(sender: Sender<GameMessage>, channels: Vec<u64>) -> Bot
    {
        Bot { sender: Mutex::new(sender), channels: Mutex::new(channels) }
    }

    /**
     * Miscellaneous tools for the bot to use.
     */

    fn is_registered(&self, channel: u64) -> bool
    {
        let channels = self.channels.lock().unwrap();
        channels.contains(&channel)
    }

    /**
     * Commands
     */

    fn process_commands(&self, cmd: &str, _author: &User, channel: u64) -> Option<String>
    {
        match cmd
        {
            "addchannel" => Some(self.add_channel(channel)
                .expect("Error writing to channels file.")),
            "removechannel" => Some(self.remove_channel(channel)
                .expect("Error writing to channels file.")),
            _ => None
        }
    }

    fn add_channel(&self, num: u64) -> io::Result<String>
    {
        let mut channels = self.channels.lock().unwrap();

        if channels.contains(&num)
        {
            return Ok(String::from("This is already a game channel."))
        }

        let mut file = open_or_create(CHANNELS_FILE);
        let line = num.to_string() + "\n";
        file.write(line.as_bytes())?;
        channels.push(num);

        Ok(String::from("Channel added successfully."))
    }

    fn remove_channel(&self, num: u64) -> io::Result<String>
    {
        let mut channels = self.channels.lock().unwrap();

        if !channels.contains(&num)
        {
            return Ok(String::from("This is not a game channel."))
        }

        //We need to close the file and open it in write-only.
        let contents = get_file_contents(CHANNELS_FILE)
            .expect("Channels file was deleted mid-operation.");
        let updated = remove_line(&contents, &num.to_string());

        let mut file = OpenOptions::new()
            .write(true)
            .open(CHANNELS_FILE)?;

        file.set_len(0)?;
        file.write(updated.as_bytes())?;
        channels.remove_item(&num);

        Ok(String::from("Channel removed successfully."))
    }
}

impl EventHandler for Bot
{
    fn message(&self, _ctx: Context, msg: Message)
    {
        if msg.author.bot { return; }

        let content = &msg.content;

        if content.starts_with(COMMAND_INDICATOR)
        {
            match self.process_commands(&content[1..], &msg.author, msg.channel_id.0)
            {
                Some(response) =>
                {
                    msg.reply(&response).expect("Unable to send reply.");
                },
                None => {/* ignore */}
            }
        }
        else if self.is_registered(msg.channel_id.0)
        {
            let sender = self.sender.lock().unwrap();

            let message = GameMessage
            {
                channel_info: Discord(msg.channel_id, msg.author.id),
                message: msg.content.to_owned()
            };
            sender.send(message)
                .expect("Unable to handle message from Discord client.");

            if let Err(_) = msg.delete()
            {
                /* ignore */
            }
        }
    }
}

/**
 * Serves the specific purpose of opening the file
 * in write / append mode for new contents to be
 * added to previous contents, or write-only mode
 * for new contents to be added to nothing.
 */
fn open_or_create(path: &str) -> File
{
    let file = OpenOptions::new()
        .append(true)
        .open(path);

    match file
    {
        Ok(f) => f,
        Err(_) => File::create(path)
            .expect("Error creating text file.")
    }
}

fn get_file_contents(path: &str) -> Option<String>
{
    if let Ok(mut f) = File::open(path)
    {
        return Some(get_contents(&mut f));
    }
    None
}

fn get_contents(file: &mut File) -> String
{
    let mut contents = String::new();

    file.read_to_string(&mut contents)
        .expect("Error reading from file.");

    contents
}

fn remove_line(text: &str, line: &str) -> String
{
    let mut updated = String::new();
    for l in text.lines()
    {
        if l != line { updated += l; }
    }
    updated
}