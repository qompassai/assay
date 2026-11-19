//! Serialize the tools array as compact UTF-8 JSON with object keys sorted
//! lexicographically at every depth. Preserve array order, use serde_json's
//! JSON escaping and number formatting, and append no newline.
//!
//! Count bytes and ordinary cl100k_base tokens with tiktoken-rs 0.7.0.
//! Primo's diet probe used Python tiktoken and compact UTF-8 JSON in wire
//! key order. Sorting is the canonical form specified for this kit.

use serde_json::Value;
use thiserror::Error;

#[derive(Debug, PartialEq, Eq)]
pub struct Measurement {
    pub canonical: String,
    pub bytes: usize,
    pub tokens: usize,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("tools/list payload must be an array")]
    Array,
    #[error("tool {0} needs a nonempty name and an object inputSchema")]
    Tool(usize),
    #[error("cannot serialize tools/list payload: {0}")]
    Json(#[from] serde_json::Error),
}

/// Validate the tools array and measure its canonical encoding.
pub fn measure(tools: &Value) -> Result<Measurement, Error> {
    for (index, tool) in tools.as_array().ok_or(Error::Array)?.iter().enumerate() {
        if !tool["name"].as_str().is_some_and(|name| !name.is_empty())
            || !tool["inputSchema"].is_object()
        {
            return Err(Error::Tool(index));
        }
    }
    let mut sorted = tools.clone();
    sorted.sort_all_objects();
    let canonical = serde_json::to_string(&sorted)?;
    Ok(Measurement {
        bytes: canonical.len(),
        tokens: tiktoken_rs::cl100k_base_singleton()
            .encode_ordinary(&canonical)
            .len(),
        canonical,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn compact_sorted_utf8_preserves_array_order() {
        let tools = json!([
            {"name":"é","inputSchema":{"z":1,"a":2}},
            {"name":"a","inputSchema":{}}
        ]);
        let result = measure(&tools).unwrap();
        assert_eq!(
            result.canonical,
            r#"[{"inputSchema":{"a":2,"z":1},"name":"é"},{"inputSchema":{},"name":"a"}]"#
        );
        assert_eq!(result.bytes, result.canonical.len());
        assert!(result.tokens > 0);
    }

    #[test]
    fn non_array_and_missing_fields_are_rejected() {
        assert!(matches!(measure(&json!({})), Err(Error::Array)));
        for tool in [json!({"name":"x"}), json!({"inputSchema":{}}), json!(null)] {
            assert!(matches!(measure(&json!([tool])), Err(Error::Tool(0))));
        }
    }
}
