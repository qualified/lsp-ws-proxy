use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::error::Error;
use super::types::Id;

/// [Response message]. Either Success or Failure response.
///
/// [Response message]: https://microsoft.github.io/language-server-protocol/specifications/specification-current/#responseMessage
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub(crate) enum Response {
    Success { id: Id, result: ResponseResult },

    Failure { id: Option<Id>, error: Error },
}

// Typed results so we can remap relative URI.
// Note that the order is significant because it's deserialized to the first variant that works.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
#[serde(deny_unknown_fields)]
pub(crate) enum ResponseResult {
    // remap location
    // {name,kind,location, tags?,deprecated?,containerName?}[]
    SymbolInfos(Vec<lsp_types::SymbolInformation>),
    // remap targetUri
    // {targetUri,targetRange,targetSelectionRange,originSelectionRange?}[]
    LocationLinks(Vec<lsp_types::LocationLink>),
    // remap uri
    // {uri,range}[]
    Locations(Vec<lsp_types::Location>),
    // remap uri
    // {uri,range}
    Location(lsp_types::Location),
    // remap uri
    // {uri,name}[]
    WorkspaceFolders(Vec<lsp_types::WorkspaceFolder>),
    // remap target
    // {range,target, tooltip?,data?}[]
    DocumentLinkWithTarget(Vec<DocumentLinkWithTarget>),
    // remap target
    // {range,target, tooltip?,data?}
    DocumentLinkWithTargetResolve(DocumentLinkWithTarget),
    // remap if action.edit is present
    // ({title,command, arguments?} | {title, kind?,diagnostics?,edit?,command?,isPreferred?})[]
    CodeAction(lsp_types::CodeActionResponse),
    // remap changes and documentChanges
    // {changes, documentChanges}
    WorkspaceEditWithBoth(WorkspaceEditWithBoth),
    // remap changes
    // {changes}
    WorkspaceEditWithChanges(WorkspaceEditWithChanges),
    // remap documentChanges
    // {documentChanges}
    WorkspaceEditWithDocumentChanges(WorkspaceEditWithDocumentChanges),

    // noremap
    // {name,kind,range,selectionRange, detail?,tags?,deprecated?,children?}[]
    // DocumentSymbols(Vec<lsp_types::DocumentSymbol>),

    // noremap
    // {capabilities: {}, serverInfo?: {}}
    // Initialize(lsp_types::InitializeResult),

    // noremap
    // {contents, range?}
    // Hover(lsp_types::Hover),

    // noremap
    // {signatures, activeSignature?,activeParameter?}
    // SignatureHelp(lsp_types::SignatureHelp),

    // noremap
    // {range,newText}[]
    // Formatting(Vec<lsp_types::TextEdit>),

    // noremap
    // {range,color}[]
    // DocumentColor(Vec<lsp_types::ColorInformation>),

    // noremap
    //   {label, kind?,detail?,documentation?,deprecated?,preselect?,sortText?,
    //           filterText?,insertText?,insertTextFormat?,tetEdit?,additionalTextEdits?,
    //           command?,data?,tags?}[]
    // | {isComplete, items}
    // Must be before `ColorPresentation`
    // Completion(lsp_types::CompletionResponse),

    // noremap
    // {label,textEdit?,additionalTextEdits?}[]
    // ColorPresentation(Vec<lsp_types::ColorPresentation>),

    // noremap
    // {applied}
    // ApplyWorkspaceEdit(lsp_types::ApplyWorkspaceEditResponse),

    // noremap
    // {label, kind?,detail?,documentation?,deprecated?,preselect?,sortText?,
    //         filterText?,insertText?,insertTextFormat?,tetEdit?,additionalTextEdits?,
    //         command?,data?,tags?}[]
    // ResolveCompletionItem(lsp_types::CompletionItem),

    // noremap
    // {title}
    // ShowMessage(lsp_types::MessageActionItem),

    // noremap
    // {start, end} | {range, placeholder}
    // PrepareRename(lsp_types::PrepareRenameResponse),

    // noremap
    // {startLine,endLine, startCharacter?,endCharacter?,kind?}
    // FoldingRange(Vec<lsp_types::FoldingRange>),

    // noremap
    // {range, command?,data?}[]
    // CodeLens(Vec<lsp_types::CodeLens>),

    // noremap
    // {range, parent?}[]
    // SelectionRange(Vec<lsp_types::SelectionRange>),

    // noremap
    // {range, kind?}[]
    // DocumentHighlight(Vec<lsp_types::DocumentHighlight>),

    // noremap
    // {range, command?,data?}
    // CodeLensResolve(lsp_types::CodeLens),

    // noremap
    Any(serde_json::Value),
    // Proposed
    //   SemanticTokensFull(lsp_types::SemanticTokensResult),
    //   SemanticTokensFullDelta(lsp_types::SemanticTokensFullDeltaResult),
    //   SemanticTokensRange(lsp_types::SemanticTokensRangeResult),
    //
    //   CallHierarchyPrepare(Vec<lsp_types::CallHierarchyItem>),
    //   CallHierarchyOutgoingCalls(Vec<lsp_types::CallHierarchyOutgoingCall>),
    //   CallHierarchyIncomingCalls(Vec<lsp_types::CallHierarchyIncomingCall>),
}

// Some custom types to make untagged enum work.
//
// `DocumentLink` (`{range, target?,tooltip?,data?}`) needs to be remapped when `target` is present.
// But using it in untagged enum will deserialize any objects with `range` as `DocumentLink`.
// We define `DocumentLinkWithTarget` (`{range,target, tooltip?,data?}`) to workaround this.
//
// `lsp_types::DocumentLink` with `target` set.
#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
pub(crate) struct DocumentLinkWithTarget {
    pub(crate) range: lsp_types::Range,
    pub(crate) target: url::Url,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tooltip: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) data: Option<serde_json::Value>,
}

// `WorkspaceEdit` (`{changes?, documentChanges?}`) needs to be remapped. But we can't
// match it using untagged enum because both fields are optional making it match any objects.
// We define the following custom types to workaround it.
// - With both: `{changes, documentChanges}`
// - With `changes`: `{changes}`
// - With `documentChanges`: `{documentChanges}`
//
// `lsp_types::WorkspaceEdit` both `changes` and `document_changes`
#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkspaceEditWithBoth {
    pub(crate) changes: HashMap<url::Url, Vec<lsp_types::TextEdit>>,
    pub(crate) document_changes: lsp_types::DocumentChanges,
}

// `lsp_types::WorkspaceEdit` with `changes`
#[derive(Debug, Eq, PartialEq, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkspaceEditWithChanges {
    pub(crate) changes: HashMap<url::Url, Vec<lsp_types::TextEdit>>,
}

// `lsp_types::WorkspaceEdit` with `document_changes`
#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkspaceEditWithDocumentChanges {
    pub(crate) document_changes: lsp_types::DocumentChanges,
}
