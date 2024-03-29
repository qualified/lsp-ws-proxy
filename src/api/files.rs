use std::{
    convert::Infallible,
    path::{Path, PathBuf},
};

use lsp_types::{FileChangeType, FileEvent};
use thiserror::Error;
use tokio::fs;
use url::Url;
use warp::{http::StatusCode, Filter, Rejection, Reply};

use super::{json_body, json_response, with_context};

#[derive(Debug, Error)]
enum Error {
    #[error("{0} is not under the project root")]
    NotProjectPath(String),

    #[error("failed to create dirs {path}: {source}")]
    CreateDirs {
        path: String,
        source: std::io::Error,
    },

    #[error("failed to write {path}: {source}")]
    WriteFile {
        path: String,
        source: std::io::Error,
    },

    #[error("failed to remove {path}: {source}")]
    RemoveFile {
        path: String,
        source: std::io::Error,
    },

    #[error("failed to rename {from} to {to}: {source}")]
    RenameFile {
        from: String,
        to: String,
        source: std::io::Error,
    },
}

#[derive(Debug, serde::Deserialize)]
struct Payload {
    operations: Vec<Operation>,
}

/// File operation.
///
/// ```json
/// {"op": "write", "path": "foo.js", "contents": "// foo"}
/// {"op": "remove", "path": "bar.js"}
/// {"op": "rename", "from": "foo.js", "to": "bar.js"}
/// ```
#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(tag = "op", rename_all = "camelCase")]
enum Operation {
    /// Write `contents` to a file at relative `path`.
    ///
    /// This will create a file if it does not exist, and will replace its contents if it does.
    /// Any missing directories are also created.
    Write { path: String, contents: String },

    /// Remove a file at relative `path`.
    ///
    /// Any empty parent directories under `cwd` are also removed.
    /// Errors if `path` doesn't exist, or is not a file.
    Remove { path: String },

    /// Rename a file or directory at relative path `from` to `to`.
    ///
    /// Any missing directories are created.
    /// Any empty parent directories under `cwd` as a result of renaming are removed.
    Rename { from: String, to: String },
}

impl Operation {
    /// Perform operation relative to `cwd`.
    async fn perform<P>(&self, cwd: P, remap: bool) -> Result<Vec<FileEvent>, Error>
    where
        P: AsRef<Path>,
    {
        match self {
            Operation::Write { path, contents } => {
                let apath = get_path(&cwd, path)?;
                tracing::debug!("writing file {:?}", path);
                create_parent_dirs(&cwd, path).await?;
                let create = !apath.exists();
                fs::write(&apath, contents.as_bytes())
                    .await
                    .map_err(|source| Error::WriteFile {
                        path: path.to_owned(),
                        source,
                    })?;

                Ok(vec![FileEvent::new(
                    path_uri(&cwd, path, false, remap),
                    if create {
                        FileChangeType::Created
                    } else {
                        FileChangeType::Changed
                    },
                )])
            }

            Operation::Remove { path } => {
                let apath = get_path(&cwd, path)?;
                tracing::debug!("removing file {:?}", path);
                fs::remove_file(&apath)
                    .await
                    .map_err(|source| Error::RemoveFile {
                        path: path.to_owned(),
                        source,
                    })?;
                remove_empty_parents(&cwd, path).await;

                Ok(vec![FileEvent::new(
                    path_uri(&cwd, path, false, remap),
                    FileChangeType::Deleted,
                )])
            }

            Operation::Rename { from, to } => {
                let src = get_path(&cwd, from)?;
                let dst = get_path(&cwd, to)?;

                tracing::debug!("renaming file {:?} to {:?}", from, to);
                create_parent_dirs(&cwd, to).await?;
                let create = !dst.exists();
                fs::rename(&src, &dst)
                    .await
                    .map_err(|source| Error::RenameFile {
                        from: from.to_owned(),
                        to: to.to_owned(),
                        source,
                    })?;
                remove_empty_parents(&cwd, from).await;

                let is_dir = dst.is_dir();
                Ok(vec![
                    FileEvent::new(path_uri(&cwd, from, is_dir, remap), FileChangeType::Deleted),
                    FileEvent::new(
                        path_uri(&cwd, to, is_dir, remap),
                        if create {
                            FileChangeType::Created
                        } else {
                            FileChangeType::Changed
                        },
                    ),
                ])
            }
        }
    }
}

fn get_path<P>(cwd: P, path: &str) -> Result<PathBuf, Error>
where
    P: AsRef<Path>,
{
    let apath = cwd.as_ref().join(path);
    if !apath.starts_with(&cwd) {
        return Err(Error::NotProjectPath(path.to_owned()));
    }
    Ok(apath)
}

async fn create_parent_dirs<P, Q>(cwd: P, path: Q) -> Result<(), Error>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    if let Some(parent) = path.as_ref().parent() {
        tracing::debug!("creating directories for {:?}", path.as_ref());
        fs::create_dir_all(cwd.as_ref().join(parent))
            .await
            .map_err(|source| Error::CreateDirs {
                path: parent.to_str().expect("utf-8").to_owned(),
                source,
            })?;
    }
    Ok(())
}

/// Remove empty parents of relative `path` after removing or renaming.
async fn remove_empty_parents<P, Q>(cwd: P, path: Q)
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let mut path = path.as_ref();
    while let Some(parent) = path.parent() {
        // Fails if the directory isn't empty.
        if fs::remove_dir(cwd.as_ref().join(parent)).await.is_ok() {
            tracing::debug!("removed empty parent {:?}", parent);
            path = parent
        } else {
            break;
        }
    }
}

fn path_uri<P>(cwd: P, path: &str, is_dir: bool, remap: bool) -> Url
where
    P: AsRef<Path>,
{
    if remap {
        let uri = format!(
            "source://{}{}",
            path,
            if is_dir && !path.ends_with('/') {
                "/"
            } else {
                ""
            }
        );
        Url::parse(&uri).expect("valid uri")
    } else {
        let path = cwd.as_ref().join(path);
        if is_dir {
            Url::from_directory_path(&path).expect("no error")
        } else {
            Url::from_file_path(&path).expect("no error")
        }
    }
}

#[test]
fn test_path_uri() {
    let cwd = "/tmp";
    let path = "foo";
    let is_dir = true;
    let remap = true;
    assert_eq!(
        path_uri(cwd, path, is_dir, remap).to_string(),
        "source://foo/"
    );
    assert_eq!(
        path_uri(cwd, path, !is_dir, remap).to_string(),
        "source://foo"
    );
    assert_eq!(
        path_uri(cwd, path, is_dir, !remap).to_string(),
        "file:///tmp/foo/"
    );
    assert_eq!(
        path_uri(cwd, path, !is_dir, !remap).to_string(),
        "file:///tmp/foo"
    );
}

#[derive(Debug, serde::Serialize)]
struct Response {
    /// `FileEvent`s for `workspace/didChangeWatchedFiles` notification.
    changes: Vec<FileEvent>,
    /// Any errors that occured trying to perform operations.
    #[serde(skip_serializing_if = "Option::is_none")]
    errors: Option<Vec<OperationError>>,
}

#[derive(Debug, serde::Serialize)]
struct OperationError {
    operation: Operation,
    reason: String,
}

#[derive(Debug, Clone)]
pub struct Context {
    pub cwd: PathBuf,
    pub remap: bool,
}

/// Handler for `POST /files`
pub fn handler(ctx: Context) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::post()
        .and(warp::path("files"))
        .and(warp::path::end())
        .and(with_context(ctx))
        .and(json_body::<Payload>())
        .and_then(handle_operations)
}

#[tracing::instrument(level = "debug", skip(ctx, payload))]
async fn handle_operations(ctx: Context, payload: Payload) -> Result<impl Reply, Infallible> {
    let mut errors = Vec::new();
    let mut changes = Vec::new();
    // Do them one by one in order
    for op in payload.operations {
        match op.perform(&ctx.cwd, ctx.remap).await {
            Ok(mut events) => {
                changes.append(&mut events);
            }
            Err(err) => {
                errors.push(OperationError {
                    operation: op,
                    reason: err.to_string(),
                });
            }
        }
    }

    let (errors, status) = if errors.is_empty() {
        (None, StatusCode::OK)
    } else {
        (Some(errors), StatusCode::UNPROCESSABLE_ENTITY)
    };
    Ok(json_response(&Response { changes, errors }, status))
}
