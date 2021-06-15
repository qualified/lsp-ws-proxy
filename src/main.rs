use std::{convert::Infallible, net::SocketAddr, process::Stdio, str::FromStr};

use argh::FromArgs;
use futures_util::{
    future::{select, Either},
    SinkExt, StreamExt,
};
use tokio::{
    fs::File,
    io::AsyncWriteExt,
    process::Command,
    time::{Duration, Instant},
};
use url::Url;
use warp::Filter;

mod client;
mod lsp;

#[derive(FromArgs)]
// Using block doc comments so that `argh` preserves newlines in help output.
// We need to also write block doc comments without leading space.
/**
Start WebSocket proxy for the LSP Server.
Anything after the option delimiter is used to start the server.

Examples:
  lsp-ws-proxy -- langserver
  lsp-ws-proxy -- langserver --stdio
  lsp-ws-proxy --listen 8888 -- langserver --stdio
  lsp-ws-proxy --listen 0.0.0.0:8888 -- langserver --stdio
  lsp-ws-proxy -l 8888 -- langserver --stdio
*/
struct Options {
    /// address or port to listen on (default: 0.0.0.0:9999)
    #[argh(
        option,
        short = 'l',
        default = "String::from(\"0.0.0.0:9999\")",
        from_str_fn(parse_listen)
    )]
    listen: String,
    // TODO Using seconds for now for simplicity. Maybe accept duration strings like `1h` instead.
    /// inactivity timeout in seconds
    #[argh(option, short = 't', default = "0")]
    timeout: u64,
    /// write text document to disk on save
    #[argh(switch, short = 's')]
    sync: bool,
    /// remap relative uri (source://)
    #[argh(switch, short = 'r')]
    remap: bool,
    /// show version and exit
    #[argh(switch, short = 'v')]
    version: bool,
}

// Large enough value used to disable inactivity timeout.
const NO_TIMEOUT: u64 = 60 * 60 * 24 * 30 * 12;

#[derive(Debug, Clone)]
struct Context {
    command: Vec<String>,
    sync: bool,
    remap: bool,
    cwd: Url,
    timeout: Duration,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_owned()))
        .init();

    let (opts, command) = get_opts_and_command();
    let timeout = if opts.timeout == 0 {
        Duration::from_secs(NO_TIMEOUT)
    } else {
        Duration::from_secs(opts.timeout)
    };

    let ctx = Context {
        command,
        sync: opts.sync,
        remap: opts.remap,
        cwd: Url::from_directory_path(std::env::current_dir()?)
            .expect("valid url from current dir"),
        timeout,
    };

    // TODO Limit concurrent connection. Can get messy when `sync` is used.
    // TODO? Keep track of added files and remove them on disconnect?
    let ws = warp::path::end()
        .and(warp::ws())
        .and(with_context(ctx))
        .map(|ws: warp::ws::Ws, ctx| ws.on_upgrade(move |socket| on_upgrade(socket, ctx)));
    let healthz = warp::path::end().and(warp::get()).map(|| "OK");
    let routes = ws.or(healthz);

    let addr = opts.listen.parse::<SocketAddr>().expect("valid addr");
    warp::serve(routes).run(addr).await;
    Ok(())
}

fn get_opts_and_command() -> (Options, Vec<String>) {
    let strings: Vec<String> = std::env::args().collect();
    let splitted: Vec<&[String]> = strings.splitn(2, |s| *s == "--").collect();
    let strs: Vec<&str> = splitted[0].iter().map(|s| s.as_str()).collect();

    // Parse options or show help and exit.
    let opts = Options::from_args(&[strs[0]], &strs[1..]).unwrap_or_else(|early_exit| {
        // show generated help message
        println!("{}", early_exit.output);
        std::process::exit(match early_exit.status {
            Ok(()) => 0,
            Err(()) => 1,
        })
    });

    if opts.version {
        println!("{} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        std::process::exit(0);
    }

    if splitted.len() != 2 {
        panic!("Command to start the server is required. See --help for examples.");
    }

    (opts, splitted[1].to_vec())
}

#[tracing::instrument(level = "debug", err, skip(msg))]
async fn maybe_write_text_document(msg: &lsp::Message) -> Result<(), std::io::Error> {
    if let lsp::Message::Notification(lsp::Notification::DidSave { params }) = msg {
        if let Some(text) = &params.text {
            let uri = &params.text_document.uri;
            if uri.scheme() == "file" {
                if let Ok(path) = uri.to_file_path() {
                    tracing::debug!("writing to {:?}", path);
                    let mut file = File::create(&path).await?;
                    file.write_all(text.as_bytes()).await?;
                    file.flush().await?;
                }
            }
        }
    }
    Ok(())
}

fn parse_listen(value: &str) -> Result<String, String> {
    // Allow specifying only a port number.
    if value.chars().all(|c| c.is_ascii_digit()) {
        return Ok(format!("0.0.0.0:{}", value));
    }

    match value.parse::<SocketAddr>() {
        Ok(_) => Ok(String::from(value)),
        Err(_) => Err(format!("{} cannot be parsed as SocketAddr", value)),
    }
}

async fn on_upgrade(socket: warp::ws::WebSocket, ctx: Context) {
    tracing::info!("connected");
    if let Err(err) = connected(socket, ctx).await {
        tracing::error!("connection error: {}", err);
    }
    tracing::info!("disconnected");
}

#[tracing::instrument(level = "debug", skip(ws, ctx), fields(command = ?ctx.command[0], remap = %ctx.remap, sync = %ctx.sync))]
async fn connected(
    ws: warp::ws::WebSocket,
    ctx: Context,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing::info!("starting {} in {}", ctx.command[0], ctx.cwd);
    let mut server = Command::new(&ctx.command[0])
        .args(&ctx.command[1..])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;
    tracing::debug!("running {}", ctx.command[0]);

    let mut server_send = lsp::framed::writer(server.stdin.take().unwrap());
    let mut server_recv = lsp::framed::reader(server.stdout.take().unwrap());
    let (mut client_send, client_recv) = ws.split();
    let mut client_recv = client_recv
        .filter_map(client::filter_map_warp_ws_message)
        .boxed();

    let mut client_msg = client_recv.next();
    let mut server_msg = server_recv.next();
    let timer = tokio::time::sleep(ctx.timeout);
    tokio::pin!(timer);

    loop {
        match select(select(client_msg, server_msg), timer).await {
            // From Client
            Either::Left((Either::Left((from_client, p_server_msg)), p_timer)) => {
                match from_client {
                    // Valid LSP message
                    Some(Ok(client::Message::Message(mut msg))) => {
                        if ctx.remap {
                            lsp::ext::remap_relative_uri(&mut msg, &ctx.cwd)?;
                            tracing::debug!("remapped relative URI from client");
                        }
                        if ctx.sync {
                            maybe_write_text_document(&msg).await?;
                        }
                        let text = serde_json::to_string(&msg)?;
                        tracing::debug!("-> {}", text);
                        server_send.send(text).await?;
                    }

                    // Invalid JSON body
                    Some(Ok(client::Message::Invalid(text))) => {
                        tracing::warn!("-> {}", text);
                        // Just forward it to the server as is.
                        server_send.send(text).await?;
                    }

                    // Close message
                    Some(Ok(client::Message::Close)) => {
                        // The connection will terminate when None is received.
                        tracing::info!("received Close message");
                    }

                    // WebSocket Error
                    Some(Err(err)) => {
                        tracing::error!("websocket error: {}", err);
                    }

                    // Connection closed
                    None => {
                        tracing::info!("connection closed");
                        break;
                    }
                }

                client_msg = client_recv.next();
                server_msg = p_server_msg;
                timer = p_timer;
                timer.as_mut().reset(Instant::now() + ctx.timeout);
            }

            // From Server
            Either::Left((Either::Right((from_server, p_client_msg)), p_timer)) => {
                match from_server {
                    // Serialized LSP Message
                    Some(Ok(text)) => {
                        if ctx.remap {
                            if let Ok(mut msg) = lsp::Message::from_str(&text) {
                                lsp::ext::remap_relative_uri(&mut msg, &ctx.cwd)?;
                                tracing::debug!("remapped relative URI from server");
                                let text = serde_json::to_string(&msg)?;
                                tracing::debug!("<- {}", text);
                                client_send.send(warp::ws::Message::text(text)).await?;
                            } else {
                                tracing::warn!("<- {}", text);
                                client_send.send(warp::ws::Message::text(text)).await?;
                            }
                        } else {
                            tracing::debug!("<- {}", text);
                            client_send.send(warp::ws::Message::text(text)).await?;
                        }
                    }

                    // Codec Error
                    Some(Err(err)) => {
                        tracing::error!("{}", err);
                    }

                    // Server exited
                    None => {
                        tracing::error!("server process exited unexpectedly");
                        client_send.send(warp::ws::Message::close()).await?;
                        break;
                    }
                }

                client_msg = p_client_msg;
                server_msg = server_recv.next();
                timer = p_timer;
                timer.as_mut().reset(Instant::now() + ctx.timeout);
            }

            Either::Right(_) => {
                tracing::info!("inactivity timeout reached, closing");
                client_send.send(warp::ws::Message::close()).await?;
                break;
            }
        }
    }

    Ok(())
}

fn with_context(context: Context) -> impl Filter<Extract = (Context,), Error = Infallible> + Clone {
    warp::any().map(move || context.clone())
}
