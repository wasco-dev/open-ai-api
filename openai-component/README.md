# OpenAI Component

This is a Rust WebAssembly component that provides core OpenAI API integration functionality. It is designed to be composed with an HTTP proxy component that handles routing and HTTP protocol concerns.

## Features

- Forwards prompts to OpenAI's `/v1/responses` endpoint with the `gpt-4.1` model
- Collects and parses streaming responses from OpenAI's API
- Exports a `prompt-handle` WIT interface for composition with other components
- Accepts the OpenAI API key as a function parameter (no environment variable required)

## Architecture


This component is designed to be **composed** with an HTTP proxy component. It provides the low-level OpenAI API integration, while the HTTP proxy handles:
- HTTP request routing (e.g., `POST /openai-proxy`)
- WASI HTTP protocol handling
- Request/response lifecycle management

Composition is typically done using the `wac` tool:

```bash
wac plug ./build/http_proxy.wasm --plug ../openai-component/build/openai_component.wasm -o final.wasm
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

The component exports a `prompt-handle` function defined in the `wasmcloud:ai/response-handler` interface:

```wit
prompt-handle: func(prompt: string, api-key: string) -> string;
```

This function:
- Accepts a text prompt and an OpenAI API key
- Forwards the prompt to the OpenAI API using the provided key for authentication
- Collects and parses the json response, returning the final text as a string

## How It Works

1. The component receives a text prompt and API key via the `prompt-handle` function
2. It constructs an HTTP POST request to `https://api.openai.com/v1/responses` using the `wstd` HTTP client
3. The request includes:
   - Model: `gpt-4.1`
   - The user's prompt as input
   - The API key in the `Authorization: Bearer` header
   - Streaming disabled (`"stream": false`)
4. The response body is read and the output text is extracted from the JSON structure
