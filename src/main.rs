use std::{
    process::{Command, Stdio},
    str::FromStr,
    time::Duration,
};

use argh::FromArgs;
use async_net::TcpListener;
use async_tungstenite::accept_async;
use async_tungstenite::tungstenite as ws;
use futures_timer::Delay;
use futures_util::{
    future::{select, Either},
    SinkExt, StreamExt,
};
use smol::Unblock;

mod lsp;

// TODO Remap Document URIs
// TODO Synchronize files

#[derive(FromArgs)]
/// Start WebSocket proxy for the LSP Server.
/// Anything after the option delimiter is used to start the server.
struct Options {
    /// port to listen on (default: 9999)
    #[argh(option, short = 'p', default = "9999")]
    port: usize,
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

    smol::run(async {
        let addr = format!("127.0.0.1:{}", opts.port);
        let listener = TcpListener::bind(&addr).await.expect("Failed to bind");
        log::info!("Listening on: {}", addr);

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
            let stdin = Unblock::new(lang_server.stdin.take().unwrap());
            let stdout = Unblock::new(lang_server.stdout.take().unwrap());

            let mut server_send = lsp::framed::writer(stdin);
            let mut server_recv = lsp::framed::reader(stdout);
            let (mut client_send, mut client_recv) = ws_stream.split();

            let mut client_msg = client_recv.next();
            let mut server_msg = server_recv.next();
            // timer for inactivity timeout. It's reset whenever a message comes in.
            let mut timer = Delay::new(timeout);
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
                            timer.reset(timeout);
                        }

                        // Close message from client
                        Either::Left((Some(Ok(ws::Message::Close(_))), p_server_msg)) => {
                            log::info!("Received Close Message");
                            // The connection will terminate when None is received.
                            client_msg = client_recv.next();
                            server_msg = p_server_msg;
                            timer = p_timer;
                            timer.reset(timeout);
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
                            timer.reset(timeout);
                        }

                        // Error with WebSocket message
                        Either::Left((Some(Err(err)), p_server_msg)) => {
                            log::error!("{}", err);
                            client_msg = client_recv.next();
                            server_msg = p_server_msg;
                            timer = p_timer;
                            timer.reset(timeout);
                        }

                        // Error with server message
                        Either::Right((Some(Err(err)), p_client_msg)) => {
                            log::error!("{}", err);
                            client_msg = p_client_msg;
                            server_msg = server_recv.next();
                            timer = p_timer;
                            timer.reset(timeout);
                        }

                        // Connection Closed
                        Either::Left((None, _)) => {
                            log::info!("Connection Closed");
                            clean_up_server(&mut lang_server);
                            break;
                        }

                        // Process exited unexpectedly
                        Either::Right((None, _)) => {
                            log::error!("Server process exited unexpectedly");
                            client_send.send(ws::Message::Close(None)).await?;
                            clean_up_server(&mut lang_server);
                            break;
                        }
                    },

                    // Inactivity timeout reached
                    Either::Right(_) => {
                        log::info!("Inactivity timeout reached. Closing");
                        client_send.send(ws::Message::Close(None)).await?;
                        clean_up_server(&mut lang_server);
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
        // Show generated help message and some examples.
        println!("{}", early_exit.output);
        println!("Examples:");
        println!("  lsp-ws-proxy -- langserver");
        println!("  lsp-ws-proxy -- langserver --stdio");
        println!("  lsp-ws-proxy --port 8888 -- langserver --stdio");
        println!("  lsp-ws-proxy -p 8888 -- langserver --stdio");
        std::process::exit(match early_exit.status {
            Ok(()) => 0,
            Err(()) => 1,
        })
    });

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

fn clean_up_server(lang_server: &mut std::process::Child) {
    match lang_server.try_wait() {
        Ok(Some(status)) => {
            if let Some(code) = status.code() {
                log::info!("Server exited with code: {}", code);
            } else {
                log::info!("Server process terminated by signal");
            }
        }

        Ok(None) => {
            log::info!("Server exited with unknown status");
        }

        Err(err) => {
            log::error!("Server did not exit: {}", err);
            match lang_server.kill() {
                Ok(_) => log::info!("Successfully killed server"),
                Err(err) => match err.kind() {
                    std::io::ErrorKind::InvalidInput => {
                        log::info!("Failed to kill server. Already exited.");
                    }
                    _ => {
                        log::error!("Failed to kill server: {}", err);
                    }
                },
            }
        }
    }
}
