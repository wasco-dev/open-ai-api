use anyhow::{anyhow, bail, Result};
use serde_json::Value;
use tracing::debug;
use wstd::http::{Body, Client, HeaderValue, Request};

mod bindings {
    wit_bindgen::generate!({
        world: "main",
        generate_all,
    });
}

use bindings::exports::wasco_dev::open_ai_api::open_ai_api::{Guest, McpServer, Auth};

struct Component;

impl Guest for Component {
    fn open_ai_prompt(prompt: String, api_key: String, mcp_servers: Option<Vec<McpServer>>) -> String {
        wstd::runtime::block_on(async move {
            match call_openai(&prompt, &api_key, mcp_servers).await {
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

async fn call_openai(prompt: &str, api_key: &str, mcp_servers: Option<Vec<McpServer>>) -> Result<String> {
    let mut body = serde_json::json!({
        "model": "gpt-4.1",
        "input": prompt,
        "stream": false
    });

    if let Some(servers) = mcp_servers {
        let mcp_tools: Vec<Value> = servers.iter().map(|server| {
            let mut tool = serde_json::json!({
                "type": "mcp",
                "server": server.name,
                "url": server.url,
            });
            if let Some(auth) = &server.auth {
                match auth {
                    Auth::Bearer(token) => {
                        tool["authorization"] = serde_json::json!(token);
                    }
                    Auth::ApiKey(key) => {
                        tool["authorization"] = serde_json::json!(key);
                    }
                }
            }
            tool
        }).collect();
        
        if let Some(obj) = body.as_object_mut() {
            obj.insert("tools".to_string(), serde_json::json!(mcp_tools));
        }
    }

    let body_str = body.to_string();

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
        .body(Body::from(body_str))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_mcp_server_no_auth() {
        let server = McpServer {
            name: "my-server".to_string(),
            url: "http://localhost:3000".to_string(),
            auth: None,
        };

        assert_eq!(server.name, "my-server");
        assert_eq!(server.url, "http://localhost:3000");
        assert!(server.auth.is_none());
    }

    #[test]
    fn test_create_mcp_server_bearer_auth() {
        let server = McpServer {
            name: "my-server".to_string(),
            url: "http://localhost:3000".to_string(),
            auth: Some(Auth::Bearer("token123".to_string())),
        };

        assert_eq!(server.name, "my-server");
        assert_eq!(server.url, "http://localhost:3000");
        assert!(server.auth.is_some());

        if let Some(Auth::Bearer(token)) = &server.auth {
            assert_eq!(token, "token123");
        } else {
            panic!("Expected Bearer auth");
        }
    }

    #[test]
    fn test_mcp_server_auth_match_bearer() {
        let server = McpServer {
            name: "test-server".to_string(),
            url: "https://api.example.com/mcp".to_string(),
            auth: Some(Auth::Bearer("secret-token-abc123".to_string())),
        };

        match &server.auth {
            Some(Auth::Bearer(t)) => {
                assert_eq!(t, "secret-token-abc123");
            }
            _ => panic!("Expected Bearer variant"),
        }
    }

    #[test]
    fn test_multiple_mcp_servers() {
        let servers = vec![
            McpServer {
                name: "server-1".to_string(),
                url: "http://localhost:3000".to_string(),
                auth: None,
            },
            McpServer {
                name: "server-2".to_string(),
                url: "http://localhost:4000".to_string(),
                auth: Some(Auth::Bearer("secure-token".to_string())),
            },
        ];

        assert_eq!(servers.len(), 2);
        assert!(servers[0].auth.is_none());
        assert!(servers[1].auth.is_some());
    }

    #[test]
    fn test_create_mcp_server_api_key_auth() {
        let server = McpServer {
            name: "api-server".to_string(),
            url: "http://localhost:5000".to_string(),
            auth: Some(Auth::ApiKey("api-key-123".to_string())),
        };

        assert_eq!(server.name, "api-server");
        assert!(server.auth.is_some());

        if let Some(Auth::ApiKey(key)) = &server.auth {
            assert_eq!(key, "api-key-123");
        } else {
            panic!("Expected ApiKey auth");
        }
    }

    #[test]
    fn test_mixed_auth_types() {
        let servers = vec![
            McpServer {
                name: "bearer-server".to_string(),
                url: "http://localhost:3000".to_string(),
                auth: Some(Auth::Bearer("bearer-token".to_string())),
            },
            McpServer {
                name: "api-key-server".to_string(),
                url: "http://localhost:4000".to_string(),
                auth: Some(Auth::ApiKey("api-key-token".to_string())),
            },
        ];

        match &servers[0].auth {
            Some(Auth::Bearer(t)) => assert_eq!(t, "bearer-token"),
            _ => panic!("Expected Bearer"),
        }

        match &servers[1].auth {
            Some(Auth::ApiKey(t)) => assert_eq!(t, "api-key-token"),
            _ => panic!("Expected ApiKey"),
        }
    }
}
