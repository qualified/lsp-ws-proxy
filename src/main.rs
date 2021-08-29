use std::net::SocketAddr;

use argh::FromArgs;
use tokio::time::Duration;
use url::Url;
use warp::Filter;

mod api;
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
    /// write text document to disk on save, and enable `/files` endpoint
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

    let cwd = std::env::current_dir()?;
    // TODO Limit concurrent connection. Can get messy when `sync` is used.
    // TODO? Keep track of added files and remove them on disconnect?
    let proxy = api::proxy::handler(api::proxy::Context {
        command,
        sync: opts.sync,
        remap: opts.remap,
        cwd: Url::from_directory_path(&cwd).expect("valid url from current dir"),
        timeout,
    });
    let healthz = warp::path::end().and(warp::get()).map(|| "OK");
    let addr = opts.listen.parse::<SocketAddr>().expect("valid addr");
    // Enable `/files` endpoint if sync
    if opts.sync {
        let files = api::files::handler(api::files::Context {
            cwd,
            remap: opts.remap,
        });
        warp::serve(proxy.or(healthz).or(files).recover(api::recover))
            .run(addr)
            .await;
    } else {
        warp::serve(proxy.or(healthz).recover(api::recover))
            .run(addr)
            .await;
    }
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
