# OpenAI Component

This is a Rust WebAssembly component that provides core OpenAI API integration functionality. It is designed to be composed with an HTTP proxy component that handles routing and HTTP protocol concerns.

## Features

- Forwards prompts to OpenAI's `/v1/responses` endpoint with the `gpt-4.1` model
- Collects and parses streaming responses from OpenAI's API
- Exports a `open-ai-prompt` WIT interface for composition with other components
- Accepts the OpenAI API key as a function parameter (no environment variable required)
- Supports optional MCP servers with authentication

## Architecture


This component is designed to be **composed** with an HTTP proxy component. It provides the low-level OpenAI API integration, while the HTTP proxy handles:
- HTTP request routing (e.g., `POST /openai-proxy`)
- WASI HTTP protocol handling
- Request/response lifecycle management

Composition is typically done using the `wac` tool:

```bash
wac plug ./build/http_proxy.wasm --plug ../open-ai-api/build/open-ai-api.wasm -o final.wasm
```

## Prerequisites

- `cargo` 1.82+
- [`wash`](https://wasmcloud.com/docs/installation) 0.36.1+
- `wasmtime` >=25.0.0 (if running with wasmtime)
- `wstd` 0.6 (async runtime for WASI components, pulled automatically via Cargo)

## Building


```bash
wash build
```

## Usage


This component **cannot** be run standalone. It must be composed with an HTTP proxy component that provides the HTTP interface layer. See the composition example above.

Once composed, the final component can be run with wasmtime or deployed to wasmCloud. The API key is passed directly to the `prompt-handle` function by the calling component.


## WIT Interface

The component exports a `open-ai-prompt` function defined in the `wasco-dev:open-ai-api/open-ai-api` interface:

```wit
record mcp-server {
    name: string,
    url: string,
    auth: option<auth>,
}

variant auth {
    bearer(string),
    api-key(string),
}

open-ai-prompt: func(prompt: string, api-key: string, mcp-servers: option<list<mcp-server>>) -> string;
```

This function:
- Accepts a text prompt and an OpenAI API key
- Optionally accepts a list of MCP servers with authentication
- Forwards the prompt to the OpenAI API using the provided key for authentication
- Collects and parses the json response, returning the final text as a string

## How It Works

1. The component receives a text prompt, API key, and optional MCP servers via the `open-ai-prompt` function
2. It constructs an HTTP POST request to `https://api.openai.com/v1/responses` using the `wstd` HTTP client
3. The request includes:
   - Model: `gpt-4.1`
   - The user's prompt as input
   - The API key in the `Authorization: Bearer` header
   - Streaming disabled (`"stream": false`)
   - Optional MCP tools if MCP servers are provided
4. The response body is read and the output text is extracted from the JSON structure
