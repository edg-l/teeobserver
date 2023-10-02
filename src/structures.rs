use std::{collections::HashMap, hash::Hash, sync::Arc};

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize, Clone, PartialEq, Eq, Serialize, Hash)]
pub struct Skin {
    pub name: Option<String>,
    pub color_body: Option<i64>,
    pub color_feet: Option<i64>,
}

#[derive(Debug, Deserialize, Clone, Eq, Serialize)]
pub struct Client {
    pub name: Option<String>,
    pub clan: Option<String>,
    pub country: Option<i64>,
    pub score: Option<i64>,
    pub afk: Option<bool>,
    pub team: Option<i64>,
    pub is_player: Option<bool>,
    pub skin: Option<Skin>,
}

impl PartialEq for Client {
    fn eq(&self, other: &Self) -> bool {
        self.name.eq(&other.name)
    }
}

impl Hash for Client {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state)
    }
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq, Serialize)]
pub struct Map {
    pub name: Option<String>,
    pub sha256: Option<String>,
    pub size: Option<i64>,
    #[serde(flatten)]
    pub other: HashMap<String, Value>,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq, Serialize)]
pub struct Info {
    pub clients: Option<Vec<Client>>,
    pub game_type: Option<String>,
    pub max_clients: Option<i64>,
    pub max_players: Option<i64>,
    pub passworded: Option<bool>,
    pub name: Option<String>,
    pub version: Option<String>,
    #[serde(flatten)]
    pub other: HashMap<String, Value>,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq, Serialize)]
pub struct Server {
    pub addresses: Vec<String>,
    pub location: Option<String>,
    pub info: Info,
    #[serde(flatten)]
    pub other: HashMap<String, Value>,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq, Serialize)]
pub struct ServerList {
    pub servers: Vec<Arc<Server>>,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq, Serialize)]
pub struct ServerListMap {
    pub servers: HashMap<String, Arc<Server>>,
}

#[derive(Debug, Serialize, Clone)]
pub enum MasterEvent {
    ClientJoined(Client, Arc<Server>),
    ClientLeft(Client, Arc<Server>),
    ServerWentOnline(Arc<Server>),
    ServerWentOffline(Arc<Server>),
}
