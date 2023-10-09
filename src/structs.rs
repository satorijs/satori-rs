use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Event {
    pub id: i64,
    #[serde(rename = "type")]
    pub ty: String,
    pub platform: String,
    pub self_id: String,
    pub timestamp: i64,
    pub channel: Option<Channel>,
    pub guild: Option<Guild>,
    pub login: Option<Login>,
    pub member: Option<GuildMember>,
    pub operator: Option<User>,
    pub role: Option<GuildRole>,
    pub user: Option<User>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Channel {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub ty: ChannelType,
    pub parent_id: String,
}

#[derive(Serialize_repr, Deserialize_repr, Clone, Debug)]
#[repr(u8)]
pub enum ChannelType {
    Text = 0,
    Voice = 1,
    Category = 2,
    Direct = 3,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Guild {
    pub id: String,
    pub name: String,
    pub avatar: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Login {
    pub user: Option<User>,
    pub self_id: Option<String>,
    pub platform: Option<String>,
    pub status: Status,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct User {
    pub id: String,
    pub name: String,
    pub avatar: Option<String>,
    pub is_bot: Option<bool>,
}

#[derive(Serialize_repr, Deserialize_repr, Clone, Debug)]
#[repr(u8)]
pub enum Status {
    Offline = 0,
    Online = 1,
    Connect = 2,
    Disconnect = 3,
    Reconnect = 4,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GuildMember {
    pub user: Option<User>,
    pub name: Option<String>,
    pub avatar: Option<String>,
    pub joined_at: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GuildRole {
    pub id: Option<String>,
    pub name: Option<String>,
}
