#![allow(dead_code)]

use std::net::TcpStream;
use std::io::Read;
use std::io::Write;
use std::str::FromStr;
use std::thread;
use std::sync::mpsc;

use std::ops;

use regex::Regex;


// IRC Prefix type
// This field is optional in IRC messages, and contains information about the sender, source username and host
#[derive(Debug)]
pub struct Prefix {
    pub servername_nick: String,
    pub user: Option<String>,
    pub host: Option<String>
}

// Create a Prefix from a str
// Err(1): IRC prefix regex didn't match
// Err(2): Internal error
impl FromStr for Prefix {
    type Err = u8;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let re = Regex::new(r"(?P<servername_nick>[^!]+)(!(?P<user>[^@]*))?(@(?P<host>.*))?").unwrap();
        
        let caps = match re.captures(s) {
            Some(cap) => cap,
            None => return Err(1)
        };
        
        let servername_nick_group = caps.name("servername_nick");
        let user_group = caps.name("user");
        let host_group = caps.name("host");

        // The servername_nick group should never be empty if the regex matched
        // Return an internal error (in either this function or the regex crate) if it is
        let servername_nick_str = match servername_nick_group {
            Some(servername_nick_str) => servername_nick_str,
            None => return Err(2)
        };
        // Similarly, the servername_nick group should never contain less than one character
        // Check it just in case of an internal error
        if servername_nick_str.is_empty() {
            return Err(2);
        }

        // All mandatory fields are good... make the struct
        Ok(Prefix {
            servername_nick: FromStr::from_str(servername_nick_str).unwrap(),
            user: match user_group {
                Some(user) => Some(FromStr::from_str(user).unwrap()),
                None => None
            },
            host: match host_group {
                Some(host) => Some(FromStr::from_str(host).unwrap()),
                None => None
            }
        })
    }
}

// Convert a Prefix into a String
impl Into<String> for Prefix {
    fn into(self) -> String {
        let mut result = String::from(":");

        result.push_str(&self.servername_nick);
        match self.user {
            Some(user) => {
                result.push_str("!");
                result.push_str(&user);
            },
            None => ()
        };
        match self.host {
            Some(host) => {
                result.push_str("@");
                result.push_str(&host);
            },
            None => ()
        };

        result
    }
}


// An enumeration of IRC command types
// The command is a mandatory field in IRC messages
// We only implement support for a very small subset - the ones needed to establish and maintain a bare connection
#[derive(Debug)]
pub enum Command {
    ReplyWelcome,
    Pass,
    Nick,
    Join,
    ReplyEndOfNames,
    Privmsg,
    Ping,
    Pong
}

// Create a Command from a str
// Err(1): the command is probably not in our internal enum of IRC commands. Otherwise, it's not an IRC command
impl FromStr for Command {
    type Err = u8;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "001"|"RPL_WELCOME"     =>  Ok(Command::ReplyWelcome),
            "PASS"                  =>  Ok(Command::Pass),
            "NICK"                  =>  Ok(Command::Nick),
            "JOIN"                  =>  Ok(Command::Join),
            "366"|"RPL_ENDOFNAMES"  =>  Ok(Command::ReplyEndOfNames),
            "PRIVMSG"               =>  Ok(Command::Privmsg),
            "PING"                  =>  Ok(Command::Ping),
            "PONG"                  =>  Ok(Command::Pong),
            _                       =>  Err(1)
        }
    }
}

// Convert a Command into a &str
impl Into<&'static str> for Command {
    fn into(self) -> &'static str {
        match self {
            Command::ReplyWelcome => "RPL_WELCOME",
            Command::Pass => "PASS",
            Command::Nick => "NICK",
            Command::Join => "JOIN",
            Command::ReplyEndOfNames => "RPL_ENDOFNAMES",
            Command::Privmsg => "PRIVMSG",
            Command::Ping => "PING",
            Command::Pong => "PONG"
        }
    }
}


// IRC Parameters type
// Parameters are optional in IRC messages, and are a collection of strings that comprise a message's payload
#[derive(Debug)]
pub struct Params(Vec<String>);

// Convert a set of parameters into a String
impl Into<String> for Params {
    fn into(self) -> String {
        let mut result = String::new();

        // C-style... is there a better way to identify the last element?
        let iter = self.iter();
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

        result
    }
}

// It'd be cool if rustc auto-implemented these for single-unit tuple structs
impl ops::Deref for Params {
    type Target = Vec<String>;

    fn deref(&self) -> &Vec<String> {
        &self.0
    }
}
impl ops::DerefMut for Params {
    fn deref_mut<'a>(&'a mut self) -> &'a mut Vec<String> {
        &mut self.0
    }
}

//@todo could we deref for Vec<String> to Params?
impl From<Vec<String>> for Params {
    fn from(v: Vec<String>) -> Self {
        Params(v)
    }
}


// A representation of an IRC message
#[derive(Debug)]
pub struct IrcMessage {
    pub prefix: Option<Prefix>,
    pub command: Command,
    pub params: Option<Params>
}

// Create an IrcMessage from a str
// Err(1): str does not match our IRC regex
// Err(2): command does not match our list of IRC commands
// Err(3): internal error: command group not found
impl FromStr for IrcMessage {
    type Err = u8;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Dissect the message to identify its prefix (if present), its command (if present), and its
        // arguments (if present)
        //@todo probably a bad idea to have written this regex, google "irc regex" and replace this
        let re = Regex::new(r"(^:(?P<prefix>([:alnum:]|[:punct:])+) )?((?P<command>([:alnum:])+)? ?)(?P<params>.*?)\r").unwrap();

        let cap_option = re.captures(s);
        if cap_option.is_none() {
            return Err(1);
        }
        let caps = cap_option.unwrap();

        let prefix_group = caps.name("prefix");
        let command_group = caps.name("command");
        let params_group = caps.name("params");

        // Populate an IrcMessage struct with the fields, splitting up the parameters (if present)
        // into a list of Strings
        Ok(IrcMessage {
            prefix: match prefix_group {
                Some(prefix_str) => match Prefix::from_str(prefix_str) {
                    Ok(prefix) => Some(prefix),
                    Err(_) => None
                },
                None => None
            },

            command: match command_group {
                Some(command_str) => match Command::from_str(command_str){
                    Ok(command) => command,
                    Err(_) => return Err(2)
                },
                _ => return Err(3) // this shouldn't happen - the regex shouldn't have matched if the command group
                                   // is empty
            },

            params: match params_group {
                Some(params_str) => {
                    // Tokenize the parameters.
                    // Tokenizing them is nontrivial, because of the TRAILING parameter pattern.
                    // So we use regex.
                    //@TODO this regex fails if any middle parameter has a colon in it
                    //@TODO or if the trailing parameter has a \r without a following \n
                    //@todo probably a bad idea to have written this regex, google "irc regex" and replace this
                    let re = Regex::new(r"^(?P<middles>[^:]+)?(:(?P<trailing>[^\r\n]+))?").unwrap();
                    let caps = re.captures(params_str).unwrap();

                    let middles_group = caps.name("middles");
                    let trailing_group = caps.name("trailing");

                    let mut paramvec = Vec::new();

                    match middles_group {
                        Some(middles_str)   => {
                            for middle in middles_str.trim().split(" ") {
                                paramvec.push(FromStr::from_str(middle).unwrap());
                            }
                        },
                        _               => ()
                    }

                    match trailing_group {
                        Some(trailing_str)  => { paramvec.push(FromStr::from_str(trailing_str).unwrap()) },
                        _               => ()
                    };

                    Some(Params::from(paramvec))
                },
                None => None
            }
        })
    }
}

// Convert an IrcMessage into a String
// This implementation dumps out a carriage return and line feed at the end of the command, to make it ready for
// sending out an IRC connection
impl Into<String> for IrcMessage {
    // Assumes that the last parameter is the trailing parameter
    fn into(self) -> String {
        let mut result = String::new();

        if let Some(prefix) = self.prefix {
            let prefix_string: String = prefix.into();
            result.push_str(&prefix_string);
            result.push_str(" ");
        }
        
        result.push_str(self.command.into());
        result.push_str(" ");

        if let Some(params) = self.params {
            let params_string: String = params.into();
            result.push_str(&params_string);
        }
        
        result.push_str("\r\n");
        
        result
    }
}


// Public interface to an IRC connection
pub struct IrcStream {
    join_handle: thread::JoinHandle<()>,
    rx_privmsg: mpsc::Receiver<IrcMessage>,
    tx_kill: mpsc::Sender<()>,
}


impl IrcStream {
    // Spawn a thread that establishes and maintains an IRC connection
    pub fn establish(server: String, pass: String, nick: String, channel: String) -> Result<IrcStream, ()> {
        // Establish a TCP stream with our target server
        let mut stream = match TcpStream::connect(&server[..]) {
            Ok(stream) => stream,
            Err(_) => return Err(())
        };
        
        // Create two application-local channels: one for passing received privmsgs to our user app,
        // and one for listening from our user app for a kill command
        let (tx_privmsg, rx_privmsg) = mpsc::channel();
        let (tx_kill, rx_kill) = mpsc::channel();
        
        // Spawn an IRC stream servicing thread. This thread maintains an IRC connection and
        // passes chat messages (privmsgs) through one of its channels
        let join_handle = thread::spawn(move|| {
            let mut connected = false;
            let mut awaiting_endofnames = false;
            
            loop {
                if connected == false && awaiting_endofnames == false {
                    IrcStream::connect_to_channel(&mut stream, &pass, &nick, &channel);
                    awaiting_endofnames = true;
                }
            
                // Check for kill signal; kill this thread if received
                match rx_kill.try_recv() {
                    Ok(()) => return,
                    Err(_) => ()
                }
                
                match IrcStream::get_message(&mut stream) {
                    Ok(message) => match message.command {
                        // as a bot, all we really care about is:
                        // has the server acknowledged our connection?
                        // did the server ping us? if so, pong it
                        // did another client send a message? if so, pass it to our user
                        Command::ReplyEndOfNames => {
                            connected = true;
                            awaiting_endofnames = false;
                        }
                        Command::Ping => {
                            match IrcStream::send_pong(&mut stream) {
                                Ok(_) => (),
                                Err(_) => println!("Unable to send pong!")
                            }
                        },
                        Command::Privmsg => {
                            match tx_privmsg.send(message) {
                                Ok(_) => (),
                                Err(err) => println!("Error sending received IRC message to user\
                                                      app: {}", err)
                            };
                        },
                        _ => ()
                    },
                    Err(num) => match num {
                        1|2|3 => {
                            println!("Got error: {}", num);
                            connected = false;
                            loop {
                                match TcpStream::connect(&server[..]) {
                                    Ok(the_stream) => {
                                        stream = the_stream;
                                        break;
                                    },
                                    Err(_) => { thread::sleep_ms(100); }
                                }
                            }
                        },
                        _ => ()
                    }
                }
            }
        });
        
        Ok( IrcStream { join_handle: join_handle, rx_privmsg: rx_privmsg, tx_kill: tx_kill } )
    }
    
    pub fn join(self) {
        self.join_handle.join();
    }
    
    pub fn receive_privmsg(&self) -> Result<IrcMessage, mpsc::RecvError> {
        self.rx_privmsg.recv()
    }
    
    pub fn kill(&self) {
        self.tx_kill.send(());
    }
    
    // Consider making a Message serializer...
    fn send_message(stream: &mut TcpStream, message: IrcMessage) -> Result<(), ()> {
        let message_string: String = message.into();
        
        match stream.write_all(message_string.as_bytes()) {
            Ok(_) => Ok(()),
            Err(_) => Err(())
        }
    }
    
    fn send_credentials(stream: &mut TcpStream, pass: &String, nick: &String) -> Result<(), ()> {
        let pass_message = IrcMessage { prefix: None, command: Command::Pass, params: Some(Params::from(vec![pass.clone()])) };
        let nick_message = IrcMessage { prefix: None, command: Command::Nick, params: Some(Params::from(vec![nick.clone()])) };
        
        try!(IrcStream::send_message(stream, pass_message));
        try!(IrcStream::send_message(stream, nick_message));
        
        Ok(())
    }

    fn send_join(stream: &mut TcpStream, channel: &String) -> Result<(), ()> {
        let join_message = IrcMessage { prefix: None, command: Command::Join, params: Some(Params::from(vec![channel.clone()])) };
        
        try!(IrcStream::send_message(stream, join_message));
        
        Ok(())
    }

    fn send_pong(stream: &mut TcpStream) -> Result<(), ()> {
        let pong_message = IrcMessage { prefix: None, command: Command::Pong, params: None };
        
        try!(IrcStream::send_message(stream, pong_message));
        
        Ok(())
    }
    
    fn connect_to_channel(stream: &mut TcpStream, pass: &String, nick: &String, channel: &String) {
        // Send the server our credentials
        //@todo implement log in failure recovery
        match IrcStream::send_credentials(stream, pass, nick) {
            Ok(_) => (),
            Err(_) => panic!("Unable to send credentials!")
        }
        
        match IrcStream::send_join(stream, channel) {
            Ok(_) => (),
            Err(_) => panic!("Unable to join target channel!")
        }
    }
    
    // Get a message from an IRC channel.
    //@todo make this nonblocking - any way to do this without function-local static?
    // Err(1): stream EOF - closed by other party
    // Err(2): TCP read error - probably need to reconnect socket
    // Err(3): stream received a non-UTF8 character?
    // Err(4): received unrecognized message
    fn get_message(stream: &mut TcpStream) -> Result<IrcMessage, u8> {
        // Receive a message from the server as raw bytes.
        // We'll convert it to a String once we've received the whole thing, to simplify parsing
        let mut response: Vec<u8> = Vec::new();

        // Read from the TCP stream until we get CRLF (which signals the termination of an IRC message)
        // or the socket read fails
        loop {
            let mut read_byte_vec: Vec<u8> = Vec::with_capacity(1);
            
            let mut read_adaptor = stream.take(1);
            let read_result = read_adaptor.read_to_end(&mut read_byte_vec);
            match read_result {
                Ok(_) => {
                    match read_byte_vec.get(0) {
                        Some(byte) => response.push(*byte),
                        None => return Err(1)
                    }
                    if last_two_are_crlf(&response) {
                        // Convert our raw byte vector into a String for easier, native processing
                        //@TODO Better handle invalid messages
                        match String::from_utf8(response) {
                            Ok(msg_str) => {
                                match IrcMessage::from_str(&msg_str) {
                                    Ok(msg) => return Ok(msg),
                                    _ => return Err(4)
                                }
                            },
                            _ => return Err(3)
                        }
                    }
                },
                Err(_) => return Err(2)
            }
        }
    }
}


fn last_two_are_crlf(vec: &Vec<u8>) -> bool {
    let vec_len = vec.len();

    if vec_len < 2 {
        return false;
    }

    match (vec[vec_len-2], vec[vec_len-1]) {
        (b'\r', b'\n')  => true,
        _               => false
    }
}