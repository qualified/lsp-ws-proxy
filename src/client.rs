use std::str::FromStr;

use tokio_tungstenite::tungstenite as ws;

use crate::lsp;

// Type to describe a message from the client conveniently.
pub enum Message {
    // Valid LSP message
    Message(lsp::Message),
    // Invalid JSON
    Invalid(String),
    // Close message
    Close(Option<ws::protocol::CloseFrame<'static>>),
}

// Parse the message and ignore anything we don't care.
pub async fn filter_map_ws_message(
    wsm: Result<ws::Message, ws::Error>,
) -> Option<Result<Message, ws::Error>> {
    match wsm {
        Ok(ws::Message::Text(text)) => match lsp::Message::from_str(&text) {
            Ok(msg) => Some(Ok(Message::Message(msg))),
            Err(_) => Some(Ok(Message::Invalid(text))),
        },
        Ok(ws::Message::Close(frame)) => Some(Ok(Message::Close(frame))),
        // Ignore any other message types
        Ok(_) => None,
        Err(err) => Some(Err(err)),
    }
}
