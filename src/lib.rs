use std::net::TcpStream;
use std::io::Read;
use std::io::Write;
use std::thread;
use std::str::FromStr;

mod libirc;
mod libvn64c;

#[macro_use]
extern crate log;
extern crate regex;

//static TCP_SOCKET: &'static str = "127.0.0.1:6697";
static TCP_SOCKET: &'static str = "irc.twitch.tv:6667";

//static MESSAGE1: &'static str = "NICK rusty\r\n";
//static MESSAGE2: &'static str = "USER rusty dummyhost dummyserver :Rusty Bucket\r\n";
//static MESSAGE3: &'static str = "JOIN #test\r\n";
static MESSAGE1: &'static str = "PASS oauth:qh7rpnj5z7d6i3crs64jtwojm8a0wq\r\n";
static MESSAGE2: &'static str = "NICK pumpmagic\r\n";
static MESSAGE3: &'static str = "JOIN #twitchplayspapermario\r\n";

#[test]
fn it_works() {
    /*
    let myConf: LibircConfig = LibircConfig {
        server: "irc.twitch.tv",
        port: 6667,
        nick: "pumpmagic",
        pass: "oauth:qh7rpnj5z7d6i3crs64jtwojm8a0wq",
        channel: "#twitchplayspokemon"
    };
    */

    // Connect to the IRC server
    let mut stream = TcpStream::connect(TCP_SOCKET).unwrap();

    // Send the server our nick and user
    let _ = stream.write_all(MESSAGE1.as_bytes());
    let _ = stream.write_all(MESSAGE2.as_bytes());

    let mut sent_join = false;

    
    // vn64c stuff
    let mut controller = libvn64c::VirtualN64Controller { ..Default::default() };
    if !libvn64c::init(&mut controller) {
        panic!("Unable to initialize controller!");
    }
    
    loop {
        let m = libirc::get_message_from_stream(&mut stream);
        if m.command.is_none() {
            println!("Ignoring message with unidentified command...");
        } else {
            let null_prefix = String::new();
            let mut null_params = Vec::new();
            null_params.push(String::new());

            let prefix = m.prefix.unwrap_or(null_prefix);
            let command = m.command.unwrap();
            let params = m.params.unwrap_or(null_params);

            println!("{:?}: {:?}", command, params);

            match command {
                // as a bot, all we really care about is:
                // does the server consider our channel valid yet? if so, JOIN target channel
                // has the server acknowledged our join? (are we in our target channel?)
                // did another client send a message? if so, log it and act on it
                // did the server ping us? if so, pong it
                //Command::Notice =>  println!("NOTICE {:?} {:?}", prefix, params),
                //Command::Error =>   println!("Got an error!"),
                libirc::Command::ReplyWelcome => {
                    // Ready to join!
                    //@TODO there must be a better sign that the server likes our pipe now?
                    if sent_join == false {
                        println!("Server welcomed us. Joining target channel");
                        stream.write_all(MESSAGE3.as_bytes());
                        sent_join = true;
                    }
                },
                libirc::Command::Ping => {
                    stream.write_all("PONG\r\n".as_bytes());
                },
                //Command::Unknown => println!("Got unknown!"),
                libirc::Command::Privmsg => {
                    println!("Got privmsg:");
                    for x in params.iter() {
                        println!("\t {}", x);
                    }
                    
                    //match params.get(1).unwrap() {
                    let blah = params.get(1).unwrap();
                    if blah == "jump" {
                        libvn64c::set_button(&controller, libvn64c::BUTTON_A, false);
                        thread::sleep_ms(1000);
                        libvn64c::set_button(&controller, libvn64c::BUTTON_A, true);
                        thread::sleep_ms(1000);
                    } else if blah == "up" {
                        libvn64c::set_joystick(&mut controller, 90, 0.5);
                    } else if blah == "down" {
                        libvn64c::set_joystick(&mut controller, 270, 0.5);
                    } else if blah == "left" {
                        libvn64c::set_joystick(&mut controller, 180, 0.5);
                    } else if blah == "right" {
                        libvn64c::set_joystick(&mut controller, 0, 0.5);
                    }
                }
                _ => ()
            }
        }
    }
}
