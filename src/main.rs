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
use std::thread;

use demc::{DemC, ChatInterfaced};
use demc::vgcnc::{VGcnC, sample_gcn_controller_hardware};


const CONFIG_FILE_PATH: &'static str = "tppm.toml";
const CHAT_LOG_PATH: &'static str = "chat.txt";
const VJOY_DEVICE_NUMBER: u32 = 1;
const ACCEPT_CONTROLLER_COMMANDS_ON_BOOT: bool = true;


enum ModCommand {
    SaveState,
    LoadState,
    UnplugController,
    PlugController
}
fn parse_mod_commands(sender: &String, msg: &String) -> Option<ModCommand> {
    match sender.to_lowercase().as_ref() {
        "twitchplayspapermario"|"xxn1"|"kalarmar"|"rashama_izouki"|"mooismyusername" => {
            match msg.to_lowercase().as_ref() {
                "!savestate" => Some(ModCommand::SaveState),
                "!loadstate" => Some(ModCommand::LoadState),
                "!unplugcontroller" => Some(ModCommand::UnplugController),
                "!plugcontroller" => Some(ModCommand::PlugController),
                _ => None
            }
        },
        _ => None
    }
}


enum ChatMessageHandler {
    ModCommandHandler,
    ControllerCommandHandler,
}
fn log_tmi_message(sender: &String, message: &String, handler: &Option<ChatMessageHandler>, log: &mut File) {
    let log_string = match handler {
        &Some(ref handler) => match handler {
            &ChatMessageHandler::ModCommandHandler => format!("!{}: {}", sender, message),
            &ChatMessageHandler::ControllerCommandHandler => format!("_{}: {}", sender, message),
        },
        &None => format!("{}: {}", sender, message)
    };

    log.write_all(&log_string.as_bytes());
    log.write_all("\r\n".as_bytes());
    log.flush();
    println!("{}", log_string);
}

fn handle_tmi_message<T>(sender: &String, message: &String, accepting_controller_commands: bool,
                         controller: &DemC<T>, log: &mut File) -> Option<bool>
{
    let mut message_handler = None;
    let mut new_accept_controller_command_value = None;
    
    if !message_handler.is_some() {
        match parse_mod_commands(sender, message) {
            Some(mod_command) => {
                match mod_command {
                    ModCommand::SaveState => {
                        keystroke::press_key(keystroke::Key::Scan(keystroke::Scan::F1));
                        thread::sleep_ms(500);
                        keystroke::release_key(keystroke::Key::Scan(keystroke::Scan::F1));
                    },
                    ModCommand::LoadState => {
                        keystroke::press_key(keystroke::Key::Scan(keystroke::Scan::F7));
                        thread::sleep_ms(500);
                        keystroke::release_key(keystroke::Key::Scan(keystroke::Scan::F7));
                    },
                    ModCommand::UnplugController => {
                        new_accept_controller_command_value = Some(false);
                    },
                    ModCommand::PlugController => {
                        new_accept_controller_command_value = Some(true);
                    }
                }
                message_handler = Some(ChatMessageHandler::ModCommandHandler);
            },
            None => ()
        }
    }

    if !message_handler.is_some() {
        match accepting_controller_commands {
            true => match controller.handle_commands(message) {
                Ok(_) => {
                    message_handler = Some(ChatMessageHandler::ControllerCommandHandler);
                },
                Err(_) => ()
            },
            false => ()
        };
    }
    
    log_tmi_message(sender, message, &message_handler, log);
    
    new_accept_controller_command_value
}




fn main() {
    let (axes, joysticks, buttons) = sample_gcn_controller_hardware(VJOY_DEVICE_NUMBER).unwrap();
    let raw_controller = match VGcnC::new(VJOY_DEVICE_NUMBER, axes, joysticks, buttons) {
        Ok(controller) => controller,
        Err(err) => panic!("Unable to make raw controller: err {}", err)
    };

    // Initialize a democratized virtual controller
    let controller = match DemC::new(raw_controller, demc::ControllerConstraints {
        illegal_combinations: vec![
                                (String::from("start"), vec!(String::from("b"), String::from("x"))),
                                (String::from("b"), vec!(String::from("start"), String::from("x"))),
                                (String::from("x"), vec!(String::from("b"), String::from("start")))] } )
    {
        Ok(controller) => controller,
        Err(err) => panic!("Unable to create democratized controller: DemC error {}", err)
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
    
    let mut accepting_controller_commands = ACCEPT_CONTROLLER_COMMANDS_ON_BOOT;

    // Poll the IRC connection and handle its messages forever
    loop {
        match tmi_stream.receive() {
            Ok((sender, message)) => {
                match handle_tmi_message(&sender, &message, accepting_controller_commands, &controller, &mut chat_log_file) {
                    Some(val) => { accepting_controller_commands = val; },
                    None => ()
                }
            },
            Err(_) => ()
        }
    }
}
