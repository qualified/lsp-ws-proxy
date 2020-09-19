use serde::{Deserialize, Serialize};

use super::types::Id;

// NOTE Not using `lsp_types::lsp_request!` because rust-analyzer
// doesn't seem to understand it well at the moment and shows `{unknown}`.

/// [Request message]. Includes both from the Client and from the Server.
///
/// [Request message]: https://microsoft.github.io/language-server-protocol/specifications/specification-current/#requestMessage
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "method")]
pub enum Request {
    // To Server
    /// > The [initialize] request is sent as the first request from the client
    /// > to the server.
    ///
    /// [initialize]: https://microsoft.github.io/language-server-protocol/specifications/specification-current/#initialize
    #[serde(rename = "initialize")]
    Initialize {
        id: Id,
        params: lsp_types::InitializeParams,
    },

    // To Server
    /// > The [shutdown] request is sent from the client to the server.
    /// > It asks the server to shut down, but to not exit (otherwise the
    /// > response might not be delivered correctly to the client).
    ///
    /// [shutdown]: https://microsoft.github.io/language-server-protocol/specifications/specification-current/#shutdown
    #[serde(rename = "shutdown")]
    Shutdown { id: Id, params: () },

    // To Server
    // https://microsoft.github.io/language-server-protocol/specifications/specification-current/#workspace_symbol
    #[serde(rename = "workspace/symbol")]
    Symbol {
        id: Id,
        params: lsp_types::WorkspaceSymbolParams,
    },

    // To Server
    // https://microsoft.github.io/language-server-protocol/specifications/specification-current/#workspace_executeCommand
    #[serde(rename = "workspace/executeCommand")]
    ExecuteCommand {
        id: Id,
        params: lsp_types::ExecuteCommandParams,
    },

    // To Server
    // https://microsoft.github.io/language-server-protocol/specifications/specification-current/#textDocument_willSaveWaitUntil
    WillSaveWaitUntil {
        id: Id,
        params: lsp_types::WillSaveTextDocumentParams,
    },

    // To Server
    // https://microsoft.github.io/language-server-protocol/specifications/specification-current/#textDocument_completion
    #[serde(rename = "textDocument/completion")]
    Completion {
        id: Id,
        params: lsp_types::CompletionParams,
    },

    // To Server
    // https://microsoft.github.io/language-server-protocol/specifications/specification-current/#completionItem_resolve
    #[serde(rename = "completionItem/resolve")]
    CompletionResolve {
        id: Id,
        params: lsp_types::CompletionItem,
    },

    // To Server
    // https://microsoft.github.io/language-server-protocol/specifications/specification-current/#textDocument_hover
    #[serde(rename = "textDocument/hover")]
    Hover {
        id: Id,
        params: lsp_types::HoverParams,
    },

    // To Server
    // https://microsoft.github.io/language-server-protocol/specifications/specification-current/#textDocument_signatureHelp
    #[serde(rename = "textDocument/signatureHelp")]
    SignatureHelp {
        id: Id,
        params: lsp_types::SignatureHelpParams,
    },

    // To Server
    // https://microsoft.github.io/language-server-protocol/specifications/specification-current/#textDocument_declaration
    #[serde(rename = "textDocument/declaration")]
    GotoDeclaration {
        id: Id,
        params: lsp_types::request::GotoDeclarationParams,
    },

    // To Server
    // https://microsoft.github.io/language-server-protocol/specifications/specification-current/#textDocument_definition
    #[serde(rename = "textDocument/definition")]
    GotoDefinition {
        id: Id,
        params: lsp_types::GotoDefinitionParams,
    },

    // To Server
    // https://microsoft.github.io/language-server-protocol/specifications/specification-current/#textDocument_typeDefinition
    #[serde(rename = "textDocument/typeDefinition")]
    GotoTypeDefinition {
        id: Id,
        params: lsp_types::request::GotoTypeDefinitionParams,
    },

    // To Server
    // https://microsoft.github.io/language-server-protocol/specifications/specification-current/#textDocument_implementation
    #[serde(rename = "textDocument/implementation")]
    GotoImplementation {
        id: Id,
        params: lsp_types::request::GotoImplementationParams,
    },

    // To Server
    // https://microsoft.github.io/language-server-protocol/specifications/specification-current/#textDocument_references
    #[serde(rename = "textDocument/references")]
    References {
        id: Id,
        params: lsp_types::ReferenceParams,
    },

    // To Server
    // https://microsoft.github.io/language-server-protocol/specifications/specification-current/#textDocument_documentHighlight
    #[serde(rename = "textDocument/documentHighlight")]
    DocumentHighlight {
        id: Id,
        params: lsp_types::DocumentHighlightParams,
    },

    // To Server
    // https://microsoft.github.io/language-server-protocol/specifications/specification-current/#textDocument_documentSymbol
    #[serde(rename = "textDocument/documentSymbol")]
    DocumentSymbol {
        id: Id,
        params: lsp_types::DocumentSymbolParams,
    },

    // To Server
    // https://microsoft.github.io/language-server-protocol/specifications/specification-current/#textDocument_codeAction
    #[serde(rename = "textDocument/codeAction")]
    CodeAction {
        id: Id,
        params: lsp_types::CodeActionParams,
    },

    // To Server
    // https://microsoft.github.io/language-server-protocol/specifications/specification-current/#textDocument_codeLens
    #[serde(rename = "textDocument/codeLens")]
    CodeLens {
        id: Id,
        params: lsp_types::CodeLensParams,
    },

    // To Server
    // https://microsoft.github.io/language-server-protocol/specifications/specification-current/#codeLens_resolve
    #[serde(rename = "codeLens/resolve")]
    CodeLensResolve { id: Id, params: lsp_types::CodeLens },

    // To Server
    // https://microsoft.github.io/language-server-protocol/specifications/specification-current/#textDocument_documentLink
    #[serde(rename = "textDocument/documentLink")]
    DocumentLink {
        id: Id,
        params: lsp_types::DocumentLinkParams,
    },

    // To Server
    // https://microsoft.github.io/language-server-protocol/specifications/specification-current/#documentLink_resolve
    #[serde(rename = "documentLink/resolve")]
    DocumentLinkResolve {
        id: Id,
        params: lsp_types::DocumentLink,
    },

    // To Server
    // https://microsoft.github.io/language-server-protocol/specifications/specification-current/#textDocument_documentColor
    #[serde(rename = "textDocument/documentColor")]
    DocumentColor {
        id: Id,
        params: lsp_types::DocumentColorParams,
    },

    // To Server
    // https://microsoft.github.io/language-server-protocol/specifications/specification-current/#textDocument_colorPresentation
    #[serde(rename = "textDocument/colorPresentation")]
    ColorPresentation {
        id: Id,
        params: lsp_types::ColorPresentationParams,
    },

    // To Server
    // https://microsoft.github.io/language-server-protocol/specifications/specification-current/#textDocument_formatting
    #[serde(rename = "textDocument/formatting")]
    Formatting {
        id: Id,
        params: lsp_types::DocumentFormattingParams,
    },

    // To Server
    // https://microsoft.github.io/language-server-protocol/specifications/specification-current/#textDocument_rangeFormatting
    #[serde(rename = "textDocument/rangeFormatting")]
    RangeFormatting {
        id: Id,
        params: lsp_types::DocumentRangeFormattingParams,
    },

    // To Server
    // https://microsoft.github.io/language-server-protocol/specifications/specification-current/#textDocument_onTypeFormatting
    #[serde(rename = "textDocument/onTypeFormatting")]
    OnTypeFormatting {
        id: Id,
        params: lsp_types::DocumentOnTypeFormattingParams,
    },

    // To Server
    // https://microsoft.github.io/language-server-protocol/specifications/specification-current/#textDocument_rename
    #[serde(rename = "textDocument/rename")]
    Rename {
        id: Id,
        params: lsp_types::RenameParams,
    },

    // To Server
    // https://microsoft.github.io/language-server-protocol/specifications/specification-current/#textDocument_prepareRename
    #[serde(rename = "textDocument/prepareRename")]
    PrepareRename {
        id: Id,
        params: lsp_types::TextDocumentPositionParams,
    },

    // To Server
    // https://microsoft.github.io/language-server-protocol/specifications/specification-current/#textDocument_foldingRange
    #[serde(rename = "textDocument/foldingRange")]
    FoldingRange {
        id: Id,
        params: lsp_types::FoldingRangeParams,
    },

    // To Server
    // https://microsoft.github.io/language-server-protocol/specifications/specification-current/#textDocument_selectionRange
    #[serde(rename = "textDocument/selectionRange")]
    SelectionRange {
        id: Id,
        params: lsp_types::SelectionRangeParams,
    },

    // To Server
    // https://microsoft.github.io/language-server-protocol/specifications/specification-current/#window_workDoneProgress_cancel
    #[serde(rename = "window/workDoneProgress/cancel")]
    CancelWorkDoneProgress {
        id: Id,
        params: lsp_types::WorkDoneProgressCancelParams,
    },

    // To Client
    // https://microsoft.github.io/language-server-protocol/specifications/specification-current/#window_showMessageRequest
    #[serde(rename = "window/showMessageRequest")]
    ShowMessage {
        id: Id,
        params: lsp_types::ShowMessageRequestParams,
    },

    // To Client
    // https://microsoft.github.io/language-server-protocol/specifications/specification-current/#client_registerCapability
    #[serde(rename = "client/registerCapability")]
    RegisterCapability {
        id: Id,
        params: lsp_types::RegistrationParams,
    },

    // To Client
    // https://microsoft.github.io/language-server-protocol/specifications/specification-current/#client_unregisterCapability
    #[serde(rename = "client/unregisterCapability")]
    UnregisterCapability {
        id: Id,
        params: lsp_types::UnregistrationParams,
    },

    // To Client
    // https://microsoft.github.io/language-server-protocol/specifications/specification-current/#workspace_workspaceFolders
    #[serde(rename = "workspace/workspaceFolders")]
    WorkspaceFolders { id: Id, params: () },

    // To Client
    // https://microsoft.github.io/language-server-protocol/specifications/specification-current/#workspace_configuration
    #[serde(rename = "workspace/configuration")]
    Configuration {
        id: Id,
        params: lsp_types::ConfigurationParams,
    },

    // To Client
    // https://microsoft.github.io/language-server-protocol/specifications/specification-current/#workspace_applyEdit
    #[serde(rename = "workspace/applyEdit")]
    ApplyEdit {
        id: Id,
        params: lsp_types::ApplyWorkspaceEditParams,
    },

    // To Client
    // https://microsoft.github.io/language-server-protocol/specifications/specification-current/#window_workDoneProgress_create
    #[serde(rename = "window/workDoneProgress/create")]
    CreateWorkDoneProgress {
        id: Id,
        params: lsp_types::WorkDoneProgressCreateParams,
    },
}
