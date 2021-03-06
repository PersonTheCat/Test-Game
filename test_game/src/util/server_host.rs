use std::io;
use std::io::{ErrorKind::*, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::str::Lines;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

use lazy_static::lazy_static;
use parking_lot::Mutex;
use hashbrown::HashMap;
use yyid::yyid_string;

use crate::ChannelInfo::Remote;
use crate::GameMessage;
use crate::*;

// To-do: Implement passwords using password-hashing (crate).

/// Much faster than the main game loop -> lower latency.
const REFRESH_RATE: u64 = 50;
const MSG_SIZE: usize = 256;
const MAX_USERS: usize = 8;
const MAX_VISITORS: usize = 8;

/// These users have not yet logged in.
type Visitors = Vec<(SocketAddr, TcpStream)>;

/// A map of username -> stream info
type Clients = HashMap<String, (SocketAddr, TcpStream)>;

/// A map of token -> username
type Tokens = HashMap<String, String>;

/// Message, address it was sent from; might be local.
struct MessageData(String, Option<SocketAddr>);

lazy_static! {
    static ref LOCAL_TX: Mutex<Option<Sender<MessageData>>> = Mutex::new(None);
}

pub fn send_message_to_client(username: &str, msg: &str) {
    let tx = LOCAL_TX.lock();
    if let Some(ref t) = *tx {
        t.send(MessageData(format!("OUTGOING\nUSER|{}\nMSG|{}", username, msg),None))
            .expect("Unable to send message to server.");
    } else {
        panic!("Tried to send a message before the server started.");
    }
}

pub fn init_listener(sender: Sender<GameMessage>) {
    let listener = match TcpListener::bind("0.0.0.0:12131") {
        Ok(l) => { println!("\nListening on port 12131."); l },
        Err(_) => { println!("\nError binding port 12131."); return; }
    };

    listener.set_nonblocking(true)
        .expect("Error setting listener as non-blocking.");

    let (server_tx, server_rx) = mpsc::channel::<MessageData>();
    *LOCAL_TX.lock() = Some(server_tx.clone());

    start_server(listener, server_tx, server_rx, sender);
}

fn start_server(listener: TcpListener, server_tx: Sender<MessageData>, server_rx: Receiver<MessageData>, game_tx: Sender<GameMessage>) {
    let mut visitors: Visitors = Vec::new();
    let mut clients: Clients = HashMap::new();
    let mut tokens: Tokens = HashMap::new();

    loop {
        if let Ok((mut socket, address)) = listener.accept() {
            // Hold visitors in a separate array from established
            // clients. They will get their own threads once they
            // have been registered successfully and have received
            // `LOGIN_OK` as well as a `TOKEN` for communicating
            // with the game.
            println!("Received a connection from {}.", address);

            if visitors.len() > MAX_VISITORS {
                // There were too many users waiting to log in.
                write_directly("LOGIN_ERR\nREASON|MAX_VISITORS", &mut socket)
                    .expect("Error writing to socket.");
                continue;
            }

            // The user's IP will serve as a temporary identifier.
            write_directly("ESTABLISH", &mut socket)
                .expect("Error writing to socket.");

            visitors.push((address, socket));
        }

        // Process incoming messages from visitors in the current thread.
        // Sever connections when messages can't be read.
        visitors.drain_filter(|(address, socket)|
            handle_reads(socket, &address, &server_tx).is_err());

        if let Ok(msg) = server_rx.try_recv() {
            match handle_incoming_message(msg, &mut visitors, &mut clients, &mut tokens, &server_tx, &game_tx) {
                Ok(_o) => (), //println!("Ok: {}", o),
                Err(_) => ()//println!("Err: {}", e),
            };
        }
        sleep();
    }
}

fn spawn_client_thread(mut socket: TcpStream, address: SocketAddr, user_tx: Sender<MessageData>) {
    thread::spawn(move || loop {
        if handle_reads(&mut socket, &address, &user_tx).is_err() {
            break;
        };
        sleep();
    });
}

fn handle_reads(socket: &mut TcpStream, address: &SocketAddr, server_tx: &Sender<MessageData>) -> io::Result<()> {
    let mut buf = vec![0; MSG_SIZE];

    match socket.read(&mut buf) {
        Ok(_) => {
            let msg: Vec<u8> = buf.into_iter()
                .take_while(|b| *b != 0)
                .collect();
            let msg = String::from_utf8(msg)
                .expect("Client sent an invalid utf8 message.");

            server_tx.send(MessageData(msg, Some(address.clone())))
                .expect("Failed to send user message");
        }
        Err(ref e) if e.kind() == WouldBlock => (),
        Err(e) => {
            server_tx.send(MessageData("CLOSE".to_string(), Some(address.clone())))
                .expect("Failed to send user message");
            println!("Closing connection with: {}.", address);
            return Err(e);
        }
    }
    Ok(())
}

fn handle_incoming_message(
    msg: MessageData,
    visitors: &mut Visitors,
    clients: &mut Clients,
    tokens: &mut Tokens,
    server_tx: &Sender<MessageData>,
    game_tx: &Sender<GameMessage>
) -> Result<&'static str, &'static str> {
    let mut lines = msg.0.lines();

    let msg_type = match lines.next() {
        Some(s) => s,
        None => return Err("Client message contained no info."),
    };

    match msg_type {
        "OUTGOING" => outgoing_message(lines, clients),
        "STANDARD" => standard_message(lines, tokens, game_tx),
        "REGISTER" => register_user(lines, &msg, visitors, clients, tokens, server_tx),
        "CLOSE" => disconnect_message(&msg, clients),
        _ => Err("Unregistered message header"),
    }
}

/**
 * Game sent a message in this format:
 * ```
 * OUTGOING
 * USER|my_username
 * MSG|text_to_display
 * ```
 * Forwarding it out as a standard message.
 */
fn outgoing_message(mut lines: Lines, clients: &mut Clients) -> Result<&'static str, &'static str> {
    let username = match lines.next() {
        Some(u) if u.starts_with("USER|") => &u[5..],
        _ => return Err("Outgoing call was sent incorrectly."),
    };
    let msg = match lines.next() {
        Some(m) if m.starts_with("MSG|") => {
            let mut ret = String::new();
            ret += &m[4..];
            while let Some(t) = lines.next() {
                ret += "\n";
                ret += t;
            }
            ret
        }
        _ => return Err("Outgoing call was sent incorrectly."),
    };
    write_to_client(&msg, username, clients);
    Ok("Success.")
}

/**
 * Client sent a message in this format:
 * ```
 * STANDARD
 * TOKEN|token
 * MSG|text_to_process
 * ```
 * To-do: Replace usernames with tokens.
 */
fn standard_message(mut lines: Lines, tokens: &Tokens, game_tx: &Sender<GameMessage>) -> Result<&'static str, &'static str> {
    let token = match lines.next() {
        Some(s) if s.starts_with("TOKEN|") => s[6..].to_string(),
        _ => return Err("Standard call was sent incorrectly."),
    };
    let msg = match lines.next() {
        Some(s) if s.starts_with("MSG|") => {
            let mut msg = s[4..].to_string();
            while let Some(line) = lines.next() {
                msg += line;
            }
            msg
        }
        _ => return Err("Standard call was sent incorrectly."),
    };
    let username = match tokens.get(&token) {
        Some(u) => u.to_owned(),
        None => return Err("An invalid token was sent. The client will not be informed."),
    };
    let game_message = GameMessage {
        message: msg,
        channel_info: Remote(username),
    };

    match game_tx.send(game_message) {
        Ok(_) => Ok("Everything looks okay."),
        Err(_) => Err("Unable to send message from server to game thread."),
    }
}

/**
 * Client sent a message in this format:
 * ```
 * REGISTER
 * USER|my_username
 * ```
 * To-do: Include a password.
 */
fn register_user(
    mut lines: Lines,
    data: &MessageData,
    visitors: &mut Visitors,
    clients: &mut Clients,
    tokens: &mut Tokens,
    server_tx: &Sender<MessageData>
) -> Result<&'static str, &'static str> {
    let username = match lines.next() {
        Some(s) if s.starts_with("USER|") => s[5..].to_string(),
        _ => return Err("Register call was sent incorrectly."),
    };
    let address = data.1
        .expect("A register call did not contain the user's address.");

    if tokens.len() >= MAX_USERS {
        // Too many users are currently logged in.
        write_to_visitor("LOGIN_ERR\nREASON|CAPACITY", address, visitors);
        Err("There were too many users logged in.")
    } else if is_logged_in(&username, clients) {
        // The username was already taken.
        write_to_visitor("LOGIN_ERR\nREASON|TAKEN", address, visitors);
        Err("Username was already taken.")
    } else {
        // All seems well.
        let new_client = match locate_visitor(&address, visitors) {
            Some(v) => v,
            None => return Err("Client disconnected before registration."),
        };

        let token = yyid_string();
        let response = format!(
            "LOGIN_OK\n\
             TOKEN|{}",
            token
        );

        let clone = clone_client_info(&new_client);
        spawn_client_thread(clone.1, clone.0, server_tx.clone());

        clients.insert(username.clone(), new_client);
        write_to_client(&response, &username, clients);
        send_global_message(&format!("{} has logged in.", username));
        tokens.insert(token, username);

        Ok("Client registered successfully.")
    }
}

/**
 * Server sent a message in this format:
 * ```
 * CLOSE
 * ```
 * Using this to inform other users.
 */
fn disconnect_message(msg: &MessageData, clients: &Clients) -> Result<&'static str, &'static str> {
    if let Some(ref address) = msg.1 {
        if let Some(username) = locate_client_username(address, clients) {
            send_global_message(&format!("{} has disconnected.", username));
            return Ok("Users were informed.");
        }
    }
    Err("Unable to inform users of disconnect.")
}

fn clone_client_info(client: &(SocketAddr, TcpStream)) -> (SocketAddr, TcpStream) {
    let socket_clone = client.1.try_clone()
        .expect("Unable to clone client info.");
    (client.0.clone(), socket_clone)
}

fn write_to_client(msg: &str, username: &str, clients: &mut Clients) {
    let mut remove_user = false;

    if let Some((_address, stream)) = clients.get_mut(username) {
        match write_directly(msg, stream) {
            Ok(_) => return,
            Err(_) => remove_user = true,
        };
    }
    if remove_user {
        clients.remove(username);
    }
}

fn write_to_visitor(msg: &str, address: SocketAddr, visitors: &mut Visitors) {
    let mut remove_index: Option<usize> = None;

    for (index, (addr, stream)) in visitors.iter_mut().enumerate() {
        if *addr == address {
            match write_directly(msg, stream) {
                Ok(_) => return,
                Err(_) => remove_index = Some(index),
            };
        }
    }
    if let Some(index) = remove_index {
        visitors.remove(index);
    }
}

fn write_directly(msg: &str, stream: &mut TcpStream) -> Result<(), io::Error> {
    stream.write(msg.as_bytes())?;
    stream.flush()?;
    Ok(())
}

fn is_logged_in(username: &str, clients: &Clients) -> bool {
    clients.contains_key(username)
}

fn locate_visitor(address: &SocketAddr, visitors: &mut Visitors) -> Option<(SocketAddr, TcpStream)> {
    visitors.iter()
        .position(|(a, _)| *a == *address)
        .and_then(|i| Some(visitors.remove(i)))
}

fn locate_client_username<'a>(address: &SocketAddr, clients: &'a Clients) -> Option<&'a str> {
    for (username, (addr, _stream)) in clients {
        if *addr == *address {
            return Some(username);
        }
    }
    None
}

fn sleep() {
    thread::sleep(Duration::from_millis(REFRESH_RATE))
}
