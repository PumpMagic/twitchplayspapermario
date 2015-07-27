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

use libvn64c::{VirtualN64Controller, VirtualN64ControllerButton};
use libdemc::DemC;


const CONFIG_FILE_PATH: &'static str = "tppm.toml";
const VJOY_DEVICE_NUMBER: u8 = 1;


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


fn handle_irc_message(msg_vec: Vec<String>, demc: &mut DemC) {
    //@todo understand as_ref()
    //@todo remove this 1 hardcode (which is there to ignore the channel name parameter)
    match msg_vec.get(1) {
        Some(param) => match param.as_ref() {
            "a" => demc.cast_button_vote(VirtualN64ControllerButton::A),
            "b" => demc.cast_button_vote(VirtualN64ControllerButton::B),
            "z" => demc.cast_button_vote(VirtualN64ControllerButton::Z),
            "l" => demc.cast_button_vote(VirtualN64ControllerButton::L),
            "r" => demc.cast_button_vote(VirtualN64ControllerButton::R),
            "start" => demc.cast_button_vote(VirtualN64ControllerButton::Start),
            "cup" => demc.cast_button_vote(VirtualN64ControllerButton::Cup),
            "cdown" => demc.cast_button_vote(VirtualN64ControllerButton::Cdown),
            "cleft" => demc.cast_button_vote(VirtualN64ControllerButton::Cleft),
            "cright" => demc.cast_button_vote(VirtualN64ControllerButton::Cright),
            "dup" => demc.cast_button_vote(VirtualN64ControllerButton::Dup),
            "ddown" => demc.cast_button_vote(VirtualN64ControllerButton::Ddown),
            "dleft" => demc.cast_button_vote(VirtualN64ControllerButton::Dleft),
            "dright" => demc.cast_button_vote(VirtualN64ControllerButton::Dright),
            "up" => demc.cast_joystick_vote(90, 1.0),
            "down" => demc.cast_joystick_vote(270, 1.0),
            "left" => demc.cast_joystick_vote(180, 1.0),
            "right" => demc.cast_joystick_vote(0, 1.0),
            _ => ()
        },
        _ => { println!("Received unrecognized message: {:?}", msg_vec); }
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
    
    // Poll the IRC connection and handle its messages forever
    loop {
        match irc_connection.receive_privmsg() {
            Ok(msg_vec) => { handle_irc_message(msg_vec, &mut dem_controller); },
            _ => ()
        }
    }
}
