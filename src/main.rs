#![allow(unused_must_use)]

mod irc;
mod tmi;
mod vn64c;
mod demc;

#[macro_use]
extern crate regex;
extern crate toml;
extern crate time;

use std::fs::File;
use std::io::Read;

use regex::Regex;

use time::Duration;

use vn64c::{Controller, ButtonName};
use demc::DemC;
use demc::TimedInputCommand;
use vn64c::InputCommand;


const CONFIG_FILE_PATH: &'static str = "tppm.toml";
const VJOY_DEVICE_NUMBER: u8 = 1;
const MAX_JOYSTICK_COMMAND_DURATION: u32 = 5000;
const MAX_BUTTON_COMMAND_DURATION: u32 = 5000;


// Parse the TPPM toml configuration file; return the server, password, nick, and channel
fn parse_config_file() -> (String, String, String, String) {
    let mut config_file = File::open(CONFIG_FILE_PATH).unwrap();
    let mut config_string = String::new();
    config_file.read_to_string(&mut config_string);
    
    //@todo understand this generics magic
    let toml_tree: toml::Value = config_string.parse().unwrap();
    
    let server = String::from(toml_tree.lookup("irc.server").unwrap().as_str().unwrap());
    let pass = String::from(toml_tree.lookup("irc.pass").unwrap().as_str().unwrap());
    let nick = String::from(toml_tree.lookup("irc.nick").unwrap().as_str().unwrap());
    let channel = String::from(toml_tree.lookup("irc.channel").unwrap().as_str().unwrap());
    
    (server, pass, nick, channel)
}


// Attempt to parse an IRC message into a list of controller commands
//@todo just use a custom state machine rather than regex, this has to be insanely slow
//@todo this function is huge
fn parse_string_as_commands(msg: &String, re: &Regex) -> Option<Vec<TimedInputCommand>> {
    let mut last_cap_end = None;
    let mut cumulative_delay: u32 = 0;
    let mut last_command: Option<TimedInputCommand> = None;
    let mut cap_start_zero = false;
    let mut final_cap_end: usize = 0;
    let mut res: Vec<TimedInputCommand> = Vec::new();
    
    for cap in re.captures_iter(&msg.to_lowercase()) {
        let (cap_start, cap_end) = cap.pos(0).unwrap();

        // Raise a flag if this capture starts at the first string position - after iterating over all captures, we'll
        // make sure it was raised, and if it wasn't, we didn't parse a valid command message
        if cap_start == 0 {
            cap_start_zero = true;
        }

        // Make sure that all captures are continuous; that we're parsing commands and only commands
        // eg. we don't want "hahah" to parse as two "a" commands
        if let Some(last_cap_end) = last_cap_end {
            if last_cap_end != cap_start {
                return None;
            }
        }

        // Store the last capture's ending position - after iterating over all captures, we'll make sure the last one
        // ended
        final_cap_end = cap_end;

        // Our regex should match on exactly one of three groups: "joystick", "button", or "delay"
        if let Some(_) = cap.name("joystick") {
            match last_command {
                Some(command) => {
                    cumulative_delay += command.duration.num_milliseconds() as u32;
                    match command.command {
                        InputCommand::Joystick{direction: _, strength: _} => {
                            cumulative_delay += 51;
                        },
                        InputCommand::Button{name: _, value: _} => {
                            ()
                        }
                    }
                },
                None =>  ()
            }
            // joystick command - should have one, two, or four groups:
            // "joystick_strength" (optional)
            // "joystick_direction" (mandatory)
            // "joystick_duration" (optional),
            // "joystick_duration_units" (optional; must be present if joystick_duration is)
            let mut joystick_strength: f32 = 1.0;
            let mut joystick_direction: u16 = 0;
            let mut joystick_duration: u32 = 500;
            if let Some(jscap) = cap.name("joystick_strength") {
                match jscap.parse::<u8>() {
                    Ok(strength_u8) => { joystick_strength = strength_u8 as f32 / 100.0; },
                    _ => return None
                }
            }
            if let Some(jdcap) = cap.name("joystick_direction") {
                match jdcap {
                    "up" => { joystick_direction = 90; },
                    "down" => { joystick_direction = 270; },
                    "left" => { joystick_direction = 180; },
                    "right" => { joystick_direction = 0; },
                    _ => ()
                }
            } else {
                return None;
            }
            if let Some(jdcap) = cap.name("joystick_duration") {
                match jdcap.parse::<u32>() {
                    Ok(duration_u32) => { joystick_duration = duration_u32; },
                    _ => return None
                }
                
                if let Some(jdcap_units) = cap.name("joystick_duration_units") {
                    if jdcap_units == "s" {
                        joystick_duration *= 1000;
                    }
                } else {
                    return None;
                }
            }
            
            // treat joystick commands with strength <0%, >100% or duration >5s as invalid
            if joystick_strength > 1.0 || joystick_strength < 0.0 || joystick_duration > MAX_JOYSTICK_COMMAND_DURATION {
                return None;
            }
            
            let time_now = time::get_time();
            let command = TimedInputCommand { start_time: time_now + time::Duration::milliseconds(cumulative_delay as i64),
                                                             duration: time::Duration::milliseconds(joystick_duration as i64),
                                                             command: InputCommand::Joystick { direction: joystick_direction,
                                                                                               strength: joystick_strength} };
            res.push(command);
            
            last_command = Some(command.clone());
        } else if let Some(_) = cap.name("button") {
            match last_command {
                Some(command) => {
                    match command.command {
                        InputCommand::Joystick{direction: _, strength: _} => {
                            cumulative_delay += command.duration.num_milliseconds() as u32;
                            if command.duration.num_milliseconds() >= 17 {
                                cumulative_delay -= 17;
                            }
                        },
                        InputCommand::Button{name: _, value: _} => {
                            if command.duration.num_milliseconds() == 167 {
                                cumulative_delay += 500; //massive hack
                            } else {
                                cumulative_delay += command.duration.num_milliseconds() as u32;
                            }
                            cumulative_delay += 51;
                        }
                    }
                },
                None =>  ()
            }
            // button command - should have one or three groups:
            // "button_name" (mandatory)
            // "button_duration" (optional),
            // "button_duration_units" (optional; must be present if joystick_duration is)
            let mut button_name;
            let mut button_duration: u32 = 167;
            
            if let Some(bncap) = cap.name("button_name") {
                button_name = match bncap {
                    "a" => ButtonName::A,
                    "b" => ButtonName::B,
                    "z" => ButtonName::Z,
                    "l" => ButtonName::L,
                    "r" => ButtonName::R,
                    "start" => ButtonName::Start,
                    "cup" => ButtonName::Cup,
                    "cdown" => ButtonName::Cdown,
                    "cleft" => ButtonName::Cleft,
                    "cright" => ButtonName::Cright,
                    "dup" => ButtonName::Dup,
                    "ddown" => ButtonName::Ddown,
                    "dleft" => ButtonName::Dleft,
                    "dright" => ButtonName::Dright,
                    _ => return None
                };
            } else {
                return None;
            }
            
            if let Some(bdcap) = cap.name("button_duration") {
                match bdcap.parse::<u32>() {
                    Ok(duration_u32) => { button_duration = duration_u32; },
                    _ => return None
                }
                
                if let Some(bdcap_units) = cap.name("button_duration_units") {
                    if bdcap_units == "s" {
                        button_duration *= 1000;
                    }
                } else {
                    return None;
                }
            }

            if button_duration > MAX_BUTTON_COMMAND_DURATION {
                return None;
            }
            
            let time_now = time::get_time();
            let command = TimedInputCommand { start_time: time_now + Duration::milliseconds(cumulative_delay as i64),
                                              duration: time::Duration::milliseconds(button_duration as i64),
                                              command: InputCommand::Button { name: button_name, value: true } };
            res.push(command);
            
            last_command = Some(command.clone());
        } else if let Some(dcap) = cap.name("delay") {
            // delay command - only one argument, the delay to insert
            match dcap {
                "+" => { cumulative_delay += 17; },
                "!" => { cumulative_delay += 217; },
                _ => { return None; }
            }
            last_command = None
        }
        
        last_cap_end = Some(cap_end);
    }
    
    if final_cap_end != msg.len() {
        return None;
    }
    
    if cap_start_zero != true {
        return None;
    }
    
    return Some(res);
}


fn main() {
    // Parse our configuration file
    let (server, pass, nick, channel) = parse_config_file();
    
    // Initialize a democratized virtual N64 controller
    let controller = Controller::new(VJOY_DEVICE_NUMBER).unwrap();
    let dem_controller = DemC::new(controller);
    
    // Start our IRC connection
    let irc_connection = irc::IrcConnection::spawn(server, pass, nick, channel).unwrap();
    
    // Our regex for parsing IRC messages - this is here so that it need not be instantiated every
    // time we handle an IRC message
    let re = Regex::new(r"\s*((?P<joystick>((?P<joystick_strength>[:digit:]+)%\s*)?(?P<joystick_direction>up|down|left|right)(\s*(?P<joystick_duration>[:digit:]+)(?P<joystick_duration_units>s|ms))?)|(?P<button>((?P<button_name>start|cup|cdown|cleft|cright|dup|ddown|dleft|dright|a|b|z|l|r)(\s*(?P<button_duration>[:digit:]+)(?P<button_duration_units>s|ms))?))|(?P<delay>[\+!]))\s*").unwrap();
    
    // Poll the IRC connection and handle its messages forever
    loop {
        match irc_connection.receive_privmsg() {
            Ok(msg) => { 
                //@todo remove this 1 hardcode (which is there to ignore the channel name parameter)
                let (sender, message) = tmi::parse_irc_message_as_tmi(msg).unwrap();
                println!("{}: {}", sender, message);
                if let Some(cmds) = parse_string_as_commands(&message, &re) {
                    for &cmd in cmds.iter() {
                        dem_controller.add_command(cmd);
                    }
                }
            },
            _ => ()
        }
    }
}
