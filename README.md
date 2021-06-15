# lsp-ws-proxy

WebSocket proxy for Language Servers.

## Usage

```
$ lsp-ws-proxy --help

Usage: lsp-ws-proxy [-l <listen>] [-t <timeout>] [-s] [-r] [-v]

Start WebSocket proxy for the LSP Server.
Anything after the option delimiter is used to start the server.

Examples:
  lsp-ws-proxy -- langserver
  lsp-ws-proxy -- langserver --stdio
  lsp-ws-proxy --listen 8888 -- langserver --stdio
  lsp-ws-proxy --listen 0.0.0.0:8888 -- langserver --stdio
  lsp-ws-proxy -l 8888 -- langserver --stdio

Options:
  -l, --listen      address or port to listen on (default: 0.0.0.0:9999)
  -t, --timeout     inactivity timeout in seconds
  -s, --sync        write text document to disk on save
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
- [x] Inactivity timeout
- [x] Synchronize files
- [x] Remap relative `DocumentUri` (`source://`)

[codemirror]: https://codemirror.net/
[monaco]: https://microsoft.github.io/monaco-editor/
[qualified/lsps]: https://github.com/qualified/lsps
[rust-analyzer]: https://github.com/rust-analyzer/rust-analyzer
