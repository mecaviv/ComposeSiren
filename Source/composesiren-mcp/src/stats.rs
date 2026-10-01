//! How many times each tool was called. Read by the About dialog.
//!
//! One tool call takes a lock for a map update; the calls come from a person or
//! an agent, never from the audio thread.

use std::collections::BTreeMap;
use std::sync::Mutex;

use serde_json::{Value, json};

/// Calls and failures of one tool.
#[derive(Default, Clone, Copy)]
struct Count {
    calls: u64,
    errors: u64,
}

/// Per-tool counts since the server started. Shared by every MCP session.
#[derive(Default)]
pub struct Stats {
    tools: Mutex<BTreeMap<String, Count>>,
}

impl Stats {
    /// Record one call of `tool`.
    pub fn record(&self, tool: &str, failed: bool) {
        let mut tools = self.tools.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let count = tools.entry(tool.to_owned()).or_default();
        count.calls += 1;
        count.errors += u64::from(failed);
    }

    /// `{"tools": {"set_parameter": {"calls": 3, "errors": 1}, ...}}`
    #[must_use]
    pub fn to_json(&self) -> Value {
        let tools = self.tools.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let tools: serde_json::Map<String, Value> = tools
            .iter()
            .map(|(name, c)| (name.clone(), json!({"calls": c.calls, "errors": c.errors})))
            .collect();
        json!({ "tools": tools })
    }
}

#[cfg(test)]
mod tests {
    use super::Stats;

    #[test]
    fn counts_calls_and_errors_per_tool() {
        let stats = Stats::default();
        stats.record("set_parameter", false);
        stats.record("set_parameter", true);
        stats.record("send_midi", false);
        let json = stats.to_json();
        assert_eq!(json["tools"]["set_parameter"]["calls"], 2);
        assert_eq!(json["tools"]["set_parameter"]["errors"], 1);
        assert_eq!(json["tools"]["send_midi"]["errors"], 0);
    }
}
