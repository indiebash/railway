use serde_json::Value;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Component {
    pub name: String,
    pub value: Value
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Entity {
    pub id: u64,
    pub components: Vec<Component>
}

pub type Entities = Vec<Entity>;
