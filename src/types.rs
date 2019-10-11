use std::collections::HashMap;
use serde_json::Value;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Component {
    pub name: String,
    pub value: Value
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Message {
    pub id: u64,
    pub components: Vec<Component>
}

// Entity is comprised of component names and their data
pub type Entity = HashMap<String,Value>;

// Each entity is tracked by a uid
pub type Entities = HashMap<u64,Entity>;
