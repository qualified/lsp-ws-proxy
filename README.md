# lsp-ws-proxy

Single binary WebSocket proxy for Language Server.

## Usage

```
$ lsp-ws-proxy --help

Usage: lsp-ws-proxy [-l <listen>] [-t <timeout>]

Start WebSocket proxy for the LSP Server.
Anything after the option delimiter is used to start the server.

Options:
  -l, --listen      address or localhost's port to listen on (default: 9999)
  -t, --timeout     inactivity timeout in seconds
  --help            display usage information

Examples:
  lsp-ws-proxy -- langserver
  lsp-ws-proxy -- langserver --stdio
  lsp-ws-proxy --listen 8888 -- langserver --stdio
  lsp-ws-proxy --listen 0.0.0.0:8888 -- langserver --stdio
  lsp-ws-proxy -l 8888 -- langserver --stdio
```

## Features

- [x] Proxy messages
- [x] Inactivity timeout
- [ ] Remap `DocumentUri`
- [ ] Synchronize files
