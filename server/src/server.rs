use crate::{
    message::{Command, Message, ReplyCode, Response, ToIrc},
    user::{Channel, User},
};
use std::{
    collections::HashMap,
    io::{Read, Write},
    net::TcpStream,
    str::{self},
    sync::{Arc, Mutex},
};
use uuid::Uuid;

type UserTable = Mutex<HashMap<Uuid, User>>;
type ChannelTable = Mutex<HashMap<String, Arc<Channel>>>;

#[derive(PartialEq)]
enum CommandResponse {
    Continue,
    Quit,
}

pub fn handle_connection(
    mut stream: TcpStream,
    users: Arc<UserTable>,
    channels: Arc<ChannelTable>,
    hostname: &str,
) {
    let address = stream
        .local_addr()
        .expect("Failed to get IP address of client socket.")
        .ip();

    // Add new user to the table
    let user_id = {
        let mut lock = users.lock().expect("Failed to lock the users table.");
        let user = User::new(address, stream.try_clone().unwrap());
        let id = user.id;
        lock.insert(id, user);
        println!(
            "New connection from {}. {} active connections.",
            address,
            lock.len()
        );
        id
    };

    loop {
        // Wait for message from client
        let mut message_ascii = vec![0; shared::MESSAGE_SIZE];
        stream
            .read(&mut message_ascii)
            .expect("Failed to read message from client.");

        // Convert `message` to a String and print it out
        let message_str = str::from_utf8(&message_ascii)
            .expect("Client sent an invalid UTF-8 message.")
            .replace('\0', "");
        println!("{:?}", message_str);

        // Extract IRC command from client input
        let message = match Message::from(&message_str) {
            Ok(message) => message,
            Err(err) => {
                // TODO: Fix reply code
                let response =
                    Response::new(hostname, ReplyCode::ERR_UNKNOWNCOMMAND, &[&err.to_string()]);
                send_to_user(&response, &users, user_id).expect("Failed to send message.");
                continue;
            }
        };

        if handle_message(message, &users, &channels, user_id, hostname)
            .expect("Failed to parse command.")
            == CommandResponse::Quit
        {
            break;
        }
    }

    // Remove user from the table
    users
        .lock()
        .expect("Unable to get lock on users table.")
        .remove(&user_id);
}

fn handle_message<'a>(
    mut message: Message,
    users: &'a UserTable,
    channels: &'a ChannelTable,
    user_id: Uuid,
    server_prefix: &str,
) -> Result<CommandResponse, Box<dyn std::error::Error + 'a>> {
    // Update message's prefix to the user's in case we need to broadcast this message to other
    // users
    message.prefix = users
        .lock()
        .expect("Unable to get a lock on the users table.")
        .get(&user_id)
        .unwrap()
        .prefix();

    // In order for a user to become registered, the client has to send a NICK message with a valid
    // nickname and a USER message with their username. If all checks pass, they will receieve a
    // welcome message.

    // Only allow USER, NICK, and QUIT commands if user is not registered
    if !users.lock().unwrap().get(&user_id).unwrap().is_registered
        && !matches!(
            message.command,
            Command::User | Command::Nick | Command::Quit
        )
    {
        let response = Response::new(
            server_prefix,
            ReplyCode::ERR_NOTREGISTERED,
            &["You have not registered."],
        );
        send_to_user(&response, &users, user_id)?;
        return Ok(CommandResponse::Continue);
    }

    // Perform command associated with message
    match message.command {
        Command::User => {
            // Example: USER guest 0 * :Ronnie Reagan

            // We will only parse the first argument (username) and ignore the rest
            let username = match message.params.get(0) {
                Some(name) => name.clone(),
                None => {
                    let response = Response::new(
                        server_prefix,
                        ReplyCode::ERR_NONICKNAMEGIVEN,
                        &["No nickname was given."],
                    );
                    send_to_user(&response, &users, user_id)?;
                    // user.stream.write_all(response.to_irc().as_bytes())?;

                    return Ok(CommandResponse::Continue);
                }
            };

            let mut lock = users.lock().expect("Unable to get lock on users table.");
            let mut user = lock.get_mut(&user_id).unwrap();

            // If the user is already registered, ignore the request and send ERR_ALREADYREGISTERED
            if user.is_registered {
                drop(lock);
                let response = Response::new(
                    server_prefix,
                    ReplyCode::ERR_ALREADYREGISTRED,
                    &["Cannot send USER message since the client is already registered."],
                );

                // Send response to client
                // user.stream.write_all(response.to_irc().as_bytes())?;

                send_to_user(&response, &users, user_id)?;
                return Ok(CommandResponse::Continue);
            }

            user.username = Some(username);
        }
        Command::Nick => {
            // Example: NICK Wiz

            // Get the first parameter in the message
            let nickname = match message.params.get(0) {
                Some(name) => name.clone(),
                None => {
                    let response = Response::new(
                        server_prefix,
                        ReplyCode::ERR_NONICKNAMEGIVEN,
                        &["No nickname was given."],
                    );
                    // user.stream.write_all(response.to_irc().as_bytes())?;
                    send_to_user(&response, &users, user_id)?;
                    return Ok(CommandResponse::Continue);
                }
            };

            if nickname_in_use(&nickname, &users) {
                let response = Response::new(
                    server_prefix,
                    ReplyCode::ERR_NICKNAMEINUSE,
                    &["Nickname is already in use."],
                );
                // user.stream.write_all(response.to_irc().as_bytes())?;
                send_to_user(&response, &users, user_id)?;
                return Ok(CommandResponse::Continue);
            }

            // Update nickname and get the registration state of the user
            let is_registered = {
                let mut lock = users.lock().expect("Unable to get lock on users table.");
                let mut user = lock.get_mut(&user_id).unwrap();
                user.nickname = Some(nickname);
                user.is_registered
            };

            // Only broadcast NICK message if user is registered
            if is_registered {
                broadcast_to_all(&message, &users)?;
            }
        }
        Command::Away => {
            let mut lock = users.lock().expect("Unable to get lock on users table.");
            let mut user = lock.get_mut(&user_id).unwrap();

            // Toggle away status
            let away = !user.is_away;
            user.is_away = away;
            drop(lock);

            let response = if away {
                Response::new(
                    server_prefix,
                    ReplyCode::RPL_NOWAWAY,
                    &["You are now away."],
                )
            } else {
                Response::new(
                    server_prefix,
                    ReplyCode::RPL_UNAWAY,
                    &["You are no longer away."],
                )
            };
            send_to_user(&response, &users, user_id)?;
        }
        Command::PrivMsg => {
            // Example: PRIVMSG user :Hello there!
            //          PRIVMSG #channel :Hello there!
            if message.params.len() != 2 {
                let response = Response::new(
                    server_prefix,
                    ReplyCode::ERR_NORECIPIENT,
                    &["No recipient for the message was given."],
                );
                send_to_user(&response, &users, user_id)?;
                return Ok(CommandResponse::Continue);
            }

            let recipient = message.params.get(0).unwrap().clone();

            // It's not a channel
            if !recipient.starts_with("#") {
                if let Some(nickname_id) = get_nickname_id(&recipient, &users) {
                    let is_away = users
                        .lock()
                        .expect("Unable to get lock on users table.")
                        .get(&nickname_id)
                        .unwrap()
                        .is_away;
                    if is_away {
                        let response = Response::new(
                            server_prefix,
                            ReplyCode::RPL_AWAY,
                            &[&recipient, "The recipient is marked as away."],
                        );
                        send_to_user(&response, &users, user_id)?;
                    }

                    send_to_user(&message, &users, nickname_id)?;
                } else {
                    let response = Response::new(
                        server_prefix,
                        ReplyCode::ERR_NOSUCHNICK,
                        &["The given nick was not found."],
                    );
                    send_to_user(&response, &users, user_id)?;
                }
            } else {
                if let Some(channel) = channels.lock().unwrap().get(&recipient) {
                    send_to_channel(&message, &users, channel)?;
                } else {
                    let response = Response::new(
                        server_prefix,
                        ReplyCode::ERR_NOSUCHCHANNEL,
                        &["The given channel was not found."],
                    );
                    send_to_user(&response, &users, user_id)?;
                }
            }
        }
        Command::Quit => {
            let acknowledgement_response = Message::new(
                Some(server_prefix.to_string()),
                Command::Error,
                &["User disconnected."],
            );
            send_to_user(&acknowledgement_response, &users, user_id)?;

            // If the user is registered, tell everyone else that the user has left.
            let is_registered = users
                .lock()
                .expect("Unable to get lock on users table.")
                .get(&user_id)
                .unwrap()
                .is_registered;
            if is_registered {
                broadcast_message(&message, &users, user_id)?;
            }

            return Ok(CommandResponse::Quit);
        }
        Command::Unknown => {
            let response = Response::new(
                server_prefix,
                ReplyCode::ERR_UNKNOWNCOMMAND,
                &["Unknown command."],
            );
            send_to_user(&response, &users, user_id)?;
        }
        Command::Join => {
            let channel_name = match message.params.get(0) {
                Some(name) => name.clone(),
                None => {
                    let response = Response::new(
                        server_prefix,
                        ReplyCode::ERR_NEEDMOREPARAMS,
                        &["Specify which channel to join."],
                    );
                    send_to_user(&response, &users, user_id)?;
                    return Ok(CommandResponse::Continue);
                }
            };

            // Get a reference to the channel if it is in the channels table, otherwise create it
            let channel = channels
                .lock()
                .unwrap()
                .entry(channel_name.clone())
                .or_insert(Arc::new(Channel::new(&channel_name)))
                .clone();

            // Set the user's channel to the channel from the table
            users.lock().unwrap().get_mut(&user_id).unwrap().channel = Some(channel);
            // TODO: Broadcast
        }
        // Command::Kick => todo!(),
        Command::Part => {
            let channel_name = match message.params.get(0) {
                Some(name) => name.clone(),
                None => {
                    let response = Response::new(
                        server_prefix,
                        ReplyCode::ERR_NEEDMOREPARAMS,
                        &["Specify which channel to join."],
                    );
                    send_to_user(&response, &users, user_id)?;
                    return Ok(CommandResponse::Continue);
                }
            };

            let exists = channels.lock().unwrap().get(&channel_name).is_some();
            if exists {
                users.lock().unwrap().get_mut(&user_id).unwrap().channel = None;
                // TODO: Broadcast
            } else {
                let response = Response::new(
                    server_prefix,
                    ReplyCode::ERR_NOSUCHCHANNEL,
                    &["The given channel was not found."],
                );
                send_to_user(&response, &users, user_id)?;
            }
        }
        Command::List => todo!(),
        _ => {
            // let response = Response {
            //     prefix: server_prefix.to_string(),
            //     code: ReplyCode::RPL_WELCOME,
            //     params: vec!["Welcome to the Internet Relay Network!".to_string()],
            // };
            // user.stream.write_all(response.to_irc().as_bytes())?;
            send_to_user(&message, &users, user_id)?;
        }
    }

    let mut lock = users.lock().expect("Unable to get lock on users table.");
    let mut user = lock.get_mut(&user_id).unwrap();

    // Send welcome message if user is now registered
    if !user.is_registered && user.prefix() != None {
        user.is_registered = true;
        let response = Response::new(
            &user.prefix().unwrap(),
            ReplyCode::RPL_WELCOME,
            &[
                user.nickname.as_ref().unwrap(),
                &format!(
                    "Welcome to the Internet Relay Network {}",
                    user.prefix().unwrap()
                ),
            ],
        );
        user.stream.write_all(response.to_irc().as_bytes())?;
    }

    drop(lock);

    Ok(CommandResponse::Continue)
}

fn handle_user<'a>(
    message: Message,
    users: &'a UserTable,
    user_id: Uuid,
    server_prefix: &str,
) -> Result<CommandResponse, Box<dyn std::error::Error + 'a>> {
    // Example: USER guest 0 * :Ronnie Reagan

    // We will only parse the first argument (username) and ignore the rest
    let username = match message.params.get(0) {
        Some(name) => name.clone(),
        None => {
            let response = Response::new(
                server_prefix,
                ReplyCode::ERR_NONICKNAMEGIVEN,
                &["No nickname was given."],
            );
            send_to_user(&response, &users, user_id)?;
            // user.stream.write_all(response.to_irc().as_bytes())?;

            return Ok(CommandResponse::Continue);
        }
    };

    let mut lock = users.lock().expect("Unable to get lock on users table.");
    let mut user = lock.get_mut(&user_id).unwrap();

    // If the user is already registered, ignore the request and send ERR_ALREADYREGISTERED
    if user.is_registered {
        drop(lock);
        let response = Response::new(
            server_prefix,
            ReplyCode::ERR_ALREADYREGISTRED,
            &["Cannot send USER message since the client is already registered."],
        );

        // Send response to client
        // user.stream.write_all(response.to_irc().as_bytes())?;

        send_to_user(&response, &users, user_id)?;
        return Ok(CommandResponse::Continue);
    }

    user.username = Some(username);
    return Ok(CommandResponse::Continue);
}

fn handle_nick<'a>(
    message: Message,
    users: &'a UserTable,
    user_id: Uuid,
    server_prefix: &str,
) -> Result<CommandResponse, Box<dyn std::error::Error + 'a>> {
    // Example: NICK Wiz

    // Get the first parameter in the message
    let nickname = match message.params.get(0) {
        Some(name) => name.clone(),
        None => {
            let response = Response::new(
                server_prefix,
                ReplyCode::ERR_NONICKNAMEGIVEN,
                &["No nickname was given."],
            );
            // user.stream.write_all(response.to_irc().as_bytes())?;
            send_to_user(&response, &users, user_id)?;
            return Ok(CommandResponse::Continue);
        }
    };

    if nickname_in_use(&nickname, &users) {
        let response = Response::new(
            server_prefix,
            ReplyCode::ERR_NICKNAMEINUSE,
            &["Nickname is already in use."],
        );
        // user.stream.write_all(response.to_irc().as_bytes())?;
        send_to_user(&response, &users, user_id)?;
        return Ok(CommandResponse::Continue);
    }

    let mut lock = users.lock().expect("Unable to get lock on users table.");
    let mut user = lock.get_mut(&user_id).unwrap();
    user.nickname = Some(nickname);
    let is_registered = user.is_registered;
    drop(lock);

    // Only broadcast NICK message if user is registered
    if is_registered {
        broadcast_to_all(&message, &users)?;
        // broadcast_message(&message, users);
    }
    return Ok(CommandResponse::Continue);
}

pub fn send_to_user<'a, T: ToIrc>(
    message: &T,
    users: &'a UserTable,
    id: Uuid,
) -> Result<(), Box<dyn std::error::Error + 'a>> {
    Ok(users
        .lock()?
        .get_mut(&id)
        .unwrap()
        .stream
        .write_all(message.to_irc().as_bytes())?)
}

pub fn send_to_channel<'a, T: ToIrc>(
    message: &T,
    users: &'a UserTable,
    channel: &Arc<Channel>,
) -> Result<(), Box<dyn std::error::Error + 'a>> {
    Ok(users
        .lock()?
        .iter_mut()
        .filter(|(_, user)| user.channel == Some(channel.clone()))
        .for_each(|(_, user)| user.stream.write_all(message.to_irc().as_bytes()).unwrap()))
}

pub fn broadcast_message<'a, T: ToIrc>(
    message: &T,
    users: &'a UserTable,
    id_to_exclude: Uuid,
) -> Result<(), Box<dyn std::error::Error + 'a>> {
    Ok(users
        .lock()?
        // .unwrap()
        .iter_mut()
        .filter(|(id, _)| **id != id_to_exclude)
        .for_each(|(_, user)| user.stream.write_all(message.to_irc().as_bytes()).unwrap()))
}

pub fn broadcast_to_all<'a, T: ToIrc>(
    message: &T,
    users: &'a UserTable,
) -> Result<(), Box<dyn std::error::Error + 'a>> {
    Ok(users
        .lock()?
        .iter_mut()
        .for_each(|(_, user)| user.stream.write_all(message.to_irc().as_bytes()).unwrap()))
}

pub fn nickname_in_use(nickname: &str, users: &UserTable) -> bool {
    for (_, user) in users.lock().unwrap().iter() {
        if let Some(name) = &user.nickname {
            if name == nickname {
                return true;
            }
        }
    }

    return false;
}

pub fn get_nickname_id(nickname: &str, users: &UserTable) -> Option<Uuid> {
    for (id, user) in users.lock().unwrap().iter() {
        if let Some(name) = &user.nickname {
            if name == nickname {
                return Some(*id);
            }
        }
    }

    return None;
}
