#![allow(unused_must_use)]
#![allow(dead_code)]

mod tmi;
mod demc;
mod keystroke;

extern crate regex;
extern crate toml;
extern crate time;

use std::fs::File;
use std::fs::OpenOptions;
use std::io::Read;
use std::io::Write;
use std::path::Path;

use demc::{DemGcnC, ChatInterfaced};


const CONFIG_FILE_PATH: &'static str = "tppm.toml";
const CHAT_LOG_PATH: &'static str = "chat.txt";
const VJOY_DEVICE_NUMBER: u32 = 1;



fn handle_mod_commands(sender: &String, msg: &String) {
    match sender.to_lowercase().as_ref() {
        "twitchplayspapermario"|"xxn1"|"kalarmar" => {
            match msg.to_lowercase().as_ref() {
                "!savestate" => keystroke::press_key(keystroke::Key::Physical(keystroke::Physical::F5)),
                "!loadstate" => keystroke::press_key(keystroke::Key::Physical(keystroke::Physical::F7)),
                _ => ()
            }
        },
        _ => ()
    }
}


fn main() {
    // Initialize a democratized virtual controller
    let controller = match DemGcnC::new(VJOY_DEVICE_NUMBER) {
        Ok(controller) => controller,
        Err(err) => panic!("Unable to create virtual controller: DemC error {}", err)
    };

    // Start our IRC connection
    let tmi_stream = match tmi::TmiStream::establish(CONFIG_FILE_PATH) {
        Ok(stream) => stream,
        Err(err) => panic!("Unable to establish TMI stream: TMI error {}", err)
    };

    let chat_log_path = Path::new(CHAT_LOG_PATH);
    let mut chat_log_file = match OpenOptions::new().read(true).write(true).append(true).create(true).
                                  open(&chat_log_path)
    {
        Ok(file) => file,
        Err(reason) => panic!("Couldn't open chat log file for writing! {}", std::error::Error::description(&reason))
    };

    // Poll the IRC connection and handle its messages forever
    loop {
        match tmi_stream.receive() {
            Ok((sender, message)) => {
                let log_string = match controller.handle_commands(&message) {
                    Ok(_) => format!("_{}: {}", sender, message),
                    Err(_) => format!("{}: {}", sender, message)
                };
                
                handle_mod_commands(&sender, &message);

                chat_log_file.write_all(&log_string.as_bytes());
                chat_log_file.write_all("\r\n".as_bytes());
                chat_log_file.flush();
                println!("{}", log_string);
            },
            Err(_) => ()
        }
    }
}
