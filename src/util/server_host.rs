use std::net::{ TcpListener, TcpStream, SocketAddr };
use std::io::{ ErrorKind::*, Read, Write };
use std::sync::mpsc::{ self, Sender, Receiver };
use std::time::Duration;
use std::str::Lines;
use std::thread;

use hashbrown::HashMap;

use ::ChannelInfo::Remote;
use GameMessage;

/**
 * To-do: Implement passwords using password-hashing (crate).
 */

/** Slightly faster than the main game loop. */
const REFRESH_RATE: u64 = (::MS_BETWEEN_UPDATES * 3 / 2) as u64;
const MSG_SIZE: usize = 128;

type Clients = Vec<(SocketAddr, TcpStream)>;
type ClientMap = HashMap<String, SocketAddr>;

/**
 * Only accessed by the game thread.
 * Should be safe.
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
    let listener = match TcpListener::bind("127.0.0.1:12131")
    {
        Ok(l) => { println!("Listening on port 12131."); l },
        Err(_) => { println!("Error binding port 12131."); return; }
    };

    listener.set_nonblocking(true)
        .expect("Error setting listener as non-blocking.");

    let (server_tx, server_rx) = mpsc::channel::<String>();

    spawn_server_thread(listener, server_tx.clone(), server_rx, sender);

    unsafe { LOCAL_TX = Some(server_tx); }
}

fn spawn_server_thread(listener: TcpListener, server_tx: Sender<String>, server_rx: Receiver<String>, game_tx: Sender<GameMessage>)
{
    let mut clients: Clients = Vec::new();
    let mut client_map: ClientMap = HashMap::new();

    loop
    {
        if let Ok((socket, address)) = listener.accept()
        {
            println!("Received a connection from {}.", address);
            let user_tx = server_tx.clone();

            let socket_clone = socket.try_clone()
                .expect("Failed to clone client info.");

            clients.push((address.clone(), socket_clone));

            spawn_client_thread(socket, address, user_tx);
        }

        if let Ok(msg) = server_rx.try_recv()
        {
            match handle_incoming_message(&msg, &mut clients, &mut client_map, &game_tx)
            {
                Ok(o) => println!("Ok: {}", o),
                Err(e) => println!("Err: {}", e)
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

                println!("{}: {:?}", address, msg);
                user_tx.send(msg)
                    .expect("Failed to send user message");
            }
            Err(ref e) if e.kind() == WouldBlock => continue,
            Err(_) => { println!("Closing connection with: {}.", address); break; }
        }
        sleep();
    });
}

fn handle_incoming_message(msg: &str, clients: &mut Clients, client_map: &mut ClientMap, game_tx: &Sender<GameMessage>) -> Result<&'static str, &'static str>
{
    let mut lines = msg.lines();

    let msg_type = match lines.next()
    {
        Some(s) => s,
        None => return Err("Client message contained no info.")
    };

    match msg_type
    {
        "OUTGOING" => outgoing_message(lines, clients, client_map),
        "STANDARD" => standard_message(lines, game_tx),
        "REGISTER" => register_user(lines, client_map),
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
 */
fn outgoing_message(mut lines: Lines, clients: &mut Clients, client_map: &mut ClientMap) -> Result<&'static str, &'static str>
{
    let username = match lines.next()
    {
        Some(s) if s.starts_with("USER|") => s[5..].to_string(),
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
    let address = match client_map.get(&username)
    {
        Some(a) => a,
        None => return Err("Client was not registered correctly.")
    };

    write_to_stream(&msg, address, clients);

    Ok("All seems well. Update this message soon.")
}

/**
 * Client sent a message in this format:
 * ```
 * STANDARD
 * USER|my_username
 * MSG|text_to_process
 * ```
 */
fn standard_message(mut lines: Lines, game_tx: &Sender<GameMessage>) -> Result<&'static str, &'static str>
{
    let username = match lines.next()
    {
        Some(s) if s.starts_with("USER|") => s[5..].to_string(),
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
 */
fn register_user(mut lines: Lines, client_map: &mut ClientMap) -> Result<&'static str, &'static str>
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

    client_map.insert(username, address);

    Ok("Client registered successfully.")
}

fn write_to_stream(msg: &str, address: &SocketAddr, clients: &mut Clients)
{
    let mut remove_index: Option<usize> = None;

    for (index, (addr, stream)) in clients.iter_mut().enumerate()
    {
        if addr == address
        {
            let mut buf = msg.to_string().into_bytes();
            buf.resize(MSG_SIZE, 0);
            match stream.write_all(&buf).map(|_| stream)
            {
                Ok(_) => return,
                Err(_) => { remove_index = Some(index); break; }
            };
        }
    }
    if let Some(index) = remove_index
    {
        clients.remove(index);
    }
}

fn sleep()
{
    thread::sleep(Duration::from_millis(REFRESH_RATE))
}