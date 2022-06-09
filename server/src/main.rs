#![allow(unused)]

mod message;
mod message_old;
mod server;
mod user;

use log::{debug, error, info};
use message::{Command, Message, ReplyCode, Response, ToIrc};
use std::{
    io::{Error, Read, Write},
    net::{TcpListener, TcpStream},
    str::{self, FromStr},
    sync::{Arc, Mutex},
    thread,
};
use user::User;

fn main() {
    env_logger::init();

    // TODO: Switch to HashMap since it'll solve the index deletion issue
    let users: Arc<Mutex<Vec<User>>> = Arc::new(Mutex::new(vec![]));
    // let channels: Arc<Mutex<Vec<Channel>>> = Arc::new(Mutex::new(vec![]));
    let hostname = "127.0.0.1:8080"; // TODO: Allow for custom port
    let listener = TcpListener::bind(hostname).expect(&format!("Couldn't bind to {hostname}."));
    let server_prefix = "localhost";

    println!("Listening on {hostname}.");

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        let users = users.clone();
        thread::spawn(move || handle_client(stream, users, server_prefix));
    }
}

fn handle_client(mut stream: TcpStream, users: Arc<Mutex<Vec<User>>>, server_prefix: &str) {
    // Add new user to the users array
    let user_index;
    match users.lock() {
        Ok(mut users) => {
            let writer = stream.try_clone().unwrap();
            user_index = users.len();
            users.push(User::new(writer));
            println!("User has connected. {} active connections.", users.len());
        }
        Err(err) => panic!("{err}"),
    }
    let username = "Client";

    loop {
        // Wait for message from client
        let mut message_ascii = vec![0; shared::MESSAGE_SIZE];
        stream
            .read(&mut message_ascii)
            .expect("Failed to read message from client.");

        // Convert `message` to a String and print it out
        // TODO: Figure out a way to avoid this mess
        let message_str = str::from_utf8(&message_ascii)
            .expect("Client sent an invalid UTF-8 message.")
            .replace('\0', "");
        println!("{:?}", message_str);

        // Update prefix from latest user and server info
        let server_prefix = server_prefix.to_string();
        let user_prefix = match users.lock() {
            Ok(users) => users[user_index].prefix(),
            Err(err) => None,
        };

        // Extract IRC command from client input
        let message = match Message::from(&message_str) {
            Ok(message) => message,
            Err(err) => {
                // TODO: Fix reply code
                let response = Response {
                    prefix: server_prefix,
                    code: ReplyCode::ERR_UNKNOWNCOMMAND,
                    params: vec![err.to_string()],
                };
                send_to_user(&response.to_irc(), &users, user_index);
                continue;
            }
        };

        // Perform associated command with message
        match message.command {
            // Command::Nick => todo!(),
            // Command::Join => todo!(),
            // Command::Kick => todo!(),
            // Command::Part => todo!(),
            // Command::PrivMsg => todo!(),
            // Command::List => todo!(),
            // Command::Away => todo!(),
            // Command::Quit => todo!(),
            Command::Unknown => {
                let response = Response {
                    prefix: server_prefix,
                    code: ReplyCode::ERR_UNKNOWNCOMMAND,
                    params: vec!["Unknown command.".to_string()],
                };
                send_to_user(&response.to_irc(), &users, user_index);
                // println!("{:?}", response.to_irc());
            }
            _ => {
                let response = Response {
                    prefix: server_prefix,
                    code: ReplyCode::RPL_WELCOME,
                    params: vec!["Welcome to the Internet Relay Network!".to_string()],
                };
                send_to_user(&response.to_irc(), &users, user_index);
                // println!("{:?}", response.to_irc());
            }
        }
    }

    // Remove user from the users array after connection has been terminated
    match users.lock() {
        Ok(mut users) => {
            // TODO: Make removal and adding of new users more robust
            users.remove(user_index);
            println!("User has disconnected. {} users left.", users.len());
        }
        Err(err) => panic!("{err}"),
    };
}

// TODO: Figure out why I need to declare an explicit lifetime for this function
// -> It seems like the Box struct has an implicit 'static lifetime, so I have to specify the
//    lifetime of `users` because it returns an error if lock() fails.
fn send_to_user<'a>(
    message: &str,
    users: &'a Arc<Mutex<Vec<User>>>,
    user_index: usize,
) -> Result<(), Box<dyn std::error::Error + 'a>> {
    let mut users = users.lock()?;
    for i in 0..users.len() {
        if i == user_index {
            users[i].stream.write_all(message.as_bytes())?;
        }
    }
    println!("{:?}", message);
    Ok(())
}

fn send_excluding_user<'a>(
    message: &str,
    users: &'a Arc<Mutex<Vec<User>>>,
    user_index: usize,
) -> Result<(), Box<dyn std::error::Error + 'a>> {
    let mut users = users.lock()?;
    for i in 0..users.len() {
        if i != user_index {
            users[i].stream.write_all(message.as_bytes())?;
            // .expect("Failed to send message to user.");
        }
    }
    Ok(())
}

fn send_to_all_users<'a>(
    message: &str,
    users: &'a Arc<Mutex<Vec<User>>>,
) -> Result<(), Box<dyn std::error::Error + 'a>> {
    let mut users = users.lock()?;
    for i in 0..users.len() {
        users[i].stream.write_all(message.as_bytes())?;
    }
    Ok(())
}
