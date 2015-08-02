#![allow(unused_must_use)]

mod libirc;
mod libvn64c;
mod libdemc;

#[macro_use]
extern crate regex;
extern crate toml;
extern crate time;

use std::fs::File;
use std::io::Read;

use regex::Regex;

use libvn64c::{VirtualN64Controller, VirtualN64ControllerButton};
use libdemc::DemC;


const CONFIG_FILE_PATH: &'static str = "tppm.toml";
const VJOY_DEVICE_NUMBER: u8 = 1;
const IMPLICIT_CHAIN_DELAY: u32 = 1000;

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


//@todo document this and move it somewhere cleaner
enum ControllerCommandType {
    Joystick,
    Button,
    Delay
}
// Handle an incoming IRC message by attempting to parse it as a controller command or series of
// controller commands and queueing these commands for sending to the democratized controller
//@todo just use a custom state machine rather than regex, this has to be insanely slow
//@todo this function is huge
//@todo this will match messages with a single instance of a command no matter where that instance is
// eg. "hah!" matches "a", "the one to the left" matches "left"...
fn handle_irc_message(msg: &String, demc: &mut DemC, re: &Regex) {
    let mut last_cap_end = None;
    let mut cumulative_delay: u32 = 0;
    let mut last_command_type = None;
    for cap in re.captures_iter(msg) {
        // Make sure that all captures are continuous; that is, we're parsing commands and only commands
        // eg. we don't want "hahah" to parse as two "a" commands
        let (cap_start, cap_end) = cap.pos(0).unwrap(); //@todo don't rely on capture position 0 existing
        if let Some(last_cap_end) = last_cap_end {
            if last_cap_end != cap_start {
                return;
            }
        }
        
        // Our regex should match on exactly one of three groups: "joystick", "button", or "delay"
        if let Some(jcap) = cap.name("joystick") {
            match last_command_type {
                Some(ctype) => match ctype {
                    ControllerCommandType::Joystick => { cumulative_delay += IMPLICIT_CHAIN_DELAY; },
                    ControllerCommandType::Button => { cumulative_delay += IMPLICIT_CHAIN_DELAY; },
                    ControllerCommandType::Delay => ()
                },
                _ => ()
            }
            // joystick command - should have one, two, or four groups:
            // "joystick_strength" (optional)
            // "joystick_direction" (mandatory)
            // "joystick_duration" (optional),
            // "joystick_duration_units" (optional; must be present if joystick_duration is)
            println!("joystick command: {:?}", jcap);
            
            let mut joystick_strength: f32 = 1.0;
            let mut joystick_direction: u16 = 0;
            let mut joystick_duration: u32 = 1000;
            if let Some(jscap) = cap.name("joystick_strength") {
                let strength_u8: u8 = jscap.parse().unwrap();
                joystick_strength = strength_u8 as f32 / 100.0;
            }
            if let Some(jdcap) = cap.name("joystick_direction") {
                match jdcap {
                    "up" => { joystick_direction = 90; },
                    "down" => { joystick_direction = 270; },
                    "left" => { joystick_direction = 180; },
                    "right" => { joystick_direction = 0; },
                    _ => ()
                }
            }
            if let Some(jdcap) = cap.name("joystick_duration") {
                joystick_duration = jdcap.parse().unwrap();
                if let Some(jdcap_units) = cap.name("joystick_duration_units") {
                    if jdcap_units == "s" {
                        joystick_duration *= 1000;
                    }
                }
            }
            
            // treat joystick commands with strength <0%, >100% or duration >5s as invalid
            if joystick_strength > 1.0 || joystick_strength < 0.0 || joystick_duration > 5000 {
                return;
            }
            
            demc.cast_joystick_vote(joystick_direction, joystick_strength, joystick_duration, cumulative_delay);
            
            last_command_type = Some(ControllerCommandType::Joystick);
        } else if let Some(bcap) = cap.name("button") {
            match last_command_type {
                Some(ctype) => match ctype {
                    ControllerCommandType::Joystick => { cumulative_delay += IMPLICIT_CHAIN_DELAY; },
                    ControllerCommandType::Button => { cumulative_delay += IMPLICIT_CHAIN_DELAY; },
                    ControllerCommandType::Delay => ()
                },
                _ => ()
            }
            // button command - only one argument, the button to press
            println!("button command: {:?}", bcap);
            match bcap {
                "a" => demc.cast_button_vote(VirtualN64ControllerButton::A, cumulative_delay),
                "b" => demc.cast_button_vote(VirtualN64ControllerButton::B, cumulative_delay),
                "z" => demc.cast_button_vote(VirtualN64ControllerButton::Z, cumulative_delay),
                "l" => demc.cast_button_vote(VirtualN64ControllerButton::L, cumulative_delay),
                "r" => demc.cast_button_vote(VirtualN64ControllerButton::R, cumulative_delay),
                "start" => demc.cast_button_vote(VirtualN64ControllerButton::Start, cumulative_delay),
                "cup" => demc.cast_button_vote(VirtualN64ControllerButton::Cup, cumulative_delay),
                "cdown" => demc.cast_button_vote(VirtualN64ControllerButton::Cdown, cumulative_delay),
                "cleft" => demc.cast_button_vote(VirtualN64ControllerButton::Cleft, cumulative_delay),
                "cright" => demc.cast_button_vote(VirtualN64ControllerButton::Cright, cumulative_delay),
                "dup" => demc.cast_button_vote(VirtualN64ControllerButton::Dup, cumulative_delay),
                "ddown" => demc.cast_button_vote(VirtualN64ControllerButton::Ddown, cumulative_delay),
                "dleft" => demc.cast_button_vote(VirtualN64ControllerButton::Dleft, cumulative_delay),
                "dright" => demc.cast_button_vote(VirtualN64ControllerButton::Dright, cumulative_delay),
                _ => return
            }
            
            last_command_type = Some(ControllerCommandType::Button);
        } else if let Some(dcap) = cap.name("delay") {
            // delay command - only one argument, the delay to insert
            println!("delay command: {:?}", dcap);
            cumulative_delay += 17;
            last_command_type = Some(ControllerCommandType::Delay);
        }
        
        last_cap_end = Some(cap_end);
    }
}


fn main() {
    // Parse our configuration file
    let (server, pass, nick, channel) = parse_config_file();
    
    // Initialize a democratized virtual N64 controller
    let controller = VirtualN64Controller::new(VJOY_DEVICE_NUMBER).unwrap();
    let mut dem_controller = DemC::new(controller);
    
    // Start our IRC connection
    let irc_connection = libirc::IrcConnection::spawn(server, pass, nick, channel).unwrap();
    
    // Our regex for parsing IRC messages - this is here so that it need not be instantiated every
    // time we handle an IRC message
    let re = Regex::new(r"\s*((?P<joystick>((?P<joystick_strength>[:digit:]+)%\s*)?(?P<joystick_direction>up|down|left|right)(\s*(?P<joystick_duration>[:digit:]+)(?P<joystick_duration_units>s|ms))?)|(?P<button>start|cup|cdown|cleft|cright|dup|ddown|dleft|dright|a|b|z|l|r)|(?P<delay>\+))\s?").unwrap();
    
    // Poll the IRC connection and handle its messages forever
    loop {
        match irc_connection.receive_privmsg() {
            Ok(msg_vec) => { 
                //@todo remove this 1 hardcode (which is there to ignore the channel name parameter)
                match msg_vec.get(1) {
                    Some(string) => { handle_irc_message(string, &mut dem_controller, &re); },
                    _ => ()
                }
            },
            _ => ()
        }
    }
}
