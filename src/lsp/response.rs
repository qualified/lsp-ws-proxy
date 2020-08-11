use serde::{Deserialize, Serialize};

use super::error::Error;
use super::types::Id;

/// Untyped [response message]. Either Success or Failure response.
///
/// [response message]: https://microsoft.github.io/language-server-protocol/specifications/specification-current/#responseMessage
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub(crate) enum Response {
    Success { id: Id, result: serde_json::Value },

    Failure { id: Option<Id>, error: Error },
}
