use irc;

// parse IRC message into (source user, message) tuple
pub fn parse_irc_message_as_tmi(irc_message: irc::IrcMessage) -> Option<(String, String)> {
    Some((
        match irc_message.prefix {
            Some(prefix) => prefix.servername_nick,
            None => return None
        },
        match irc_message.params {
            Some(mut params) => params.remove(1),
            None => return None
    }))
}