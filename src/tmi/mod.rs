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

    //@todo Make this nonblocking
    pub fn receive(&self) -> (String, String) {
        loop {
            match self.irc_stream.receive_privmsg() {
                Ok(msg) => {
                    let nick = match msg.prefix {
                        Some(prefix) => prefix.servername_nick,
                        None => format!("poop")
                    };
                    let message = match msg.params {
                        Some(mut params) => params.remove(1),
                        None => format!("poop")
                    };
                    return (nick, message);
                },
                Err(_) => ()
            }
        }
    }
}

