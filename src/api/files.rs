use std::{
    convert::Infallible,
    path::{Path, PathBuf},
};

use thiserror::Error;
use tokio::fs;
use warp::{http::StatusCode, reply, Filter, Rejection, Reply};

#[derive(Debug, Error)]
enum Error {
    #[error("path {0} must be relative")]
    NotRelativePath(PathBuf),

    #[error("failed to create dirs {path}: {source}")]
    CreateDirs {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("failed to write {path}: {source}")]
    WriteFile {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("failed to remove {path}: {source}")]
    RemoveFile {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("failed to rename {from} to {to}: {source}")]
    RenameFile {
        from: PathBuf,
        to: PathBuf,
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
    Write { path: PathBuf, contents: String },

    /// Remove a file at relative `path`.
    ///
    /// Errors if the file doesn't exist at path.
    Remove { path: PathBuf },

    /// Rename a file or directory at relative path `from` to `to`.
    /// Any missing directories are created.
    Rename { from: PathBuf, to: PathBuf },
}

impl Operation {
    /// Perform operation relative to `cwd`.
    async fn perform<P: AsRef<Path>>(&self, cwd: P) -> Result<(), Error> {
        match self {
            Operation::Write { path, contents } => {
                ensure_relative(path)?;

                create_parent_dirs(&cwd, path).await?;
                tracing::debug!("writing file {:?}", path);
                let apath = cwd.as_ref().join(path);
                fs::write(&apath, contents.as_bytes())
                    .await
                    .map_err(|source| Error::WriteFile {
                        path: path.to_owned(),
                        source,
                    })
            }

            Operation::Remove { path } => {
                ensure_relative(path)?;

                tracing::debug!("removing file {:?}", path);
                let apath = cwd.as_ref().join(path);
                fs::remove_file(&apath)
                    .await
                    .map_err(|source| Error::RemoveFile {
                        path: path.to_owned(),
                        source,
                    })
            }

            Operation::Rename { from, to } => {
                ensure_relative(from)?;
                ensure_relative(to)?;

                create_parent_dirs(&cwd, to).await?;
                tracing::debug!("renaming file {:?} to {:?}", from, to);
                let src = cwd.as_ref().join(from);
                let dst = cwd.as_ref().join(to);
                fs::rename(&src, &dst)
                    .await
                    .map_err(|source| Error::RenameFile {
                        from: from.to_owned(),
                        to: to.to_owned(),
                        source,
                    })
            }
        }
    }
}

fn ensure_relative<P: AsRef<Path>>(path: P) -> Result<(), Error> {
    let path = path.as_ref();
    if path.is_absolute() {
        Err(Error::NotRelativePath(path.to_owned()))
    } else {
        Ok(())
    }
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
                path: parent.to_owned(),
                source,
            })?;
    }
    Ok(())
}

// TODO? Include `changes` for `workspace/didChangeWatchedFiles` notification?
//       Should respect `remap` option for `uri`.
#[derive(Debug, serde::Serialize)]
struct Response {
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
}

// TODO Handle desrialize error rejection.
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
    // Do them one by one in order
    for op in payload.operations {
        if let Err(err) = op.perform(&ctx.cwd).await {
            errors.push(OperationError {
                operation: op,
                reason: err.to_string(),
            });
        }
    }

    let (errors, status) = if errors.is_empty() {
        (None, StatusCode::OK)
    } else {
        (Some(errors), StatusCode::UNPROCESSABLE_ENTITY)
    };
    Ok(json_response(&Response { errors }, status))
}

fn with_context<T>(ctx: T) -> impl Filter<Extract = (T,), Error = Infallible> + Clone
where
    T: Clone + Send,
{
    warp::any().map(move || ctx.clone())
}

fn json_body<T>() -> impl Filter<Extract = (T,), Error = Rejection> + Clone
where
    T: serde::de::DeserializeOwned + Send,
{
    warp::body::content_length_limit(2 * 1024 * 1024).and(warp::body::json())
}

fn json_response<T: serde::Serialize>(res: &T, status: StatusCode) -> reply::Response {
    reply::with_status(reply::json(res), status).into_response()
}
