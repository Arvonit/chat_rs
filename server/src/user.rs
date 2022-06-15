use std::net::{IpAddr, TcpStream};

use uuid::Uuid;

#[derive(Debug)]
pub struct User {
    pub id: Uuid,
    pub nickname: Option<String>,
    pub username: Option<String>,
    pub hostname: String,
    pub channel: Option<Channel>,
    pub is_registered: bool,
    pub is_away: bool,
    pub stream: TcpStream,
}

#[derive(Debug, Clone)]
pub struct Channel {
    pub id: Uuid,
    pub name: String,
}

impl User {
    pub fn new(hostname: IpAddr, writer: TcpStream) -> Self {
        User {
            id: Uuid::new_v4(),
            nickname: None,
            username: None,
            hostname: hostname.to_string(),
            channel: None,
            is_registered: false,
            is_away: false,
            stream: writer,
        }
    }

    pub fn prefix(&self) -> Option<String> {
        if let (Some(nickname), Some(username)) = (&self.nickname, &self.username) {
            Some(format!("{}!{}@{}", nickname, username, self.hostname))
        } else {
            None
        }
    }
}

impl Channel {
    pub fn new(name: &str) -> Channel {
        Channel {
            id: Uuid::new_v4(),
            name: name.to_string(),
        }
    }
}
