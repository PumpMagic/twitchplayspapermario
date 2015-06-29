mod libirc;
mod libvn64c;

#[macro_use]
extern crate log;
extern crate regex;
extern crate toml;

use std::thread;
use std::fs::File;
use std::io::Read;


#[test]
fn it_works() {
    // Initialize a virtual N64 controller
    let mut controller = match libvn64c::init() {
        Ok(controller) => controller,
        Err(msg) => panic!("Unable to initialize controller: {}!", msg),
    };

    // Grab our configuration from our configuration file
    // Open our config file and capture it into a string
    let mut config_file = File::open("tppm.toml").unwrap();
    let mut config_string = String::new();
    config_file.read_to_string(&mut config_string);
    
    let toml_tree: toml::Value = config_string.parse().unwrap();
    
    let server = String::from(toml_tree.lookup("irc.server").unwrap().as_str().unwrap());
    let pass = String::from(toml_tree.lookup("irc.pass").unwrap().as_str().unwrap());
    let nick = String::from(toml_tree.lookup("irc.nick").unwrap().as_str().unwrap());
    let channel = String::from(toml_tree.lookup("irc.channel").unwrap().as_str().unwrap());
    
    let rx = libirc::start(server, pass, nick, channel).unwrap();
    
    loop {
        let received_value = rx.recv().unwrap();
        
        match received_value.get(1).unwrap().as_ref() {
            "a" => {
                libvn64c::set_button(&controller, libvn64c::BUTTON_A, false);
                thread::sleep_ms(200);
                libvn64c::set_button(&controller, libvn64c::BUTTON_A, true);
                thread::sleep_ms(200);
            },
            "up" => { libvn64c::set_joystick(&mut controller, 90, 0.5); },
            "down" => { libvn64c::set_joystick(&mut controller, 270, 0.5); },
            "left" => { libvn64c::set_joystick(&mut controller, 180, 0.5); },
            "right" => { libvn64c::set_joystick(&mut controller, 0, 0.5); },
            _ => ()
        }
    }
}
