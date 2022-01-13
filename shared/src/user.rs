use std::net::TcpStream;
use uuid::Uuid;

#[derive(Debug)]
pub struct User {
    pub id: Uuid,
    pub is_registered: bool,
    pub is_away: bool,
    pub name: String,
    pub nickname: String,
    pub hostname: String,
    pub channel: Option<Channel>,
    pub stream: TcpStream,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Channel {
    pub name: String,
}

impl User {
    pub fn new(name: &str, nickname: &str, hostname: &str, stream: TcpStream) -> User {
        User {
            id: Uuid::new_v4(),
            is_registered: false,
            is_away: false,
            name: name.to_string(),
            nickname: nickname.to_string(),
            hostname: hostname.to_string(),
            channel: None,
            stream,
        }
    }
}

impl Channel {
    pub fn new(name: &str) -> Channel {
        Channel {
            name: name.to_string(),
        }
    }
}
