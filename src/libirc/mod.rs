#![allow(dead_code)]

use std::net::TcpStream;
use std::io::Read;
use std::io::Write;
use std::str;
use std::str::FromStr;
use std::thread;
use std::sync::mpsc;

use regex::Regex;

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

#[derive(Debug)]
struct Message {
    prefix: Option<String>,
    command: Option<Command>,
    params: Option<Vec<String>>
}

fn parse_message(msg: &str) -> Message {
    // Dissect the message to identify its prefix (if present), its command (if present), and its
    // arguments (if present)
    let re = Regex::new(r"(^:(?P<prefix>([:alnum:]|[:punct:])+) )?((?P<command>([:alnum:])+)? ?)(?P<params>.*?)\r").unwrap();
    let cap = re.captures(msg).unwrap();

    let prefix = cap.name("prefix");
    let command = cap.name("command");
    let params = cap.name("params");

    // Populate a Message struct with the fields, splitting up the arguments (if present) into a
    // list of Strings
    let message_struct = Message {
        prefix: match prefix {
            Some(prefix)   => Some(FromStr::from_str(prefix).unwrap()),
            None           => None
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

    //println!("Parsed {:?}", message_struct);

    message_struct
}

fn last_two_are_crlf(myvec: &Vec<u8>) -> bool {
    let myvec_len = myvec.len();

    if myvec_len < 2 {
        return false;
    }

    match (myvec[myvec_len-2], myvec[myvec_len-1]) {
        (b'\r', b'\n')  => true,
        _               => false,
    }
}

// Get a message from an IRC channel.
fn get_message_from_stream(stream: &mut TcpStream) -> Message {
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
    let msg_str = match str::from_utf8(&response) {
        Result::Ok(val)     => val,
        Result::Err(err)    => panic!("{}", err),
    };
    //println!("Got raw message: {:?}", msg_str);

    parse_message(&msg_str)
}

//@todo document
pub fn start(server: String, pass: String, nick: String, channel: String) -> Result<mpsc::Receiver<Vec<String>>, &'static str> {
    let (tx, rx) = mpsc::channel();
    
    thread::spawn(move|| {
        // Connect to the IRC server
        let mut stream = TcpStream::connect(&server[..]).unwrap();

        // Send the server our nick and user
        //@todo make these functions
        let mut pass_string = String::new();
        pass_string.push_str("PASS ");
        pass_string.push_str(&pass[..]);
        pass_string.push_str("\r\n");
        println!("pass_string: {}", pass_string);
        stream.write_all(pass_string.as_bytes());

        let mut nick_string = String::new();
        nick_string.push_str("NICK ");
        nick_string.push_str(&nick[..]);
        nick_string.push_str("\r\n");
        println!("nick_string: {}", nick_string);
        stream.write_all(nick_string.as_bytes());
        
        let mut sent_join = false;
        
        loop {
            let m = get_message_from_stream(&mut stream);
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
                        if sent_join == false {
                            println!("Server welcomed us. Joining target channel");
                            let mut join_string = String::new();
                            join_string.push_str("JOIN ");
                            join_string.push_str(&channel[..]);
                            join_string.push_str("\r\n");
                            println!("join_string: {}", join_string);
                            stream.write_all(join_string.as_bytes());
                            sent_join = true;
                        }
                    },
                    Command::Ping => {
                        stream.write_all("PONG\r\n".as_bytes());
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