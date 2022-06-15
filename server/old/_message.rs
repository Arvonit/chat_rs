#![allow(non_camel_case_types)]
#![allow(unused)]

use std::{
    fmt::{Display, Formatter, Write},
    io::{Error, ErrorKind},
    str::FromStr,
};

#[derive(Debug)]
pub struct Message {
    pub raw: String,
    pub prefix: String,
    pub command: String,
    pub parameters: Vec<String>,
}

pub trait ToIrcMessage: ToString {
    fn to_irc_message(&self) -> String {
        format!("{}\r\n", self.to_string())
    }
}

// #[derive(Debug)]
// pub struct Prefix {
//     pub nickname: String,
//     pub username: String,
//     pub hostname: String,
// }

#[derive(Debug)]
pub enum Command {
    Nick,
    Join,
    Kick,
    Part,
    PrivMsg,
    List,
    Away,
    Quit,
}

#[derive(Debug, PartialEq)]
pub struct Response {
    pub prefix: String,
    pub code: u16,
    pub message: Option<String>,
}

pub const INVALID_MESSAGE_RESPONSE: Response = Response {
    prefix: String::new(),
    code: 0,
    message: None,
};

// TODO: Add more codes
pub enum ReplyCode {
    RPL_WELCOME = 001,
    RPL_YOURHOST = 002,
    RPL_CREATED = 003,
    RPL_MYINFO = 004,
    // RPL_LUSERCLIENT = 251,
    // RPL_LUSEROP = 252,
    // RPL_LUSERUNKNOWN = 253,
    // RPL_LUSERCHANNELS = 254,
    // RPL_LUSERME = 255,
    RPL_AWAY = 301,
    RPL_UNAWAY = 305,
    RPL_NOWAWAY = 306,
    RPL_WHOISUSER = 311,
    RPL_WHOISSERVER = 312,
    RPL_WHOISOPERATOR = 313,
    RPL_WHOISIDLE = 317,
    RPL_ENDOFWHOIS = 318,
    RPL_WHOISCHANNELS = 319,
    RPL_WHOREPLY = 352,
    RPL_ENDOFWHO = 315,
    RPL_LIST = 322,
    RPL_LISTEND = 323,
    RPL_CHANNELMODEIS = 324,
    RPL_NOTOPIC = 331,
    RPL_TOPIC = 332,
    RPL_NAMREPLY = 353,
    RPL_ENDOFNAMES = 366,
    RPL_MOTDSTART = 375,
    RPL_MOTD = 372,
    RPL_ENDOFMOTD = 376,
    RPL_YOUREOPER = 381,

    ERR_NOSUCHNICK = 401,
    ERR_NOSUCHSERVER = 402,
    ERR_NOSUCHCHANNEL = 403,
    ERR_CANNOTSENDTOCHAN = 404,
    ERR_NORECIPIENT = 411,
    ERR_NOTEXTTOSEND = 412,
    ERR_UNKNOWNCOMMAND = 421,
    ERR_NOMOTD = 422,
    ERR_NONICKNAMEGIVEN = 431,
    ERR_NICKNAMEINUSE = 433,
    ERR_USERNOTINCHANNEL = 441,
    ERR_NOTONCHANNEL = 442,
    ERR_NOTREGISTERED = 451,
    ERR_NEEDMOREPARAMS = 461,
    ERR_ALREADYREGISTRED = 462,
    ERR_PASSWDMISMATCH = 464,
    ERR_UNKNOWNMODE = 472,
    ERR_NOPRIVILEGES = 481,
    ERR_CHANOPRIVSNEEDED = 482,
    ERR_UMODEUNKNOWNFLAG = 501,
    ERR_USERSDONTMATCH = 502,
}

impl Message {
    /// Parse an IRC message from a raw string. Return a message if successful and a response if
    /// there was an error in parsing.
    pub fn from(raw_message: &str) -> Result<Self, Response> {
        let text = raw_message;

        // Message must begin with ':' to indicate username, realname, and hostname
        if !text.starts_with(":") {
            return Err(INVALID_MESSAGE_RESPONSE);
        }

        // Prefix
        let (_, text) = text.split_once(":").unwrap();
        let (prefix, text) = text.split_once(" ").unwrap();
        let prefix = String::from(prefix);

        // Command
        let (command, text) = text.split_once(" ").unwrap();
        let command = match Command::from_str(command) {
            Ok(_) => command.to_uppercase(), // Make sure command is uppercase
            Err(err) => {
                return Err(Response {
                    prefix: String::from(prefix),
                    code: ReplyCode::ERR_UNKNOWNCOMMAND as u16,
                    message: Some(String::from("Unknown command.")),
                });
            }
        };

        // Parameters
        let mut parameters = vec![];
        let mut text = text;
        loop {
            if !text.contains(" ") {
                // println!("{text:?}");
                parameters.push(String::from(text));
                break;
            }
            // let delimeter = "";
            if text.starts_with(":") {
                let (_, param) = text.split_once(":").unwrap();
                parameters.push(String::from(param));
                // println!("{param}");
                text = param;
                break;
            } else {
                let (param, rest) = text.split_once(" ").unwrap();
                parameters.push(String::from(param));
                // println!("{param}");
                text = rest;
            }
        }

        Ok(Message {
            raw: String::from(raw_message),
            prefix,
            command,
            parameters,
        })
    }
}

impl FromStr for Command {
    type Err = Error;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input.to_uppercase().as_str() {
            "NICK" => Ok(Command::Nick),
            "JOIN" => Ok(Command::Join),
            "KICK" => Ok(Command::Kick),
            "PART" => Ok(Command::Part),
            "PRIVMSG" => Ok(Command::PrivMsg),
            "LIST" => Ok(Command::List),
            "AWAY" => Ok(Command::Away),
            "QUIT" => Ok(Command::Quit),
            _ => Err(Error::new(ErrorKind::InvalidInput, "Unknown command.")),
        }
    }
}

impl Display for Message {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.raw)
    }
}

impl ToIrcMessage for Message {}

impl Display for Response {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(message) = &self.message {
            write!(f, ":{} {:03} :{}", self.prefix, self.code, message)
        } else {
            write!(f, ":{} {:03}", self.prefix, self.code)
        }
    }
}

impl ToIrcMessage for Response {}
