#[macro_use]
extern crate lazy_static;
extern crate parking_lot;

use std::sync::mpsc::{ self, Sender, TryRecvError::* };
use std::io::{ self, ErrorKind::*, Read, Write };
use std::net::{ SocketAddr, TcpStream };
use std::time::Duration;
use std::str::Lines;
use std::process;
use std::thread;
use std::fs;

use parking_lot::Mutex;

const REFRESH_RATE: u64 = 50;
const RETRY_DELAY: u64 = 1000;
const SHUTDOWN_DELAY: u64 = 5000;
const MAX_RETRIES: u32 = 5;
const MSG_SIZE: usize = 1024;
const IP_STORAGE: &'static str = "last_ip.txt";

lazy_static!
{
    static ref CLIENT_INFO: Mutex<ClientInfo> = Mutex::new(ClientInfo::new());
}

fn main()
{
    let server_ip = get_ip();
    let client = load_client(server_ip);

    start_client(client);
}

fn get_ip() -> SocketAddr
{
    let input_ip = match fs::read_to_string(IP_STORAGE)
    {
        Ok(s) =>
        {
            let msg = format!("The last IP was {}. Is this okay? Y/n", s);
            let confirmation = prompt(&msg);
            match confirmation.to_lowercase().as_str()
            {
                "" | "y" | "y." | "yes" | "yes." => s,
                _ => prompt("Enter the server's IP address:")
            }
        },
        _ => prompt("Enter the server's IP address:")
    };
    let mut ip = input_ip.parse::<SocketAddr>();
    while let Err(_) = ip
    {
        ip = prompt("Invalid address. Try again.").parse()
    }
    fs::write(IP_STORAGE, input_ip)
        .expect("Unable to record IP to file.");
    ip.unwrap()
}

fn load_client(server_ip: SocketAddr) -> TcpStream
{
    let mut try_connect = TcpStream::connect(server_ip);
    let mut num_tries = 0;

    while let Err(_) = try_connect
    {
        if num_tries > MAX_RETRIES
        {
            println!("Failed to connect to server. Aborting.");
            process::exit(-3);
        }
        sleep(RETRY_DELAY);
        num_tries += 1;
        println!("No response from server. Retrying...");
        try_connect = TcpStream::connect(server_ip);
    }
    let client = try_connect.unwrap();
    client.set_nonblocking(true)
        .expect("Failed to set client as non-blocking.");
    client
}

fn start_client(mut client: TcpStream)
{
    let (tx, rx) = mpsc::channel::<String>();

    loop
    {
        let mut buf = vec![0; MSG_SIZE];

        match client.read(&mut buf)
        {
            Ok(_) =>
            {
                let msg: Vec<u8> = buf.into_iter()
                    .take_while(| b | *b != 0)
                    .collect();

                if let Ok(text) = String::from_utf8(msg)
                {
                    match handle_response(&text, &mut client)
                    {
                        Ok(o) => if o == "OK" { handle_inputs(tx.clone()) },
                        Err(_) => {/* Ignore */}
                    };
                }
                else { println!("Error parsing message from server."); }
            }
            Err(ref e) if e.kind() == WouldBlock => (),
            Err(_) =>
            {
                println!("\nLost connection to the server. Closing...");
                sleep(SHUTDOWN_DELAY);
                break;
            }
        }

        match rx.try_recv()
        {
            Ok(msg) => {write(&msg, &mut client)
                .expect("Error writing to socket.");},
            Err(Empty) => (),
            Err(Disconnected) => break
        };

        sleep(REFRESH_RATE);
    };
}

fn handle_response(msg: &str, client: &mut TcpStream) -> Result<&'static str, &'static str>
{
    let mut lines = msg.lines();
    let msg_type = match lines.next()
    {
        Some(l) => l,
        None => return Err("Empty message.")//panic!("Somehow received an empty response")
    };

    match msg_type
    {
        "ESTABLISH" => register_user(client),
        "LOGIN_ERR" => login_err(lines, client),
        "LOGIN_OK" => login_ok(lines),
        _ => standard_msg(msg)
    }
}

fn standard_msg(msg: &str) -> Result<&'static str, &'static str>
{
    io::stdout().write(msg.as_bytes()).unwrap();
    io::stdout().flush().unwrap();

    Ok("All seems well 2.")
}

fn register_user(client: &mut TcpStream) -> Result<&'static str, &'static str>
{
    let mut username = prompt("Enter a username to connect with:");

    // To-do: Handle this on the server side.
    while username.len() < 3 || username.len() > 32
    {
        username = prompt("Username should be between 3 and 32 characters. Try again:");
    }

    let mut info = CLIENT_INFO.lock();
    info.username = Some(username);

    register_from_info(&mut info, client)
}

fn login_err(mut lines: Lines, client: &mut TcpStream) -> Result<&'static str, &'static str>
{
    let err_message = match lines.next()
    {
        Some(e) if e.starts_with("REASON|") => &e[7..],
        _ => panic!("Unable to parse error message from response.")
    };

    match err_message
    {
        "CAPACITY" =>
        {
            println!("Server is at capacity. Try again later.");
            sleep(SHUTDOWN_DELAY);
            process::exit(-2);
        },
        "MAX_VISITORS" =>
        {
            println!("There are too many visitors in the lobby.\nWait a minute and try again.");
            sleep(SHUTDOWN_DELAY);
            process::exit(-4);
        },
        "TAKEN" => change_username(client),
        _ => panic!("Received an unrecognized error message.")
    }
}

fn login_ok(mut lines: Lines) -> Result<&'static str, &'static str>
{
    let token = match lines.next()
    {
        Some(t) if t.starts_with("TOKEN|") => t[6..].to_string(),
        _ => panic!("Unable to parse token from response.")
    };

    let mut info = CLIENT_INFO.lock();
    info.token = Some(token);

    println!("Logged in successfully! Press enter to begin.");

    Ok("OK")
}

fn change_username(client: &mut TcpStream) -> Result<&'static str, &'static str>
{
    let username = prompt("This username is already taken. Enter a different one:");
    let mut info = CLIENT_INFO.lock();
    info.username = Some(username);

    register_from_info(&mut info, client)
}

fn register_from_info(info: &mut ClientInfo, client: &mut TcpStream) -> Result<&'static str, &'static str>
{
    let username = match info.username
    {
        Some(ref u) => u,
        None => panic!("Info does not contain a username.")
    };

    let msg = format!(
        "REGISTER\n\
        USER|{}",
        username,
    );
    write(&msg, client)
}

fn handle_inputs(tx: Sender<String>)
{
    thread::spawn(move || loop
    {
        let mut msg = String::new();
        io::stdin().read_line(&mut msg)
            .expect("Unable to parse input.");
        let msg = msg.trim();

        match msg
        {
            "quit" | "end" | "leave" | "stop" => end(),
            _ if try_send(msg, &tx).is_err() => end(),
            _ => continue
        }
    });
}

fn prompt(msg: &str) -> String
{
    println!("{}", msg);
    let mut ret = String::new();
    io::stdin().read_line(&mut ret)
        .expect("Unable to parse input");
    ret.trim().to_string()
}

fn try_send(msg: &str, tx: &Sender<String>) -> Result<&'static str, &'static str>
{
    let info = CLIENT_INFO.lock();

    if let Some(ref token) = info.token
    {
        let data = format!(
            "STANDARD\n\
            TOKEN|{}\n\
            MSG|{}",
            token,
            msg
        );

        if let Err(_) = tx.send(data)
        {
            return Err("Unable to send message between threads.");
        }
        return Ok("All seems well.")
    }
    Ok("Token isn't ready.") // Ignore these inputs.
}

fn write(msg: &str, stream: &mut TcpStream) -> Result<&'static str, &'static str>
{
    stream.write(msg.as_bytes()).expect("Error writing message.");
    stream.flush().expect("We'll see about that!");

    Ok("Oh, okay.")
}

fn sleep(time: u64)
{
    thread::sleep(Duration::from_millis(time))
}

fn end()
{
    println!("Stopping game...");
    sleep(SHUTDOWN_DELAY);
    process::exit(0);
}

struct ClientInfo
{
    username: Option<String>,
    token: Option<String>
}

impl ClientInfo
{
    fn new() -> ClientInfo
    {
        ClientInfo{ username: None, token: None }
    }
}