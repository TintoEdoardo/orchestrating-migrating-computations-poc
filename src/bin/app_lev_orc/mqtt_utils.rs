/**********************************/
/*       UTILITIES FOR MQTT       */
/**********************************/

/// This is the message sent through MQTT containing
/// the local update in the ADMM algorithm.
pub struct MessageLocal
{
    pub src       : usize,
    pub local_sum : f32,
}

impl std::str::FromStr for MessageLocal {
    type Err = std::string::ParseError;

    fn from_str (s: &str) -> Result<Self, Self::Err>
    {
        let strs : Vec<&str> = s.split_terminator ('#').collect ();
        match (strs.first (), strs.last ())
        {
            (Some (&str1), Some (&str2)) =>
                {
                    Ok (MessageLocal
                    {
                        src: usize::from_str (str1)
                            .expect ("Unable to convert string to usize"),
                        local_sum: f32::from_str (str2)
                            .expect ("Unable to convert string to f32")
                    })
                }
            _ =>
                {
                    panic! ("Error while parsing a MessageLocal");
                }
        }
    }
}

impl std::fmt::Display for MessageLocal
{
    fn fmt (&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
    {
        write! (f, "{}", format! ("{}#{}", self.src, self.local_sum))
    }
}

pub const BROKER_TOPICS : [&str; 6] =
    [
        "federation/migration",
        "federation/local_update",
        "federation/global_update/",
        "federation/src/",
        "federation/dst/",
        "disconnect",
    ];

pub const REGULAR_TOPICS : [&str; 5] =
    [
        "federation/migration",
        "federation/global_update/",
        "federation/src/",
        "federation/dst/",
        "disconnect",
    ];