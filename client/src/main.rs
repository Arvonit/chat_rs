#![allow(unused)]

use std::{
    env,
    io::{self, Read, Write},
    net::TcpStream,
    process::exit,
    str, thread,
};

fn main() {
    env_logger::init();

    // Get username from command-line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("Usage: client <username>");
        exit(1);
    }
    let hostname = "127.0.0.1:8080";
    let username = &args[1];

    // Connect to the server
    let mut reader = TcpStream::connect(hostname).expect("Couldn't connect to the server!");
    let mut writer = reader.try_clone().expect("Failed to clone stream");

    // Spawn another thread to read from stdin and send to the server
    let send_handle = thread::spawn(move || loop {
        // Read from stdin
        let mut message = String::new();
        print!("> ");
        io::stdout().flush().expect("Failed to flush stdout.");
        io::stdin()
            .read_line(&mut message)
            .expect("Failed to read from stdin.");

        // Send message to server
        writer
            .write_all(message.as_bytes())
            .expect("Failed to send message to server.");

        // Exit if user wishes to
        let message = message.trim_end();
        if message == "quit" || message == "exit" {
            break;
        }
    });

    // Spawn a thread to read from the server
    let recv_handle = thread::spawn(move || loop {
        // Read response from server
        let mut response = vec![0; shared::MESSAGE_SIZE];
        match reader.read(&mut response) {
            Ok(bytes) => {
                if bytes == 0 {
                    print!("\r");
                    io::stdout().flush().expect("Failed to flush stdout.");
                    break;
                }
            }
            Err(err) => panic!("{}", err),
        };

        // Convert response to a `str` and print it out
        // TODO: Figure out a way to avoid this mess
        let response_str =
            str::from_utf8(&response).expect("Server sent an invalid UTF-8 message.");
        let response_str = response_str.replace('\0', "");
        let response_str = response_str.trim_end();

        print!("\r"); // Clear the current line; TODO: this needs some work
        println!("<Server> {:?}", response_str);
        print!("> ");
        io::stdout().flush().expect("Failed to flush stdout.");
    });

    // Wait for both threads to terminate
    send_handle.join();
    recv_handle.join();
}
