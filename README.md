# lsp-ws-proxy

WebSocket proxy for Language Servers.

## Usage

```
$ lsp-ws-proxy --help

Usage: lsp-ws-proxy [-l <listen>] [-s] [-r] [-v]

Start WebSocket proxy for the LSP Server.
Anything after the option delimiter is used to start the server.

Multiple servers can be registered by separating each with an option delimiter,
and using the query parameter `name` to specify the command name on connection.
If no query parameter is present, the first one is started.

Examples:
  lsp-ws-proxy -- rust-analyzer
  lsp-ws-proxy -- typescript-language-server --stdio
  lsp-ws-proxy --listen 8888 -- rust-analyzer
  lsp-ws-proxy --listen 0.0.0.0:8888 -- rust-analyzer
  # Register multiple servers.
  # Choose the server with query parameter `name` when connecting.
  lsp-ws-proxy --listen 9999 --sync --remap \
    -- typescript-language-server --stdio \
    -- css-languageserver --stdio \
    -- html-languageserver --stdio

Options:
  -l, --listen      address or port to listen on (default: 0.0.0.0:9999)
  -s, --sync        write text document to disk on save, and enable `/files`
                    endpoint
  -r, --remap       remap relative uri (source://)
  -v, --version     show version and exit
  --help            display usage information
```

## Why?

Remote Language Server is necessary when it's not possible to run the server next to the client.

For example, this can be used to let in-browser editors like [CodeMirror][codemirror] and [Monaco][monaco] to use any Language Servers.
See [qualified/lsps] for an example of using proxied [Rust Analyzer][rust-analyzer] with CodeMirror.

## Features

- [x] Proxy messages
- [x] Synchronize files
- [x] Manipulate remote files with `POST /files`
- [x] Remap relative `DocumentUri` (`source://`)

[codemirror]: https://codemirror.net/
[monaco]: https://microsoft.github.io/monaco-editor/
[qualified/lsps]: https://github.com/qualified/lsps
[rust-analyzer]: https://github.com/rust-analyzer/rust-analyzer
