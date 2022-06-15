use crate::{
    message::ToIrc,
    user::{Channel, User},
};
use std::{
    borrow::BorrowMut,
    collections::HashMap,
    io::{self, Error, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread,
};
use uuid::Uuid;

pub struct Server {
    socket: TcpListener,
    users: Arc<Mutex<HashMap<Uuid, User>>>,
    channels: Vec<Channel>,
}

pub struct ClientTable {
    pub users: Mutex<HashMap<Uuid, User>>,
    pub prefix: String,
    // channels: Mutex<HashMap<Uuid, Channel>>,
}

impl ClientTable {
    pub fn new(prefix: &str) -> Self {
        Self {
            users: Mutex::new(HashMap::new()),
            prefix: prefix.to_string(), // channels: Mutex::new(HashMap::new()),
        }
    }

    pub fn add_user(&mut self, user: User) {
        self.users.lock().unwrap().insert(user.id, user);
    }

    pub fn remove_user(&mut self, id: Uuid) {
        self.users.lock().unwrap().remove(&id);
    }

    pub fn broadcast_message<T: ToIrc>(&mut self, message: &T, id_to_exclude: Uuid) {
        self.users
            .lock()
            .unwrap()
            .iter_mut()
            .filter(|(id, user)| **id != id_to_exclude)
            .for_each(|(id, user)| user.stream.write_all(message.to_irc().as_bytes()).unwrap());
    }

    pub fn users_len(&self) -> usize {
        self.users.lock().unwrap().len()
    }
}

impl Server {
    pub fn new(address: &str) -> Result<Self, Error> {
        Ok(Self {
            socket: TcpListener::bind(address)?,
            users: Arc::new(Mutex::new(HashMap::new())),
            channels: vec![],
        })
    }

    /// The main loop and logic for the server
    pub fn start(&self) {
        for stream in self.socket.incoming() {
            let stream = match stream {
                Ok(stream) => stream,
                Err(err) => {
                    eprintln!("Could not accept connection: {err}");
                    continue; // Wait for next connection
                }
            };

            let c = ClientTable::new("");

            // Spawn a new thread to handle this client
            // thread::spawn(move || self.handle_client());
        }
    }

    fn handle_client(&mut self) {}

    // fn add_user(&mut self, user: User) {
    //     self.users.insert(Uuid::new_v4(), user);
    // }

    // fn remove_user(&mut self, id: &Uuid) {
    //     self.users.remove(id);
    // }

    // fn create_channel(&mut self, name: &str) {
    //     self.channels.push(Channel::new(name));
    // }
}
