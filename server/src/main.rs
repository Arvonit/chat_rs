// #![allow(unused)]
#![feature(result_option_inspect)]

mod message;
mod server;
mod user;

use std::{
    collections::HashMap,
    net::TcpListener,
    sync::{Arc, Mutex},
    thread,
};
use user::{Channel, User};
use uuid::Uuid;

fn main() {
    let port = 8080;
    let hostname = &format!("127.0.0.1:{port}"); // TODO: Allow for custom port
    let listener = TcpListener::bind(hostname).expect(&format!("Couldn't bind to {hostname}."));
    println!("Listening on {hostname}.");

    let users = Arc::new(Mutex::new(HashMap::<Uuid, User>::new()));
    let channels = Arc::new(Mutex::new(HashMap::<String, Channel>::new()));

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        let users = users.clone();
        let channels = channels.clone();

        thread::spawn(move || server::handle_connection(stream, users, channels, "127.0.0.1"));
    }
}
