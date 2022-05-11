pub mod error;
pub mod ext;
pub mod framed;
mod notification;
mod request;
mod response;
pub mod types;

use std::{convert::TryFrom, str::FromStr};

use serde::ser::Serializer;
use serde::{Deserialize, Serialize};

pub use notification::Notification;
pub use request::Request;
pub use response::{Response, ResponseResult};
use types::Unknown;

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(untagged)]
#[allow(clippy::large_enum_variant)]
pub enum Message {
    Request(Request),

    Notification(Notification),

    Response(Response),

    Unknown(Unknown),
}

impl From<Request> for Message {
    fn from(request: Request) -> Self {
        Self::Request(request)
    }
}

impl From<Notification> for Message {
    fn from(notification: Notification) -> Self {
        Self::Notification(notification)
    }
}

impl From<Response> for Message {
    fn from(response: Response) -> Self {
        Self::Response(response)
    }
}

impl From<Unknown> for Message {
    fn from(unknown: Unknown) -> Self {
        Self::Unknown(unknown)
    }
}

impl FromStr for Message {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
    }
}

impl TryFrom<serde_json::Value> for Message {
    type Error = serde_json::Error;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        serde_json::from_value(value)
    }
}

// We assume that all messages have `jsonrpc: "2.0"`.
impl Serialize for Message {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        struct WithJsonRpc<'a, T: Serialize> {
            jsonrpc: &'static str,
            #[serde(flatten)]
            msg: &'a T,
        }

        match &self {
            Self::Request(request) => {
                let wrapped = WithJsonRpc {
                    jsonrpc: "2.0",
                    msg: &request,
                };
                wrapped.serialize(serializer)
            }

            Self::Notification(notification) => {
                let wrapped = WithJsonRpc {
                    jsonrpc: "2.0",
                    msg: &notification,
                };
                wrapped.serialize(serializer)
            }

            Self::Response(response) => {
                let wrapped = WithJsonRpc {
                    jsonrpc: "2.0",
                    msg: &response,
                };
                wrapped.serialize(serializer)
            }

            Self::Unknown(unknown) => unknown.serialize(serializer),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn test_request_from_str_or_value() {
        let v = json!({"jsonrpc":"2.0","method":"initialize","params":{"capabilities":{}},"id":1});
        let from_str: Message = serde_json::from_str(&v.to_string()).unwrap();
        let from_value: Message = serde_json::from_value(v).unwrap();
        assert_eq!(from_str, from_value);
    }

    #[test]
    fn test_notification_from_str_or_value() {
        let v = json!({"jsonrpc":"2.0","method":"initialized","params":{}});
        let from_str: Message = serde_json::from_str(&v.to_string()).unwrap();
        let from_value: Message = serde_json::from_value(v).unwrap();
        assert_eq!(from_str, from_value);
    }

    #[test]
    fn test_response_from_str_or_value() {
        let v = json!({"jsonrpc":"2.0","result":{},"id":1});
        let from_str: Message = serde_json::from_str(&v.to_string()).unwrap();
        let from_value: Message = serde_json::from_value(v).unwrap();
        assert_eq!(from_str, from_value);
    }

    #[test]
    fn test_deserialize_unknown() {
        let v = json!({"jsonrpc":"2.0","method":"xinitialize","params":{"capabilities":{}},"id":1});
        let from_str: Message = serde_json::from_str(&v.to_string()).unwrap();
        let from_value: Message = serde_json::from_value(v).unwrap();
        assert_eq!(from_str, from_value);
    }

    #[test]
    fn test_serialize_unknown_notification() {
        let v = json!({"jsonrpc":"2.0","method":"language/status","params":{"message":""}});
        let s = v.to_string();
        let from_value: Message = serde_json::from_value(v).unwrap();
        assert_eq!(serde_json::to_string(&from_value).unwrap(), s);
    }
}
