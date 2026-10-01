//! A blocking client of ComposeSiren's MCP server (Streamable HTTP on
//! 127.0.0.1, no TLS). Running instances are found in the discovery file.

use serde::Serialize;
use serde_json::{Value, json};

use crate::discovery::{self, Instance};
use crate::tool;

/// A session with one ComposeSiren instance.
pub struct Client {
    url: String,
    session: String,
    next_id: u64,
    /// The instance's plugin name, for messages.
    pub name: String,
    /// The instance's MCP port.
    pub port: u16,
    /// Whether the instance is the standalone (the only one that owns the devices).
    pub standalone: bool,
}

/// The JSON objects in a Streamable HTTP reply: a JSON body, or SSE `data:` lines.
fn messages(body: &str) -> Vec<Value> {
    if let Ok(v) = serde_json::from_str::<Value>(body) {
        return vec![v];
    }
    body.lines().filter_map(|l| l.strip_prefix("data:")).filter_map(|d| serde_json::from_str(d.trim()).ok()).collect()
}

impl Client {
    fn post(url: &str, session: Option<&str>, body: &Value) -> Result<(Option<String>, Vec<Value>), String> {
        let mut request = ureq::post(url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream");
        if let Some(s) = session {
            request = request.header("mcp-session-id", s);
        }
        let mut response = request.send(body.to_string()).map_err(|e| format!("{url}: {e}"))?;
        let session = response.headers().get("mcp-session-id").and_then(|v| v.to_str().ok()).map(str::to_owned);
        let text = response.body_mut().read_to_string().map_err(|e| e.to_string())?;
        Ok((session, messages(&text)))
    }

    /// A session with `instance`.
    pub fn open(instance: &Instance) -> Result<Self, String> {
        let url = format!("http://127.0.0.1:{}/mcp", instance.port);
        let hello = json!({"jsonrpc": "2.0", "id": 0, "method": "initialize", "params": {
            "protocolVersion": "2025-03-26", "capabilities": {}, "clientInfo": {"name": "composesiren-mcp-api", "version": "1"}}});
        let (session, _) = Self::post(&url, None, &hello)?;
        let session = session.ok_or("the server gave no session id")?;
        Self::post(&url, Some(&session), &json!({"jsonrpc": "2.0", "method": "notifications/initialized"}))?;
        Ok(Client {
            url,
            session,
            next_id: 1,
            name: instance.plugin_name.clone(),
            port: instance.port,
            standalone: instance.standalone,
        })
    }

    fn request(&mut self, method: &str, params: Value) -> Result<Value, String> {
        let id = self.next_id;
        self.next_id += 1;
        let (_, replies) =
            Self::post(&self.url, Some(&self.session), &json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}))?;
        let reply = replies.into_iter().find(|m| m.get("id") == Some(&json!(id))).ok_or("no reply")?;
        if let Some(e) = reply.get("error") {
            return Err(e.get("message").and_then(Value::as_str).unwrap_or("error").to_owned());
        }
        Ok(reply["result"].clone())
    }

    /// The names of the tools this instance offers.
    pub fn tools(&mut self) -> Result<Vec<String>, String> {
        let reply = self.request("tools/list", json!({}))?;
        Ok(reply["tools"]
            .as_array()
            .map(|tools| tools.iter().filter_map(|t| t["name"].as_str().map(str::to_owned)).collect())
            .unwrap_or_default())
    }

    /// Whether this instance offers `tool`.
    pub fn has_tool(&mut self, tool: &str) -> bool {
        self.tools().is_ok_and(|tools| tools.iter().any(|t| t == tool))
    }

    /// Every running instance (on `port`, if given) that offers `tool`. None when ComposeSiren
    /// is not running, with no error: for what is only done if it can be.
    pub fn connect_all(port: Option<u16>, tool: &str) -> Vec<Self> {
        let mut found = Vec::new();
        for instance in discovery::running().unwrap_or_default() {
            if port.is_some_and(|want| want != instance.port) {
                continue;
            }
            if let Ok(mut client) = Self::open(&instance) {
                if client.has_tool(tool) {
                    found.push(client);
                }
            }
        }
        found
    }

    /// The instance on `port`, or the first running one that offers `tool`
    /// (`missing` says why one might not).
    pub fn connect_with(port: Option<u16>, tool: &str, missing: &str) -> Result<Self, String> {
        let path = discovery::default_path();
        let instances = discovery::read(&path)
            .map_err(|e| format!("{e} (is ComposeSiren running with its MCP server?)"))?;
        let mut tried = Vec::new();
        for instance in instances {
            if port.is_some_and(|want| want != instance.port) || !discovery::pid_alive(instance.pid) {
                continue;
            }
            match Self::open(&instance) {
                Ok(mut client) => {
                    if client.has_tool(tool) {
                        return Ok(client);
                    }
                    tried.push(format!("{} on port {} ({missing})", client.name, client.port));
                }
                Err(e) => tried.push(e),
            }
        }
        Err(if tried.is_empty() {
            format!("no running ComposeSiren instance (see {})", path.display())
        } else {
            format!("no instance has {tool}: {}", tried.join("; "))
        })
    }

    /// The instance on `port`, or the first running one that can record.
    pub fn connect(port: Option<u16>) -> Result<Self, String> {
        Self::connect_with(port, tool::START_RECORDING, "built without COMPOSESIREN_RECORD")
    }

    /// Calls a tool; its JSON reply (the plugin's `ok: false` is an error).
    pub fn call(&mut self, tool: &str, arguments: Value) -> Result<Value, String> {
        let result = self.request("tools/call", json!({"name": tool, "arguments": arguments}))?;
        let text = result["content"][0]["text"].as_str().unwrap_or("{}");
        let value: Value = serde_json::from_str(text).unwrap_or(json!({"error": text}));
        if result["isError"] == json!(true) || value["ok"] == json!(false) {
            return Err(value["error"].as_str().unwrap_or(text).to_owned());
        }
        Ok(value)
    }

    /// Calls a tool with its argument type from [`crate::args`].
    pub fn call_with<A: Serialize>(&mut self, tool: &str, arguments: &A) -> Result<Value, String> {
        self.call(tool, serde_json::to_value(arguments).map_err(|e| e.to_string())?)
    }
}
