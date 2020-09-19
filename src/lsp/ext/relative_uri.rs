use std::collections::HashMap;

use url::Url;

use crate::lsp::{Message, Notification, Request, Response, ResponseResult};

/// Remap URI relative to current directory (`source://`) to absolute URI (`file://`).  
/// `source://` was chosen because it's used by [Metals Remote Language Server].
///
/// [Metals Remote Language Server]: https://scalameta.org/metals/docs/contributors/remote-language-server.html
pub(crate) fn remap_relative_uri(msg: &mut Message, cwd: &Url) -> Result<(), std::io::Error> {
    match msg {
        Message::Notification(notification) => remap_notification(notification, cwd)?,
        Message::Request(request) => remap_request(request, cwd)?,
        Message::Response(response) => remap_response(response, cwd)?,
        Message::Unknown(_) => {}
    }
    Ok(())
}

fn remap_notification(notification: &mut Notification, cwd: &Url) -> Result<(), std::io::Error> {
    match notification {
        Notification::DidSave { params: p } => {
            remap_text_document_identifier(&mut p.text_document, cwd)?;
        }

        Notification::DidChangeWorkspaceFolders { params: p } => {
            for folder in &mut p.event.added {
                remap_workspace_folder(folder, cwd)?;
            }
            for folder in &mut p.event.removed {
                remap_workspace_folder(folder, cwd)?;
            }
        }

        Notification::DidChangeWatchedFiles { params: p } => {
            for event in &mut p.changes {
                if let Some(uri) = to_file(&event.uri, cwd)? {
                    event.uri = uri;
                }
            }
        }

        Notification::DidOpen { params: p } => {
            if let Some(uri) = to_file(&p.text_document.uri, cwd)? {
                p.text_document.uri = uri;
            }
        }

        Notification::DidChange { params: p } => {
            if let Some(uri) = to_file(&p.text_document.uri, cwd)? {
                p.text_document.uri = uri;
            }
        }

        Notification::WillSave { params: p } => {
            remap_text_document_identifier(&mut p.text_document, cwd)?;
        }

        Notification::DidClose { params: p } => {
            remap_text_document_identifier(&mut p.text_document, cwd)?;
        }

        Notification::PublishDiagnostics { params: p } => {
            // `to_source` because this goes to client
            if let Some(uri) = to_source(&p.uri, cwd)? {
                p.uri = uri;
            }
        }

        Notification::DidChangeConfiguration { params: _ }
        | Notification::Initialized { params: _ }
        | Notification::Exit { params: _ }
        | Notification::LogMessage { params: _ }
        | Notification::ShowMessage { params: _ }
        | Notification::Progress { params: _ }
        | Notification::CancelRequest { params: _ }
        | Notification::TelemetryEvent { params: _ } => {}
    }

    Ok(())
}

fn remap_request(request: &mut Request, cwd: &Url) -> Result<(), std::io::Error> {
    match request {
        Request::Initialize { id: _, params: p } => {
            if let Some(root_uri) = &p.root_uri {
                if let Some(root_uri) = to_file(root_uri, cwd)? {
                    p.root_uri = Some(root_uri);
                }
            }
            if let Some(folders) = &mut p.workspace_folders {
                for folder in folders {
                    remap_workspace_folder(folder, cwd)?;
                }
            }
        }

        Request::DocumentSymbol { id: _, params: p } => {
            remap_text_document_identifier(&mut p.text_document, cwd)?;
        }

        Request::WillSaveWaitUntil { id: _, params: p } => {
            remap_text_document_identifier(&mut p.text_document, cwd)?;
        }

        Request::Completion { id: _, params: p } => {
            remap_text_document_identifier(&mut p.text_document_position.text_document, cwd)?;
        }

        Request::Hover { id: _, params: p } => {
            remap_text_document_identifier(
                &mut p.text_document_position_params.text_document,
                cwd,
            )?;
        }

        Request::SignatureHelp { id: _, params: p } => {
            remap_text_document_identifier(
                &mut p.text_document_position_params.text_document,
                cwd,
            )?;
        }

        Request::GotoDeclaration { id: _, params: p }
        | Request::GotoDefinition { id: _, params: p }
        | Request::GotoTypeDefinition { id: _, params: p }
        | Request::GotoImplementation { id: _, params: p } => {
            remap_text_document_identifier(
                &mut p.text_document_position_params.text_document,
                cwd,
            )?;
        }

        Request::References { id: _, params: p } => {
            remap_text_document_identifier(&mut p.text_document_position.text_document, cwd)?;
        }

        Request::DocumentHighlight { id: _, params: p } => {
            remap_text_document_identifier(
                &mut p.text_document_position_params.text_document,
                cwd,
            )?;
        }

        Request::CodeAction { id: _, params: p } => {
            remap_text_document_identifier(&mut p.text_document, cwd)?;
        }

        Request::CodeLens { id: _, params: p } => {
            remap_text_document_identifier(&mut p.text_document, cwd)?;
        }

        Request::DocumentLink { id: _, params: p } => {
            remap_text_document_identifier(&mut p.text_document, cwd)?;
        }

        Request::DocumentLinkResolve { id: _, params: p } => {
            if let Some(target) = &p.target {
                if let Some(target) = to_file(target, cwd)? {
                    p.target = Some(target);
                }
            }
        }

        Request::DocumentColor { id: _, params: p } => {
            remap_text_document_identifier(&mut p.text_document, cwd)?;
        }

        Request::ColorPresentation { id: _, params: p } => {
            remap_text_document_identifier(&mut p.text_document, cwd)?;
        }

        Request::Formatting { id: _, params: p } => {
            remap_text_document_identifier(&mut p.text_document, cwd)?;
        }

        Request::RangeFormatting { id: _, params: p } => {
            remap_text_document_identifier(&mut p.text_document, cwd)?;
        }

        Request::OnTypeFormatting { id: _, params: p } => {
            remap_text_document_identifier(&mut p.text_document_position.text_document, cwd)?;
        }

        Request::Rename { id: _, params: p } => {
            remap_text_document_identifier(&mut p.text_document_position.text_document, cwd)?;
        }

        Request::PrepareRename { id: _, params: p } => {
            remap_text_document_identifier(&mut p.text_document, cwd)?;
        }

        Request::FoldingRange { id: _, params: p } => {
            remap_text_document_identifier(&mut p.text_document, cwd)?;
        }

        Request::SelectionRange { id: _, params: p } => {
            remap_text_document_identifier(&mut p.text_document, cwd)?;
        }

        // To Client
        Request::ApplyEdit { id: _, params: p } => {
            remap_workspace_edit(&mut p.edit, cwd)?;
        }

        // To Client
        Request::Configuration { id: _, params: p } => {
            for item in &mut p.items {
                if let Some(scope_uri) = &item.scope_uri {
                    if let Some(scope_uri) = to_source(scope_uri, cwd)? {
                        item.scope_uri = Some(scope_uri);
                    }
                }
            }
        }

        Request::WorkspaceFolders { id: _, params: _ }
        | Request::ShowMessage { id: _, params: _ }
        | Request::CompletionResolve { id: _, params: _ }
        | Request::CodeLensResolve { id: _, params: _ }
        | Request::RegisterCapability { id: _, params: _ }
        | Request::UnregisterCapability { id: _, params: _ }
        | Request::CreateWorkDoneProgress { id: _, params: _ }
        | Request::CancelWorkDoneProgress { id: _, params: _ }
        | Request::Symbol { id: _, params: _ }
        | Request::ExecuteCommand { id: _, params: _ }
        | Request::Shutdown { id: _, params: _ } => {}
    }

    Ok(())
}

fn remap_response(response: &mut Response, cwd: &Url) -> Result<(), std::io::Error> {
    match response {
        Response::Success { id: _, result } => {
            match result {
                ResponseResult::DocumentLinkWithTarget(links) => {
                    for link in links {
                        if let Some(target) = to_source(&link.target, cwd)? {
                            link.target = target;
                        }
                    }
                }

                ResponseResult::DocumentLinkWithTargetResolve(link) => {
                    if let Some(target) = to_source(&link.target, cwd)? {
                        link.target = target;
                    }
                }

                ResponseResult::CodeAction(actions) => {
                    for aoc in actions {
                        match aoc {
                            lsp_types::CodeActionOrCommand::Command(_) => {}
                            lsp_types::CodeActionOrCommand::CodeAction(action) => {
                                if let Some(workspace_edit) = &mut action.edit {
                                    remap_workspace_edit(workspace_edit, cwd)?;
                                }
                            }
                        }
                    }
                }

                ResponseResult::Location(location) => {
                    remap_location(location, cwd)?;
                }

                ResponseResult::Locations(locations) => {
                    for location in locations {
                        remap_location(location, cwd)?;
                    }
                }

                ResponseResult::LocationLinks(links) => {
                    for link in links {
                        if let Some(target_uri) = to_source(&link.target_uri, cwd)? {
                            link.target_uri = target_uri;
                        }
                    }
                }

                ResponseResult::SymbolInfos(syms) => {
                    for sym in syms {
                        remap_location(&mut sym.location, cwd)?;
                    }
                }

                ResponseResult::WorkspaceFolders(folders) => {
                    for folder in folders {
                        // `to_file` because this is a response from Client.
                        if let Some(uri) = to_file(&folder.uri, cwd)? {
                            folder.uri = uri;
                        }
                    }
                }

                ResponseResult::WorkspaceEditWithBoth(edit) => {
                    remap_workspace_edit_changes(&mut edit.changes, cwd)?;
                    remap_document_changes(&mut edit.document_changes, cwd)?;
                }

                ResponseResult::WorkspaceEditWithChanges(edit) => {
                    remap_workspace_edit_changes(&mut edit.changes, cwd)?;
                }

                ResponseResult::WorkspaceEditWithDocumentChanges(edit) => {
                    remap_document_changes(&mut edit.document_changes, cwd)?;
                }

                ResponseResult::Any(_) => {}
            }
        }

        Response::Failure { id: _, error: _ } => {}
    }

    Ok(())
}

fn to_file(uri: &Url, cwd: &Url) -> Result<Option<Url>, std::io::Error> {
    if uri.scheme() == "source" {
        cwd.join(uri.as_str().strip_prefix("source://").unwrap())
            .map_err(map_parse_error)
            .map(Some)
    } else {
        Ok(None)
    }
}

fn to_source(uri: &Url, cwd: &Url) -> Result<Option<Url>, std::io::Error> {
    if uri.scheme() == "file" {
        if let Some(rel) = uri.as_str().strip_prefix(cwd.as_str()) {
            let source_uri = format!("source://{}", rel);
            Url::parse(&source_uri).map_err(map_parse_error).map(Some)
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

fn map_parse_error(err: url::ParseError) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, err)
}

/// Remap `DocumentUri` in `WorkspaceEdit` to use `source://`
fn remap_workspace_edit(
    workspace_edit: &mut lsp_types::WorkspaceEdit,
    cwd: &Url,
) -> Result<(), std::io::Error> {
    if let Some(changes) = &mut workspace_edit.changes {
        remap_workspace_edit_changes(changes, cwd)?;
    }

    if let Some(doc_changes) = &mut workspace_edit.document_changes {
        remap_document_changes(doc_changes, cwd)?;
    }
    Ok(())
}

/// Remap keys of `WorkspaceEdit.changes`
fn remap_workspace_edit_changes(
    changes: &mut HashMap<Url, Vec<lsp_types::TextEdit>>,
    cwd: &Url,
) -> Result<(), std::io::Error> {
    let mut tmp = Vec::with_capacity(changes.len());
    for (key, val) in changes.drain() {
        if let Some(rel) = to_source(&key, cwd)? {
            tmp.push((rel, val));
        } else {
            tmp.push((key, val));
        }
    }
    for (key, val) in tmp {
        changes.insert(key, val);
    }
    Ok(())
}

fn remap_document_changes(
    document_changes: &mut lsp_types::DocumentChanges,
    cwd: &Url,
) -> Result<(), std::io::Error> {
    match document_changes {
        lsp_types::DocumentChanges::Edits(edits) => {
            for edit in edits {
                if let Some(uri) = to_source(&edit.text_document.uri, cwd)? {
                    edit.text_document.uri = uri;
                }
            }
        }

        lsp_types::DocumentChanges::Operations(ops) => {
            for op in ops {
                match op {
                    lsp_types::DocumentChangeOperation::Op(op) => match op {
                        lsp_types::ResourceOp::Create(c) => {
                            if let Some(uri) = to_source(&c.uri, cwd)? {
                                c.uri = uri;
                            }
                        }
                        lsp_types::ResourceOp::Rename(r) => {
                            if let Some(uri) = to_source(&r.old_uri, cwd)? {
                                r.old_uri = uri;
                            }
                            if let Some(uri) = to_source(&r.new_uri, cwd)? {
                                r.new_uri = uri;
                            }
                        }
                        lsp_types::ResourceOp::Delete(d) => {
                            if let Some(uri) = to_source(&d.uri, cwd)? {
                                d.uri = uri;
                            }
                        }
                    },

                    lsp_types::DocumentChangeOperation::Edit(e) => {
                        if let Some(uri) = to_source(&e.text_document.uri, cwd)? {
                            e.text_document.uri = uri;
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Remap `Location.uri` to use `source://`
fn remap_location(location: &mut lsp_types::Location, cwd: &Url) -> Result<(), std::io::Error> {
    if let Some(uri) = to_source(&location.uri, cwd)? {
        location.uri = uri;
    }
    Ok(())
}

/// Remap `TextDocumentIdentifier.uri` to use `file://`
fn remap_text_document_identifier(
    text_document: &mut lsp_types::TextDocumentIdentifier,
    cwd: &Url,
) -> Result<(), std::io::Error> {
    if let Some(uri) = to_file(&text_document.uri, cwd)? {
        text_document.uri = uri;
    }
    Ok(())
}

fn remap_workspace_folder(
    folder: &mut lsp_types::WorkspaceFolder,
    cwd: &Url,
) -> Result<(), std::io::Error> {
    if let Some(uri) = to_file(&folder.uri, cwd)? {
        folder.uri = uri;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use url::Url;

    #[test]
    fn test_to_file() {
        let cwd = Url::from_directory_path(Path::new("/workspace")).unwrap();
        let uri = Url::parse("source://src/main.rs").unwrap();
        let remapped = to_file(&uri, &cwd).unwrap().unwrap();
        assert_eq!(remapped.as_str(), "file:///workspace/src/main.rs");
    }

    #[test]
    fn test_to_source() {
        let cwd = Url::from_directory_path(Path::new("/workspace")).unwrap();
        let uri = Url::from_file_path(Path::new("/workspace/src/main.rs")).unwrap();
        let remapped = to_source(&uri, &cwd).unwrap().unwrap();
        assert_eq!(remapped.as_str(), "source://src/main.rs");
    }
}
