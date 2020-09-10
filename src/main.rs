use std::{process::Stdio, str::FromStr, time::Duration};

use argh::FromArgs;
use async_io::Timer;
use async_net::{SocketAddr, TcpListener};
use async_process::{Child, Command};
use async_tungstenite::accept_async;
use async_tungstenite::tungstenite as ws;
use futures_util::{
    future::{select, Either},
    SinkExt, StreamExt,
};

mod lsp;

// TODO Remap Document URIs
// TODO Synchronize files

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
    /// show version and exit
    #[argh(switch, short = 'v')]
    version: bool,
    /// address or localhost's port to listen on (default: 9999)
    #[argh(
        option,
        short = 'l',
        default = "String::from(\"127.0.0.1:9999\")",
        from_str_fn(parse_listen)
    )]
    listen: String,
    // TODO Using seconds for now for simplicity. Maybe accept duration strings like `1h` instead.
    /// inactivity timeout in seconds
    #[argh(option, short = 't', default = "0")]
    timeout: u64,
}

// Large enough value used to disable inactivity timeout.
const NO_TIMEOUT: u64 = 60 * 60 * 24 * 30 * 12;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // TODO Accept option for log level
    env_logger::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let (opts, command) = get_opts_and_command();
    let timeout = if opts.timeout != 0 {
        Duration::from_secs(opts.timeout)
    } else {
        Duration::from_secs(NO_TIMEOUT)
    };

    smol::block_on(async {
        let listener = TcpListener::bind(&opts.listen)
            .await
            .expect("Failed to bind");
        log::info!("Listening on {}", listener.local_addr()?);

        // Only accept single connection. Others will hang.
        if let Ok((stream, _)) = listener.accept().await {
            let ws_stream = accept_async(stream)
                .await
                .expect("Error during the websocket handshake occurred");
            log::info!("Connection Established");

            let mut lang_server = Command::new(&command[0])
                .args(&command[1..])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()?;
            let mut server_send = lsp::framed::writer(lang_server.stdin.take().unwrap());
            let mut server_recv = lsp::framed::reader(lang_server.stdout.take().unwrap());
            let (mut client_send, mut client_recv) = ws_stream.split();

            let mut client_msg = client_recv.next();
            let mut server_msg = server_recv.next();
            // timer for inactivity timeout. It's reset whenever a message comes in.
            let mut timer = Timer::after(timeout);
            loop {
                match select(select(client_msg, server_msg), timer).await {
                    Either::Left((either, p_timer)) => match either {
                        // Message from Client
                        Either::Left((Some(Ok(ws::Message::Text(text))), p_server_msg)) => {
                            inspect_message_from_client(&text);
                            // TODO transform the message
                            server_send.send(text).await?;
                            client_msg = client_recv.next();
                            server_msg = p_server_msg;
                            timer = p_timer;
                            timer.set_after(timeout);
                        }

                        // Close message from client
                        Either::Left((Some(Ok(ws::Message::Close(_))), p_server_msg)) => {
                            log::info!("Received Close Message");
                            // The connection will terminate when None is received.
                            client_msg = client_recv.next();
                            server_msg = p_server_msg;
                            timer = p_timer;
                            timer.set_after(timeout);
                        }

                        // Ignore any other message types from client. Inactivity timer is not rest.
                        Either::Left((Some(Ok(_)), p_server_msg)) => {
                            client_msg = client_recv.next();
                            server_msg = p_server_msg;
                            timer = p_timer;
                        }

                        // Message from Server
                        Either::Right((Some(Ok(text)), p_client_msg)) => {
                            inspect_message_from_server(&text);
                            // TODO transform the message
                            client_send.send(ws::Message::text(text)).await?;
                            client_msg = p_client_msg;
                            server_msg = server_recv.next();
                            timer = p_timer;
                            timer.set_after(timeout);
                        }

                        // Error with WebSocket message
                        Either::Left((Some(Err(err)), p_server_msg)) => {
                            log::error!("{}", err);
                            client_msg = client_recv.next();
                            server_msg = p_server_msg;
                            timer = p_timer;
                            timer.set_after(timeout);
                        }

                        // Error with server message
                        Either::Right((Some(Err(err)), p_client_msg)) => {
                            log::error!("{}", err);
                            client_msg = p_client_msg;
                            server_msg = server_recv.next();
                            timer = p_timer;
                            timer.set_after(timeout);
                        }

                        // Connection Closed
                        Either::Left((None, _)) => {
                            log::info!("Connection Closed");
                            ensure_server_exited(&mut lang_server).await?;
                            break;
                        }

                        // Process exited unexpectedly
                        Either::Right((None, _)) => {
                            log::error!("Server process exited unexpectedly");
                            client_send.send(ws::Message::Close(None)).await?;
                            ensure_server_exited(&mut lang_server).await?;
                            break;
                        }
                    },

                    // Inactivity timeout reached
                    Either::Right(_) => {
                        log::info!("Inactivity timeout reached. Closing");
                        client_send.send(ws::Message::Close(None)).await?;
                        ensure_server_exited(&mut lang_server).await?;
                        break;
                    }
                }
            }
        }
        Ok(())
    })
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

fn inspect_message_from_client(text: &str) {
    if !log::log_enabled!(log::Level::Debug) {
        return;
    }

    match lsp::Message::from_str(&text) {
        Ok(lsp::Message::Notification(notification)) => {
            log::debug!("--> Notification: {:?}", notification);
        }
        Ok(lsp::Message::Response(response)) => {
            log::debug!("--> Response: {:?}", response);
        }
        Ok(lsp::Message::Request(request)) => {
            log::debug!("--> Request: {:?}", request);
        }
        Ok(lsp::Message::Unknown(unknown)) => {
            log::debug!("--> Unknown: {:?}", unknown);
        }
        // Invalid, just let the LSP Server handle it.
        Err(err) => {
            log::error!("--> Invalid: {:?}", err);
        }
    }
}

fn inspect_message_from_server(text: &str) {
    if !log::log_enabled!(log::Level::Debug) {
        return;
    }

    match lsp::Message::from_str(text) {
        Ok(lsp::Message::Notification(notification)) => {
            log::debug!("<-- Notification: {:?}", notification);
        }

        Ok(lsp::Message::Response(response)) => {
            log::debug!("<-- Response: {:?}", response);
        }

        Ok(lsp::Message::Request(request)) => {
            log::debug!("<-- Request: {:?}", request);
        }

        Ok(lsp::Message::Unknown(unknown)) => {
            log::debug!("<-- Unknown: {:?}", unknown);
        }

        Err(err) => {
            log::error!("<-- Invalid: {:?}", err);
        }
    }
}

async fn ensure_server_exited(lang_server: &mut Child) -> Result<(), std::io::Error> {
    match lang_server.try_status()? {
        Some(status) => {
            log::info!("Language Server exited");
            log::info!("Status: {}", status);
            Ok(())
        }

        None => {
            log::info!("Language Server is still alive. Waiting 3s before killing.");
            let timeout = Timer::after(Duration::from_secs(3));
            let status = lang_server.status();
            let status = Box::pin(status);
            match select(status, timeout).await {
                Either::Left((Ok(status), _)) => {
                    log::info!("Language Server exited");
                    log::info!("Status: {}", status);
                    Ok(())
                }
                Either::Left((Err(err), _)) => Err(err),

                Either::Right(_) => {
                    log::info!("Killing Language Server...");
                    match lang_server.kill() {
                        Ok(_) => {
                            log::info!("Killed Language Server");
                            log::info!("Status: {}", lang_server.status().await?);
                            Ok(())
                        }

                        Err(err) => match err.kind() {
                            // The process had already exited
                            std::io::ErrorKind::InvalidInput => {
                                log::info!("Language Server had already exited");
                                log::info!("Status: {}", lang_server.status().await?);
                                Ok(())
                            }

                            _ => {
                                log::error!("Failed to kill Language Server: {}", err);
                                Err(err)
                            }
                        },
                    }
                }
            }
        }
    }
}

fn parse_listen(value: &str) -> Result<String, String> {
    // If a number is given, treat it as a localhost's port number
    if value.chars().all(|c| c.is_ascii_digit()) {
        return Ok(format!("127.0.0.1:{}", value));
    }

    match value.parse::<SocketAddr>() {
        Ok(_) => Ok(String::from(value)),
        Err(_) => Err(format!("{} cannot be parsed as SocketAddr", value)),
    }
}
