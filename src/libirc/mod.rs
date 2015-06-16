#![allow(dead_code)]

use std::net::TcpStream;
use std::io::Read;
use std::io::Write;
use std::str;
use std::str::FromStr;

use regex::Regex;

#[derive(Debug)]
pub enum Command {
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
pub struct Message {
    pub prefix: Option<String>,
    pub command: Option<Command>,
    pub params: Option<Vec<String>>
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

pub fn get_message_from_stream(stream: &mut TcpStream) -> Message {
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
                println!("Stream read exrror: {:?}", error);
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


#[derive(Debug)]
struct LibircConfig <'a> {
    server: &'a str,
    port: u16,
    nick: &'a str,
    pass: &'a str,
    channel: &'a str
}


/*
fn config_is_valid(libirc_config: LibircConfig) -> bool {
    // Check that server is valid domain name
    // Check that port is >0 and < 65536
    // Check that nick is a valid IRC nick
    // Check that user is a valid IRC user
    // Check that channel is a valid IRC channel
    true
}
*/

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
