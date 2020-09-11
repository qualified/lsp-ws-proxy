use serde::{Deserialize, Serialize};

// NOTE Not using `lsp_types::lsp_notification!` because rust-analyzer
// doesn't seem to understand it well at the moment and shows `{unknown}`.

// TODO Remove this when a new version of `lsp_types` is published with the fix.
// https://github.com/gluon-lang/lsp-types/pull/178
#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DidSaveTextDocumentParams {
    /// The document that was saved.
    pub text_document: lsp_types::TextDocumentIdentifier,
    /// Optional the content when saved. Depends on the includeText value
    /// when the save notification was requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// A [notification message].
///
/// [notification message]: https://microsoft.github.io/language-server-protocol/specifications/specification-current/#notificationMessage
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "method")]
pub(crate) enum Notification {
    // To Server
    // https://microsoft.github.io/language-server-protocol/specifications/specification-current/#initialized
    #[serde(rename = "initialized")]
    Initialized {
        params: lsp_types::InitializedParams,
    },

    // To Server
    // https://microsoft.github.io/language-server-protocol/specifications/specification-current/#exit
    #[serde(rename = "exit")]
    Exit { params: () },

    // To Server
    // https://microsoft.github.io/language-server-protocol/specifications/specification-current/#workspace_didChangeWorkspaceFolders
    #[serde(rename = "workspace/didChangeWorkspaceFolders")]
    DidChangeWorkspaceFolders {
        params: lsp_types::DidChangeWorkspaceFoldersParams,
    },

    // To Server
    // https://microsoft.github.io/language-server-protocol/specifications/specification-current/#workspace_didChangeConfiguration
    #[serde(rename = "workspace/didChangeConfiguration")]
    DidChangeConfiguration {
        params: lsp_types::DidChangeConfigurationParams,
    },

    // To Server
    // https://microsoft.github.io/language-server-protocol/specifications/specification-current/#workspace_didChangeConfiguration
    #[serde(rename = "workspace/didChangeWatchedFiles")]
    DidChangeWatchedFiles {
        params: lsp_types::DidChangeWatchedFilesParams,
    },

    // To Server
    // https://microsoft.github.io/language-server-protocol/specifications/specification-current/#textDocument_didOpen
    #[serde(rename = "textDocument/didOpen")]
    DidOpen {
        params: lsp_types::DidOpenTextDocumentParams,
    },

    // To Server
    // https://microsoft.github.io/language-server-protocol/specifications/specification-current/#textDocument_didChange
    #[serde(rename = "textDocument/didChange")]
    DidChange {
        params: lsp_types::DidChangeTextDocumentParams,
    },

    // To Server
    // https://microsoft.github.io/language-server-protocol/specifications/specification-current/#textDocument_willSave
    #[serde(rename = "textDocument/willSave")]
    WillSave {
        params: lsp_types::WillSaveTextDocumentParams,
    },

    // To Server
    // https://microsoft.github.io/language-server-protocol/specifications/specification-current/#textDocument_didSave
    #[serde(rename = "textDocument/didSave")]
    DidSave { params: DidSaveTextDocumentParams },

    // To Server
    // https://microsoft.github.io/language-server-protocol/specifications/specification-current/#textDocument_didClose
    #[serde(rename = "textDocument/didClose")]
    DidClose {
        params: lsp_types::DidCloseTextDocumentParams,
    },

    // To Client
    // https://microsoft.github.io/language-server-protocol/specifications/specification-current/#window_logMessage
    #[serde(rename = "window/logMessage")]
    LogMessage { params: lsp_types::LogMessageParams },

    // To Client
    // https://microsoft.github.io/language-server-protocol/specifications/specification-current/#window_showMessage
    #[serde(rename = "window/showMessage")]
    ShowMessage {
        params: lsp_types::ShowMessageParams,
    },

    // To Client
    // https://microsoft.github.io/language-server-protocol/specifications/specification-current/#telemetry_event
    #[serde(rename = "telemetry/event")]
    TelemetryEvent { params: serde_json::Value },

    // To Client
    // https://microsoft.github.io/language-server-protocol/specifications/specification-current/#textDocument_publishDiagnostics
    #[serde(rename = "textDocument/publishDiagnostics")]
    PublishDiagnostics {
        params: lsp_types::PublishDiagnosticsParams,
    },

    // To Server/Client
    // https://microsoft.github.io/language-server-protocol/specifications/specification-current/#progress
    #[serde(rename = "$/progress")]
    Progress { params: lsp_types::ProgressParams },

    // To Server/Client
    // https://microsoft.github.io/language-server-protocol/specifications/specification-current/#cancelRequest
    #[serde(rename = "$/cancelRequest")]
    CancelRequest { params: lsp_types::CancelParams },
}
