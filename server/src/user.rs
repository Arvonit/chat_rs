use std::{io::Error, net::TcpStream};

#[derive(Debug)]
pub struct User {
    pub nickname: Option<String>,
    pub username: Option<String>,
    pub realname: Option<String>,
    pub hostname: Option<String>,
    pub channel: Option<Channel>,
    pub is_registered: bool,
    pub is_away: bool,
    pub stream: TcpStream,
}

#[derive(Debug, Clone)]
pub struct Channel {
    pub name: String,
}

impl User {
    pub fn new(writer: TcpStream) -> User {
        User {
            nickname: None,
            username: None,
            realname: None,
            hostname: None,
            channel: None,
            is_registered: false,
            is_away: false,
            stream: writer,
        }
    }

    pub fn prefix(&self) -> Option<String> {
        if let (Some(nickname), Some(username), Some(hostname)) =
            (&self.nickname, &self.username, &self.hostname)
        {
            Some(format!("{}!{}@{}", nickname, username, hostname))
        } else {
            None
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
