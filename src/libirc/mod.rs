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
    Pass,
    Nick,
    Join,
    Privmsg,
    Ping,
    Pong,
    Unknown
}

impl Command {
    fn as_str(&self) -> &'static str {
        match *self {
            Command::ReplyWelcome => "RPL_WELCOME",
            Command::Pass => "PASS",
            Command::Nick => "NICK",
            Command::Join => "JOIN",
            Command::Privmsg => "PRIVMSG",
            Command::Ping => "PING",
            Command::Pong => "PONG",
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
}

// A representation of an IRC message
#[derive(Debug)]
struct IrcMessage {
    prefix: Option<String>,
    command: Command,
    params: Option<Vec<String>>
}

impl IrcMessage {
    fn from_string(msg: String) -> Option<IrcMessage> {
        // Parse a UTF-8 string into our own IRC message structure
        
        // Dissect the message to identify its prefix (if present), its command (if present), and its
        // arguments (if present)
        //@todo probably a bad idea to have written this regex, google "irc regex" and replace this
        let re = Regex::new(r"(^:(?P<prefix>([:alnum:]|[:punct:])+) )?((?P<command>([:alnum:])+)? ?)(?P<params>.*?)\r").unwrap();
        let cap = re.captures(&msg).unwrap();

        let prefix = cap.name("prefix");
        let command = cap.name("command");
        let params = cap.name("params");

        // Populate an IrcMessage struct with the fields, splitting up the parameters (if present)
        // into a list of Strings
        Some(IrcMessage {
            prefix: match prefix {
                Some(prefix) => Some(FromStr::from_str(prefix).unwrap()),
                None => None
            },

            command: match command {
                Some(command) => Command::from_string(command),
                _ =>  return None // our regex couldn't parse a command; the command is mandatory
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
        })
    }
    
    // Assumes that the last parameter is the trailing parameter
    fn into_string(self) -> String {
        let mut result = String::new();
        
        if self.prefix.is_some() {
            result.push_str(":");
            result.push_str(&self.prefix.unwrap());
            result.push_str(" ");
        }
        
        result.push_str(&self.command.as_str());
        result.push_str(" ");
        
        if self.params.is_some() {
            // C-style... is there a better way to identify the last element?
            let params = self.params.unwrap();
            let iter = params.iter();
            let num_params = iter.clone().count();
            let mut param_on = 0;
            
            for param in iter {
                param_on = param_on + 1;
                if param_on == num_params {
                    result.push_str(":");
                }
                result.push_str(&param);
                if param_on != num_params {
                    result.push_str(" ");
                }
            }
        }
        
        result.push_str("\r\n");
        
        result
    }
}


pub struct IrcConnection {
    join_handle: thread::JoinHandle<()>,
    rx_privmsg: mpsc::Receiver<Vec<String>>,
    tx_kill: mpsc::Sender<()>
}

impl IrcConnection {
    pub fn spawn(server: String, pass: String, nick: String, channel: String) -> Result<IrcConnection, ()> {
        // Establish a TCP stream with our target server
        let stream = match TcpStream::connect(&server[..]) {
            Ok(stream) => stream,
            Err(_) => return Err(())
        };
        
        // Create two application-local channels: one for passing received privmsgs to our user app,
        // and one for listening from our user app for a kill command
        let (tx_privmsg, rx_privmsg) = mpsc::channel();
        let (tx_kill, rx_kill) = mpsc::channel();
        
        // We're going to spawn a thread that services an IRC connection - we contain all of the
        // information that thread will need in an "internal" IRC connection struct and give it
        // ownership of that struct
        let irc_connection_internal = IrcConnectionInternal { tcp_stream: stream, server: server, pass: pass, nick: nick, channel: channel };
        
        // Spawn an IRC connection servicing thread. This thread maintains an IRC connection and
        // passes chat messages (privmsgs) through one of its channels
        let join_handle = thread::spawn(move|| {
            // Send the server our credentials
            //@todo implement log in failure recovery
            match irc_connection_internal.send_credentials() {
                Ok(_) => (),
                Err(_) => panic!("Unable to send credentials!")
            }
            
            match irc_connection_internal.send_join() {
                Ok(_) => (),
                Err(_) => panic!("Unable to join target channel!")
            }
            
            loop {
                // Check for kill signal; kill this thread if received
                match rx_kill.try_recv() {
                    Ok(()) => return,
                    Err(_) => ()
                }
                
                let m = irc_connection_internal.get_message();
                println!("\t\t::prefix:: {:?} ::command:: {:?}: ::params::{:?}\n", m.prefix, m.command, m.params);
                match m.command {
                    // as a bot, all we really care about is:
                    // did the server ping us? if so, pong it
                    // did another client send a message? if so, pass it to our user
                    Command::Ping => {
                        match irc_connection_internal.send_pong() {
                            Ok(_) => (),
                            Err(_) => panic!("Unable to send pong!")
                        }
                    },
                    Command::Privmsg => {
                        match m.params {
                            Some(params) => {
                                for x in params.iter() {
                                    println!("\t {}", x);
                                }
                                
                                if params.len() > 0 {
                                    match tx_privmsg.send(params) {
                                        Ok(_) => (),
                                        Err(err) => println!("Error sending received IRC message to user\
                                                              app: {}", err)
                                    };
                                }
                            },
                            _ => ()
                        }
                    }
                    _ => ()
                }
            }
        });
        
        Ok( IrcConnection { join_handle: join_handle, rx_privmsg: rx_privmsg, tx_kill: tx_kill } )
    }
    
    pub fn join(self) {
        self.join_handle.join();
    }
    
    pub fn receive_privmsg(&self) -> Vec<String> {
        self.rx_privmsg.recv().unwrap()
    }
    
    pub fn kill(&self) {
        self.tx_kill.send(());
    }
}

struct IrcConnectionInternal {
    tcp_stream: TcpStream,
    
    server: String,
    pass: String,
    nick: String,
    channel: String,
}

impl IrcConnectionInternal {
    // Get a message from an IRC channel.
    //@todo make this nonblocking - any way to do this without function-local static?
    fn get_message(&self) -> IrcMessage {
        loop {
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
            
            println!("IN  <--\t{}", msg_str);

            match IrcMessage::from_string(msg_str) {
                Some(msg) => return msg,
                _ => () // the string we received couldn't be parsed as a valid IRC message
            }
        }
    }

    // Consider making a Message serializer...
    fn send_message(&self, message: IrcMessage) -> Result<(), ()> {
        let message_string = message.into_string();
        
        println!("OUT -->\t{}", message_string);
        
        match (&(self.tcp_stream)).write_all(message_string.as_bytes()) {
            Ok(_) => Ok(()),
            Err(_) => Err(())
        }
    }

    fn send_credentials(&self) -> Result<(), ()> {
        let pass_message = IrcMessage { prefix: None, command: Command::Pass, params: Some(vec![self.pass.clone()]) };
        let nick_message = IrcMessage { prefix: None, command: Command::Nick, params: Some(vec![self.nick.clone()]) };
        
        try!(self.send_message(pass_message));
        try!(self.send_message(nick_message));
        
        Ok(())
    }

    fn send_join(&self) -> Result<(), ()> {
        let join_message = IrcMessage { prefix: None, command: Command::Join, params: Some(vec![self.channel.clone()]) };
        
        try!(self.send_message(join_message));
        
        Ok(())
    }

    fn send_pong(&self) -> Result<(), ()> {
        let pong_message = IrcMessage { prefix: None, command: Command::Pong, params: None };
        
        try!(self.send_message(pong_message));
        
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