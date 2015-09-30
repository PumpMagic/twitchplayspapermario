mod irc;

use std::fs::File;
use std::io::Read;

use toml;


// Err(1): unable to find value in tree
// Err(2): unable to parse value as string
fn get_toml_value_as_string(tree: &toml::Value, value: &str) -> Result<String, u8> {
    match tree.lookup(value) {
        Some(val) => match val.as_str() {
            Some(val) => Ok(String::from(val)),
            None => Err(2)
        },
        None => Err(1)
    }
}

// Parse the TPPM toml configuration file; return the server, password, nick, and channel
// Err(1): Unable to open config file
// Err(2): Unable to parse config file as TOML
// Err(3): Required parameter missing or malformed
fn parse_config_file(path: &str) -> Result<(String, String, String, String), u8> {
    let mut config_file = match File::open(path) {
        Ok(file) => file,
        Err(_) => return Err(1)
    };
    let mut config_string = String::new();
    config_file.read_to_string(&mut config_string);

    //@todo understand this generics magic
    let toml_tree: toml::Value = match config_string.parse() {
        Ok(tree) => tree,
        Err(_) => return Err(2)
    };

    let server = match get_toml_value_as_string(&toml_tree, "irc.server") {
        Ok(server) => server,
        Err(_) => return Err(3)
    };

    let pass = match get_toml_value_as_string(&toml_tree, "irc.pass") {
        Ok(pass) => pass,
        Err(_) => return Err(3)
    };

    let nick = match get_toml_value_as_string(&toml_tree, "irc.nick") {
        Ok(nick) => nick,
        Err(_) => return Err(3)
    };

    let channel = match get_toml_value_as_string(&toml_tree, "irc.channel") {
        Ok(channel) => channel,
        Err(_) => return Err(3)
    };

    Ok((server, pass, nick, channel))
}


pub struct TmiStream {
    irc_stream: irc::IrcStream
}

impl TmiStream {
    // Err(1): parsing TOML file failed
    // Err(2): establishing IRC stream failed
    pub fn establish(path: &str) -> Result<Self, u8> {
        // Parse our configuration file
        let (server, pass, nick, channel) = match parse_config_file(path) {
            Ok((server, pass, nick, channel)) => (server, pass, nick, channel),
            Err(_) => return Err(1)
        };

        match irc::IrcStream::establish(server, pass, nick, channel) {
            Ok(irc_stream) => Ok(TmiStream { irc_stream: irc_stream} ),
            Err(_) => Err(2)
        }
    }

    // Ok((sender, message))
    // Err(1): unable to identify sender
    // Err(2): unable to identify message payload
    // Err(3): unable to identify message payload
    // Err(4): unable to receive message
    pub fn receive(&self) -> Result<(String, String), u8> {
        match self.irc_stream.receive_privmsg() {
            Ok(msg) => {
                let nick = match msg.prefix {
                    Some(prefix) => prefix.servername_nick,
                    None => return Err(1)
                };
                let message = match msg.params {
                    Some(mut params) => {
                        if params.len() < 2 {
                            return Err(2)
                        } else {
                            params.remove(1)
                        }
                    },
                    None => return Err(3)
                };
                Ok((nick, message))
            },
            Err(_) => Err(4)
        }
    }
}
