#![allow(dead_code)]

use std::net::TcpStream;
use std::io::Read;
use std::io::Write;
use std::str::FromStr;
use std::thread;
use std::sync::mpsc;

use regex::Regex;

// A small subset of IRC command types
#[derive(Debug)]
enum Command {
    ReplyWelcome,
    ReplyYourhost,
    ReplyCreated,
    ReplyMyinfo,
    ReplyBounce,
    ReplyYourid,
    ReplyMotdstart,
    ReplyMotd,
    ReplyEndofmotd,
    ReplyLuserclient,
    ReplyLuserchannels,
    ReplyLuserme,
    ReplyLocalusers,
    ReplyGlobalusers,
    Join,
    ReplyNamereply,
    ReplyEndofnames,
    Privmsg,
    Notice,
    Ping,
    Error,
    Mode,
    Unknown
}

// A representation of an IRC message
#[derive(Debug)]
struct Message {
    prefix: Option<String>,
    command: Option<Command>,
    params: Option<Vec<String>>
}

// Parse a UTF-8 string into our own IRC message structure
fn parse_message(msg: String) -> Message {
    // Dissect the message to identify its prefix (if present), its command (if present), and its
    // arguments (if present)
    //@todo probably a bad idea to have written this regex, google "irc regex" and replace this
    let re = Regex::new(r"(^:(?P<prefix>([:alnum:]|[:punct:])+) )?((?P<command>([:alnum:])+)? ?)(?P<params>.*?)\r").unwrap();
    let cap = re.captures(&msg).unwrap();

    let prefix = cap.name("prefix");
    let command = cap.name("command");
    let params = cap.name("params");

    // Populate a Message struct with the fields, splitting up the arguments (if present) into a
    // list of Strings
    let message_struct = Message {
        prefix: match prefix {
            Some(prefix) => Some(FromStr::from_str(prefix).unwrap()),
            None => None
        },

        command: match command {
            Some(command) => match command {
                "001"|"RPL_WELCOME"     =>  Some(Command::ReplyWelcome),
                "002"|"RPL_YOURHOST"    =>  Some(Command::ReplyYourhost),
                "003"|"RPL_CREATED"     =>  Some(Command::ReplyCreated),
                "004"|"RPL_MYINFO"      =>  Some(Command::ReplyMyinfo),
                "005"|"RPL_BOUNCE"      =>  Some(Command::ReplyBounce),
                "042"|"RPL_YOURID"      =>  Some(Command::ReplyYourid),
                "375"|"RPL_MOTDSTART"   =>  Some(Command::ReplyMotdstart),
                "372"|"RPL_MOTD"        =>  Some(Command::ReplyMotd),
                "376"|"RPL_ENDOFMOTD"   =>  Some(Command::ReplyEndofmotd),
                "251"|"RPL_LUSERCLIENT" =>  Some(Command::ReplyLuserclient),
                "254"|"RPL_LUSERCHANNELS"   =>  Some(Command::ReplyLuserchannels),
                "255"|"RPL_LUSERME"     =>  Some(Command::ReplyLuserme),
                "265"|"RPL_LOCALUSERS"  =>  Some(Command::ReplyLocalusers),
                "266"|"RPL_GLOBALUSERS" =>  Some(Command::ReplyGlobalusers),
                "JOIN"                  =>  Some(Command::Join),
                "353"|"RPL_NAMREPLY"    =>  Some(Command::ReplyNamereply),
                "366"|"RPL_ENDOFNAMES"  =>  Some(Command::ReplyEndofnames),
                "PRIVMSG"               =>  Some(Command::Privmsg),
                "NOTICE"                =>  Some(Command::Notice),
                "PING"                  =>  Some(Command::Ping),
                "ERROR"                 =>  Some(Command::Error),
                "MODE"                  =>  Some(Command::Mode),
                _                       =>  {
                    println!("Received unknown message type. Raw message: {:?}", msg);
                    Some(Command::Unknown)
                }
            },
            _                           =>  None
        },

        params: match params {
            Some(params) => {
                // Tokenize the parameters.
                // Tokenizing them is nontrivial, because of the TRAILING parameter pattern.
                // So we use regex.
                //@TODO this regex fails if any middle parameter has a colon in it
                //@TODO or if the trailing parameter has a \r without a following \n
                //@todo probably a bad idea to have written this regex, google "irc regex" and replace this
                let re = Regex::new(r"^(?P<middles>[^:]+)?(:(?P<trailing>[^\r\n]+))?").unwrap();
                let cap = re.captures(params).unwrap();

                let middles = cap.name("middles");
                let trailing = cap.name("trailing");

                let mut paramvec = Vec::new();

                match middles {
                    Some(middles)   => {
                        for middle in middles.trim().split(" ") {
                            paramvec.push(FromStr::from_str(middle).unwrap());
                        }
                    },
                    _               => ()
                }

                match trailing {
                    Some(trailing)  => { paramvec.push(FromStr::from_str(trailing).unwrap()) },
                    _               => ()
                };

                Some(paramvec)
            },
            None => None
        }
    };

    message_struct
}

fn last_two_are_crlf(myvec: &Vec<u8>) -> bool {
    let myvec_len = myvec.len();

    if myvec_len < 2 {
        return false;
    }

    match (myvec[myvec_len-2], myvec[myvec_len-1]) {
        (b'\r', b'\n')  => true,
        _               => false
    }
}

// Get a message from an IRC channel.
fn get_message(stream: &mut TcpStream) -> Message {
    // Receive a message from the server as raw bytes.
    // We'll convert it to a String once we've received the whole thing, to simplify parsing
    let mut response: Vec<u8> = Vec::new();

    // Read from the TCP stream until we get CRLF (which signals the termination of an IRC message)
    // or the socket read fails
    loop {
        let mut read_byte: Vec<u8> = Vec::with_capacity(1);

        let read_result = stream.take(1).read_to_end(&mut read_byte);
        match read_result {
            Result::Ok(_) => {
                response.push(*(read_byte.get(0).unwrap())); //@todo handle None
                if last_two_are_crlf(&response) {
                    break;
                }
            },
            Result::Err(error)      => {
                println!("Stream read error: {:?}", error);
                break;
            }
        }
    }

    // Convert our raw byte vector into a String for easier, native processing
    //@TODO Better handle invalid messages
    let msg_str = match String::from_utf8(response) {
        Ok(val) => val,
        Err(err) => panic!("{}", err)
    };

    parse_message(msg_str)
}

// Consider making a Message serializer...
fn send_message(stream: &mut TcpStream, command: &String, params: Option<&String>) -> Result<(), ()> {
    let mut message_string = String::new();
    message_string.push_str(command);
    message_string.push_str(" ");
    if params.is_some() {
        message_string.push_str(params.unwrap());
    }
    message_string.push_str("\r\n");
    
    match stream.write_all(message_string.as_bytes()) {
        Ok(_) => Ok(()),
        Err(err) => Err(())
    }
}

fn send_credentials(stream: &mut TcpStream, pass: &String, nick: &String) -> Result<(), ()> {
    let pass_string = FromStr::from_str("PASS").unwrap();
    let nick_string = FromStr::from_str("NICK").unwrap();
    
    try!(send_message(stream, &pass_string, Some(pass)));
    try!(send_message(stream, &nick_string, Some(nick)));
    
    Ok(())
}

fn send_join(stream: &mut TcpStream, channel: &String) -> Result<(), ()> {
    let join_string = FromStr::from_str("JOIN").unwrap();
    
    try!(send_message(stream, &join_string, Some(channel)));
    
    Ok(())
}

fn send_pong(stream: &mut TcpStream) -> Result<(), ()> {
    let pong_string = FromStr::from_str("PONG").unwrap();
    
    try!(send_message(stream, &pong_string, None));
    
    Ok(())
}

//@todo document
pub fn start(server: String, pass: String, nick: String, channel: String) -> Result<mpsc::Receiver<Vec<String>>, &'static str> {
    let (tx, rx) = mpsc::channel();
    
    thread::spawn(move|| {
        // Connect to the IRC server
        let mut stream = TcpStream::connect(&server[..]).unwrap();

        // Send the server our credentials
        //@todo implement send failure recovery
        match send_credentials(&mut stream, &pass, &nick) {
            Ok(_) => (),
            Err(_) => panic!("Unable to send credentials!")
        };
        
        let mut sent_join = false;
        
        loop {
            let m = get_message(&mut stream);
            if m.command.is_none() {
                println!("Ignoring message with unidentified command...");
            } else {
                let null_prefix = String::new();
                let mut null_params = Vec::new();
                null_params.push(String::new());

                let prefix = m.prefix.unwrap_or(null_prefix);
                let command = m.command.unwrap();
                let params = m.params.unwrap_or(null_params);

                println!("{:?} {:?}: {:?}", prefix, command, params);

                match command {
                    // as a bot, all we really care about is:
                    // does the server consider our channel valid yet? if so, JOIN target channel
                    // has the server acknowledged our join? (are we in our target channel?)
                    // did another client send a message? if so, log it and act on it
                    // did the server ping us? if so, pong it
                    //Command::Notice =>  println!("NOTICE {:?} {:?}", prefix, params),
                    //Command::Error =>   println!("Got an error!"),
                    Command::ReplyWelcome => {
                        // Ready to join!
                        //@TODO there must be a better sign that the server likes our pipe now?
                        //@todo join failure recovery
                        if sent_join == false {
                            match send_join(&mut stream, &channel) {
                                Ok(_) => (),
                                Err(_) => panic!("Unable to join target channel!")
                            };
                            sent_join = true;
                        }
                    },
                    Command::Ping => {
                        match send_pong(&mut stream) {
                            Ok(_) => (),
                            Err(_) => panic!("Unable to send pong!")
                        };
                    },
                    //Command::Unknown => println!("Got unknown!"),
                    Command::Privmsg => {
                        for x in params.iter() {
                            println!("\t {}", x);
                        }
                        
                        if params.len() > 0 {
                            tx.send(params);
                        }
                    }
                    _ => ()
                }
            }
        }
    });
    
    Ok(rx)
}