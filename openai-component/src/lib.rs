use anyhow::{anyhow, bail, Result};
use serde_json::Value;
use tracing::debug;
use wstd::http::{Body, Client, HeaderValue, Request};

mod bindings {
    wit_bindgen::generate!({
        world: "ai",
        generate_all,
    });
}

use bindings::exports::wasmcloud::ai::response_handler::Guest;

struct Component;

impl Guest for Component {
    fn prompt_handle(prompt: String, api_key: String) -> String {
        wstd::runtime::block_on(async move {
            match call_openai(&prompt, &api_key).await {
                Ok(text) => text,
                Err(e) => {
                    debug!("[COMPONENT] Error: {e}");
                    format!("Error: {e}")
                }
            }
        })
    }
}

bindings::export!(Component with_types_in bindings);

async fn call_openai(prompt: &str, api_key: &str) -> Result<String> {
    let body = serde_json::json!({
        "model": "gpt-4.1",
        "input": prompt,
        "stream": false
    })
    .to_string();

    let request = Request::post("https://api.openai.com/v1/responses")
        .header(
            "Content-Type",
            HeaderValue::from_str("application/json")
                .map_err(|e| anyhow!("invalid content-type header: {e}"))?,
        )
        .header(
            "Authorization",
            HeaderValue::from_str(&format!("Bearer {api_key}"))
                .map_err(|e| anyhow!("invalid authorization header: {e}"))?,
        )
        .body(Body::from(body))
        .map_err(|e| anyhow!("failed to build request: {e}"))?;

    let response = Client::new()
        .send(request)
        .await
        .map_err(|e| anyhow!("request failed: {e}"))?;

    let status = response.status();
    if !status.is_success() {
        bail!("HTTP {} from OpenAI", status);
    }

    let mut body = response.into_body();
    let text = body
        .str_contents()
        .await
        .map_err(|e| anyhow!("failed to read response body: {e}"))?;

    debug!("[COMPONENT] Response: {} bytes", text.len());

    parse_response(text)
}

fn parse_response(json_str: &str) -> Result<String> {
    let json: Value = serde_json::from_str(json_str)
        .map_err(|e| anyhow!("failed to parse JSON: {e}"))?;

    if let Some(text) = json["output"][0]["content"][0]["text"].as_str() {
        return Ok(text.to_string());
    }

    if let Some(text) = json["output"][0]["content"][0]["output_text"]["text"].as_str() {
        return Ok(text.to_string());
    }

    bail!("no output text found in response")
}
