#![allow(unused)]

use log::{debug, info};
use shared::user::{Channel, User};
use std::{
    io::{Read, Write},
    net::TcpListener,
    str,
    sync::{Arc, Mutex},
    thread,
};

fn main() {
    env_logger::init();

    let users: Arc<Mutex<Vec<User>>> = Arc::new(Mutex::new(vec![]));
    let channels: Arc<Mutex<Vec<Channel>>> = Arc::new(Mutex::new(vec![]));
    let hostname = "127.0.0.1:8080";
    let listener = TcpListener::bind(hostname).expect(&format!("Couldn't bind to {}.", hostname));

    debug!("Listening on {}.", hostname);

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                // Clone users vector for new thread
                let users = Arc::clone(&users);

                thread::spawn(move || {
                    // Add new user to the users array
                    let user_index;
                    match users.lock() {
                        Ok(mut users) => {
                            let writer = stream.try_clone().unwrap();
                            user_index = users.len();
                            users.push(User::new("name", "nickname", "hostname", writer));
                            println!("User has connected. {} active connections.", users.len());
                        }
                        Err(err) => panic!("{}", err),
                    }

                    loop {
                        // Wait for message from client
                        let mut message = vec![0; shared::MESSAGE_SIZE];
                        stream
                            .read(&mut message)
                            .expect("Failed to read message from client.");

                        // Convert `message` to a String and print it out
                        // TODO: Figure out a way to avoid this mess
                        let message_str = str::from_utf8(&message)
                            .expect("Server sent an invalid UTF-8 message.");
                        let message_str = message_str.replace('\0', "");
                        let message_str = message_str.trim_end();
                        println!("<Client> {:?}", message_str);

                        // Exit thread if client wishes to quit
                        if message_str == "quit" || message_str == "exit" {
                            break;
                        }

                        // Send message back to all other clients
                        match users.lock() {
                            Ok(mut users) => {
                                for i in 0..users.len() {
                                    if i != user_index {
                                        users[i]
                                            .stream
                                            .write_all(&message)
                                            .expect("Failed to send message to client.");
                                    }
                                }
                            }
                            Err(err) => panic!("{}", err),
                        };
                        // stream
                        //     .write_all(&message)
                        //     .expect("Failed to send response back to client.");
                    }

                    // Remove user from the users array after connection has been terminated
                    match users.lock() {
                        Ok(mut users) => {
                            // TODO: Make removal and adding of new users more robust
                            users.remove(user_index);
                            println!("User has disconnected. {} users left.", users.len());
                        }
                        Err(err) => panic!("{}", err),
                    };
                });
            }
            Err(err) => {
                println!("Error: {}", err);
            }
        }
    }
}

fn send_msg(users: &mut Vec<User>, index: usize, message: &[u8]) {
    for i in 0..users.len() {
        if i != index {
            users[i]
                .stream
                .write_all(message)
                .expect("Failed to send response to client.");
        }
    }
}
