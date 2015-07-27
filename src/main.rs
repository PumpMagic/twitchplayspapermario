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


fn main() {
    // Parse our configuration file
    let (server, pass, nick, channel) = parse_config_file();
    
    // Initialize a democratized virtual N64 controller
    let controller = VirtualN64Controller::new(VJOY_DEVICE_NUMBER).unwrap();
    let mut dem_controller = DemC::new(controller);
    
    // Start our IRC connection
    let irc_connection = libirc::IrcConnection::spawn(server, pass, nick, channel).unwrap();
    
    loop {
        let received_value = irc_connection.receive_privmsg();
        
        //@todo understand as_ref()
        //@todo remove this 1 hardcode (which is there to ignore the channel name parameter)
        match received_value.get(1).unwrap().as_ref() {
            "a" => dem_controller.cast_button_vote(VirtualN64ControllerButton::A),
            "b" => dem_controller.cast_button_vote(VirtualN64ControllerButton::B),
            "z" => dem_controller.cast_button_vote(VirtualN64ControllerButton::Z),
            "l" => dem_controller.cast_button_vote(VirtualN64ControllerButton::L),
            "r" => dem_controller.cast_button_vote(VirtualN64ControllerButton::R),
            "start" => dem_controller.cast_button_vote(VirtualN64ControllerButton::Start),
            "cup" => dem_controller.cast_button_vote(VirtualN64ControllerButton::Cup),
            "cdown" => dem_controller.cast_button_vote(VirtualN64ControllerButton::Cdown),
            "cleft" => dem_controller.cast_button_vote(VirtualN64ControllerButton::Cleft),
            "cright" => dem_controller.cast_button_vote(VirtualN64ControllerButton::Cright),
            "dup" => dem_controller.cast_button_vote(VirtualN64ControllerButton::Dup),
            "ddown" => dem_controller.cast_button_vote(VirtualN64ControllerButton::Ddown),
            "dleft" => dem_controller.cast_button_vote(VirtualN64ControllerButton::Dleft),
            "dright" => dem_controller.cast_button_vote(VirtualN64ControllerButton::Dright),
            "up" => dem_controller.cast_joystick_vote(90, 1.0),
            "down" => dem_controller.cast_joystick_vote(270, 1.0),
            "left" => dem_controller.cast_joystick_vote(180, 1.0),
            "right" => dem_controller.cast_joystick_vote(0, 1.0),
            _ => ()
        }
    }
}
