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
    Privmsg,
    Ping,
    Unknown
}

impl Command {
    fn as_str(&self) -> &'static str {
        match *self {
            Command::ReplyWelcome => "RPL_WELCOME",
            Command::Privmsg => "PRIVMSG",
            Command::Ping => "PING",
            _ => ""
        }
    }
    
    fn from_string(command: &str) -> Command {
        match command {
            "001"|"RPL_WELCOME"     =>  Command::ReplyWelcome,
            "PRIVMSG"               =>  Command::Privmsg,
            "PING"                  =>  Command::Ping,
            _                       =>  Command::Unknown
        }
    }
    
    //@todo add from_str.. possible to combine with as_str() with some sort of bidirectional map?
}

// A representation of an IRC message
#[derive(Debug)]
struct IrcMessage {
    prefix: Option<String>,
    command: Option<Command>,
    params: Option<Vec<String>>
}

impl IrcMessage {
    fn from_string(msg: String) -> IrcMessage {
        // Parse a UTF-8 string into our own IRC message structure
        
        // Dissect the message to identify its prefix (if present), its command (if present), and its
        // arguments (if present)
        //@todo probably a bad idea to have written this regex, google "irc regex" and replace this
        let re = Regex::new(r"(^:(?P<prefix>([:alnum:]|[:punct:])+) )?((?P<command>([:alnum:])+)? ?)(?P<params>.*?)\r").unwrap();
        let cap = re.captures(&msg).unwrap();

        let prefix = cap.name("prefix");
        let command = cap.name("command");
        let params = cap.name("params");

        // Populate an IrcMessage struct with the fields, splitting up the arguments (if present) into a
        // list of Strings
        IrcMessage {
            prefix: match prefix {
                Some(prefix) => Some(FromStr::from_str(prefix).unwrap()),
                None => None
            },

            command: match command {
                Some(command) => Some(Command::from_string(command)),
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
        }
    }
    
    fn into_string(self) -> String {
        let mut result = String::new();
        
        if self.prefix.is_some() {
            result.push_str(&self.prefix.unwrap());
        }
        
        if self.command.is_some() {
            result.push_str(&self.command.unwrap().as_str());
        }
        
        if self.params.is_some() {
            for param in self.params.unwrap() {
                result.push_str(&param);
            }
        }
        
        result
    }
}


pub struct IrcConnection {
    tcp_stream: TcpStream,
    
    server: String,
    pass: String,
    nick: String,
    channel: String,
}

impl IrcConnection {
    pub fn spawn(server: String, pass: String, nick: String, channel: String) -> Result<(thread::JoinHandle<()>, mpsc::Receiver<Vec<String>>), ()> {
        let stream = match TcpStream::connect(&server[..]) {
            Ok(stream) => stream,
            Err(_) => return Err(())
        };
        
        let irc_connection = IrcConnection { tcp_stream: stream, server: server, pass: pass, nick: nick, channel: channel };
        let (tx, rx) = mpsc::channel();
        
        // Spawn a thread that manages an IRC connection and passes through chat messages (privmsgs)
        let join_handle = thread::spawn(move|| {
            // Send the server our credentials
            //@todo implement send failure recovery
            match irc_connection.send_credentials() {
                Ok(_) => (),
                Err(_) => panic!("Unable to send credentials!")
            };
            
            let mut sent_join = false;
            
            loop {
                //@todo match rather than if/else
                let m = irc_connection.get_message();
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
                        Command::ReplyWelcome => {
                            // Ready to join!
                            //@TODO there must be a better sign that the server likes our pipe now?
                            //@todo join failure recovery
                            if sent_join == false {
                                match irc_connection.send_join() {
                                    Ok(_) => (),
                                    Err(_) => panic!("Unable to join target channel!")
                                };
                                sent_join = true;
                            }
                        },
                        Command::Ping => {
                            match irc_connection.send_pong() {
                                Ok(_) => (),
                                Err(_) => panic!("Unable to send pong!")
                            };
                        },
                        Command::Privmsg => {
                            for x in params.iter() {
                                println!("\t {}", x);
                            }
                            
                            if params.len() > 0 {
                                match tx.send(params) {
                                    Ok(_) => (),
                                    Err(err) => println!("Error sending received IRC message to user\
                                                          app: {}", err)
                                };
                            }
                        }
                        _ => ()
                    }
                }
            }
        });
        
        Ok( (join_handle, rx) )
    }

    // Get a message from an IRC channel.
    fn get_message(&self) -> IrcMessage {
        // Receive a message from the server as raw bytes.
        // We'll convert it to a String once we've received the whole thing, to simplify parsing
        let mut response: Vec<u8> = Vec::new();

        // Read from the TCP stream until we get CRLF (which signals the termination of an IRC message)
        // or the socket read fails
        loop {
            let mut read_byte: Vec<u8> = Vec::with_capacity(1);

            let read_result = (&(self.tcp_stream)).take(1).read_to_end(&mut read_byte);
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
        let msg_str = String::from_utf8(response).unwrap();

        IrcMessage::from_string(msg_str)
    }

    // Consider making a Message serializer...
    fn send_message(&self, command: &String, params: Option<&String>) -> Result<(), ()> {
        let mut message_string = String::new();
        message_string.push_str(command);
        message_string.push_str(" ");
        if params.is_some() {
            message_string.push_str(params.unwrap());
        }
        message_string.push_str("\r\n");
        
        match (&(self.tcp_stream)).write_all(message_string.as_bytes()) {
            Ok(_) => Ok(()),
            Err(_) => Err(())
        }
    }

    fn send_credentials(&self) -> Result<(), ()> {
        let pass_string = FromStr::from_str("PASS").unwrap();
        let nick_string = FromStr::from_str("NICK").unwrap();
        
        try!(self.send_message(&pass_string, Some(&self.pass)));
        try!(self.send_message(&nick_string, Some(&self.nick)));
        
        Ok(())
    }

    fn send_join(&self) -> Result<(), ()> {
        let join_string = FromStr::from_str("JOIN").unwrap();
        
        try!(self.send_message(&join_string, Some(&(self.channel))));
        
        Ok(())
    }

    fn send_pong(&self) -> Result<(), ()> {
        let pong_string = FromStr::from_str("PONG").unwrap();
        
        try!(self.send_message(&pong_string, None));
        
        Ok(())
    }
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