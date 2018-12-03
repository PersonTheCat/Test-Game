use std::net::{ TcpListener, TcpStream, SocketAddr };
use std::io::{ ErrorKind::*, Read, Write };
use std::sync::mpsc::{ self, Sender, Receiver };
use std::time::Duration;
use std::str::Lines;
use std::thread;
use std::io;

use yyid::yyid_string;
use hashbrown::HashMap;

use ::ChannelInfo::Remote;
use GameMessage;

/**
 * To-do: Implement passwords using password-hashing (crate).
 */

/** Slightly faster than the main game loop. */
const REFRESH_RATE: u64 = (::MS_BETWEEN_UPDATES * 2 / 3) as u64;
const MSG_SIZE: usize = 128;
const MAX_USERS: usize = 8;

/** These users have not yet logged in. */
type Visitors = Vec<(SocketAddr, TcpStream)>;

/** A map of username -> stream info */
type Clients = HashMap<String, (SocketAddr, TcpStream)>;

/** A map of token -> username */
type Tokens = HashMap<String, String>;

/**
 * Only accessed by the game thread.
 * Should be safe without a mutex.
 */
static mut LOCAL_TX: Option<Sender<String>> = None;

pub fn send_message_to_client(username: &str, msg: &str)
{
    unsafe { if let Some(ref tx) = LOCAL_TX
    {
        tx.send(format!("OUTGOING\nUSER|{}\nMSG|{}", username, msg))
            .expect("Unable to send message to server.");
    }
    else { panic!("Local transmitter was not initialized."); }}
}

pub fn init_listener(sender: Sender<GameMessage>)
{
    let listener = match TcpListener::bind("0.0.0.0:12131")
    {
        Ok(l) => { println!("\nListening on port 12131."); l },
        Err(_) => { println!("\nError binding port 12131."); return; }
    };

    listener.set_nonblocking(true)
        .expect("Error setting listener as non-blocking.");

    let (server_tx, server_rx) = mpsc::channel::<String>();

    unsafe { LOCAL_TX = Some(server_tx.clone()); }

    start_server(listener, server_tx, server_rx, sender);
}

fn start_server(listener: TcpListener, server_tx: Sender<String>, server_rx: Receiver<String>, game_tx: Sender<GameMessage>)
{
    let mut visitors: Visitors = Vec::new();
    let mut clients: Clients = HashMap::new();
    let mut tokens: Tokens = HashMap::new();

    loop
    {
        if let Ok((mut socket, address)) = listener.accept()
        {
            println!("Received a connection from {}.", address);
            let user_tx = server_tx.clone();

            let socket_clone = socket.try_clone()
                .expect("Failed to clone client info.");

            // The user's IP will serve as a temporary identifier.
            write_directly(&format!("ESTABLISH\nADDR|{}", address), &mut socket);

            visitors.push((address.clone(), socket_clone));

            spawn_client_thread(socket, address, user_tx);
        }

        if let Ok(msg) = server_rx.try_recv()
        {
            match handle_incoming_message(&msg, &mut visitors, &mut clients, &mut tokens, &game_tx)
            {
                Ok(_o) => (), //println!("Ok: {}", o),
                Err(_e) => (), //println!("Err: {}", e)
            };
        }

        sleep();
    }
}

fn spawn_client_thread(mut socket: TcpStream, address: SocketAddr, user_tx: Sender<String>)
{
    thread::spawn(move || loop
    {
        let mut buf = vec![0; MSG_SIZE];

        match socket.read_exact(&mut buf)
        {
            Ok(_) =>
            {
                let msg: Vec<u8> = buf.into_iter()
                    .take_while(| b | *b != 0)
                    .collect();
                let msg = String::from_utf8(msg)
                    .expect("Client send an invalid utf8 message.");

                //println!("{}: {:?}", address, msg);
                user_tx.send(msg)
                    .expect("Failed to send user message");
            }
            Err(ref e) if e.kind() == WouldBlock => continue,
            Err(_) => { println!("Closing connection with: {}.", address); break; }
        }
        sleep();
    });
}

fn handle_incoming_message(msg: &str, visitors: &mut Visitors, clients: &mut Clients, tokens: &mut Tokens, game_tx: &Sender<GameMessage>) -> Result<&'static str, &'static str>
{
    let mut lines = msg.lines();

    let msg_type = match lines.next()
    {
        Some(s) => s,
        None => return Err("Client message contained no info.")
    };

    match msg_type
    {
        "OUTGOING" => outgoing_message(lines, clients),
        "STANDARD" => standard_message(lines, tokens, game_tx),
        "REGISTER" => register_user(lines, visitors, clients, tokens),
        _ => {Err("")}
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
fn outgoing_message(mut lines: Lines, clients: &mut Clients) -> Result<&'static str, &'static str>
{
    let username = match lines.next()
    {
        Some(u) if u.starts_with("USER|") => &u[5..],
        _ => return Err("Outgoing call was sent incorrectly.")
    };
    let msg = match lines.next()
    {
        Some(m) if m.starts_with("MSG|") =>
        {
            let mut ret = String::new();
            ret += &m[4..];
            while let Some(t) = lines.next()
            {
                ret += "\n";
                ret += t;
            }
            ret
        }
        _ => return Err("Outgoing call was sent incorrectly.")
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
fn standard_message(mut lines: Lines, tokens: &Tokens, game_tx: &Sender<GameMessage>) -> Result<&'static str, &'static str>
{
    let token = match lines.next()
    {
        Some(s) if s.starts_with("TOKEN|") => s[6..].to_string(),
        _ => return Err("Standard call was sent incorrectly.")
    };
    let msg = match lines.next()
    {
        Some(s) if s.starts_with("MSG|") =>
        {
            let mut msg = s[4..].to_string();
            while let Some(line) = lines.next()
            {
                msg += line;
            }
            msg
        }
        _ => return Err("Standard call was sent incorrectly.")
    };
    let username = match tokens.get(&token)
    {
        Some(u) => u.to_owned(),
        None => return Err("An invalid token was sent. The client will not be informed.")
    };

    let game_message = GameMessage
    {
        message: msg,
        channel_info: Remote(username)
    };

    match game_tx.send(game_message)
    {
        Ok(_) => Ok("Everything looks okay."),
        Err(_) => Err("Unable to send message from server to game thread.")
    }
}

/**
 * Client sent a message in this format:
 * ```
 * REGISTER
 * USER|my_username
 * ADDR|0.0.0.0:0000
 * ```
 * To-do: Include a password.
 */
fn register_user(mut lines: Lines, visitors: &mut Visitors, clients: &mut Clients, tokens: &mut Tokens) -> Result<&'static str, &'static str>
{
    let username = match lines.next()
    {
        Some(s) if s.starts_with("USER|") => s[5..].to_string(),
        _ => return Err("Register call was sent incorrectly.")
    };
    let address: SocketAddr = match lines.next()
    {
        Some(s) if s.starts_with("ADDR|") => match s[5..].parse()
        {
            Ok(a) => a,
            Err(_) => return Err("Unable to parse client address.")
        },
        _ => return Err("Register call was sent incorrectly.")
    };

    if tokens.len() >= MAX_USERS
    {
        let response = String::from(
            "LOGIN_ERR\n\
            REASON|CAPACITY"
        );

        write_to_visitor(&response, address, visitors);
        Err("There were too many users logged in.")
    }
    else if is_logged_in(&username, clients)
    {
        let response = String::from(
            "LOGIN_ERR\n\
            REASON|TAKEN" //This username is already taken. Try a different one."
        );

        write_to_visitor(&response, address, visitors);
        Err("Username was already taken.")
    }
    else
    {
        let new_client = match locate_visitor(address, visitors)
        {
            Some(v) => v,
            None => return Err("Client disconnected before registration.")
        };

        let token = yyid_string();

        let response = format!(
            "LOGIN_OK\n\
            TOKEN|{}",
            token
        );

        clients.insert(username.clone(), new_client);
        write_to_client(&response, &username, clients);
        tokens.insert(token, username);

        Ok("Client registered successfully.")
    }
}

fn write_to_client(msg: &str, username: &str, clients: &mut Clients)
{
    let mut remove_user = false;

    if let Some((_address, stream)) = clients.get_mut(username)
    {
        match write_directly(msg, stream)
        {
            Ok(_) => return,
            Err(_) => remove_user = true
        };
    }
    if remove_user { clients.remove(username); }
}

fn write_to_visitor(msg: &str, address: SocketAddr, visitors: &mut Visitors)
{
    let mut remove_index: Option<usize> = None;

    for (index, (addr, stream)) in visitors.iter_mut().enumerate()
    {
        if *addr == address
        {
            match write_directly(msg, stream)
            {
                Ok(_) => return,
                Err(_) => remove_index = Some(index)
            };
        }
    }

    if let Some(index) = remove_index
    {
        visitors.remove(index);
    }
}

/**
 * Splits the message into smaller packets
 * when the size overflows. This is very
 * noticeable.
 */
fn write_directly(msg: &str, stream: &mut TcpStream) -> Result<(), io::Error>
{
    if msg.len() == 0 { return Ok(()) }

    let mut bytes = msg.to_string().into_bytes();

    while bytes.len() > MSG_SIZE
    {
        let mut buf = vec![0; MSG_SIZE];
        for i in 0..MSG_SIZE
        {
            buf[i] = bytes.remove(0);
        }
        stream.write_all(&buf)?;
    }
    if bytes.len() > 0
    {
        bytes.resize(MSG_SIZE, 0);
        stream.write_all(&bytes)?;
    }
    Ok(())
}

fn is_logged_in(username: &str, clients: &Clients) -> bool
{
    clients.contains_key(username)
}

fn locate_visitor(address: SocketAddr, visitors: &mut Visitors) -> Option<(SocketAddr, TcpStream)>
{
    let index = visitors.iter()
        .position(| (a, _) | *a == address);

    match index
    {
        Some(num) => Some(visitors.remove(num)),
        None => None
    }
}

fn sleep()
{
    thread::sleep(Duration::from_millis(REFRESH_RATE))
}