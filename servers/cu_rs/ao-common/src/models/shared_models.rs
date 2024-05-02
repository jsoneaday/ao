use serde::{Deserialize, Serialize};

#[allow(unused)]
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Tag {
    pub name: String,
    pub value: String
}

#[allow(unused)]
#[derive(Deserialize, Clone, Debug)]
pub struct Owner { 
    pub address: String, 
    pub key: String
}