mod irc;

pub struct TmiStream {
    irc_stream: irc::IrcStream
}

impl TmiStream {
    // Err(1): establishing IRC stream failed
    pub fn establish(server: String, pass: String, nick: String, channel: String) -> Result<Self, u8> {
        match irc::IrcStream::establish(server, pass, nick, channel) {
            Ok(irc_stream) => Ok(TmiStream { irc_stream: irc_stream} ),
            Err(_) => Err(1)
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
                            params.remove(1)
                        } else {
                            return Err(2)
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
