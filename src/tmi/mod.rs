mod irc;

pub struct TmiStream {
    irc_stream: irc::IrcConnection
}

pub fn spawn(server: String, pass: String, nick: String, channel: String) -> TmiStream {
    TmiStream { irc_stream: irc::IrcConnection::spawn(server, pass, nick, channel).unwrap() }
}

pub fn receive(stream: &TmiStream) -> (String, String) {
    let msg = stream.irc_stream.receive_privmsg().unwrap();
    let nick = match msg.prefix {
        Some(prefix) => prefix.servername_nick,
        None => format!("poop")
    };
    let message = match msg.params {
        Some(mut params) => params.remove(1),
        None => format!("poop")
    };
    
    return (nick, message)
}