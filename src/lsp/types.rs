use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

/// Request ID
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Id {
    /// Numeric ID.
    Number(u64),
    /// String ID.
    String(String),
}

impl Display for Id {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Number(id) => Display::fmt(id, f),
            Self::String(id) => fmt::Debug::fmt(id, f),
        }
    }
}

/// Parameters for Request and Notification.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(untagged)]
pub enum Params {
    Array(Vec<serde_json::Value>),
    Object(serde_json::Map<String, serde_json::Value>),
}

/// Unknown message type.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Unknown(serde_json::Value);
