//! Assert the code and message common to fleet tool refusals.
//! Ardor and Ache wrap them in an error object inside a text block.
//! Bloch emits a plain text block with a code prefix for typed refusals.

use serde_json::Value;
use thiserror::Error;

#[derive(Debug, PartialEq, Eq)]
pub struct Refusal<'a> {
    pub code: &'a str,
    pub message: &'a str,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum Error {
    #[error("missing or invalid snake_case refusal code")]
    Code,
    #[error("missing or blank refusal message")]
    Message,
}

/// Validate a decoded Ardor or Ache error envelope.
pub fn from_json(payload: &Value) -> Result<Refusal<'_>, Error> {
    let error = &payload["error"];
    validate(error["code"].as_str(), error["message"].as_str())
}

/// Validate a Bloch typed refusal text block with a `code: message` prefix.
pub fn from_text(text: &str) -> Result<Refusal<'_>, Error> {
    let (code, message) = text.split_once(':').ok_or(Error::Code)?;
    validate(Some(code), Some(message))
}

fn validate<'a>(code: Option<&'a str>, message: Option<&'a str>) -> Result<Refusal<'a>, Error> {
    let code = code
        .filter(|code| {
            code.split('_').all(|word| {
                word.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
                    && word
                        .bytes()
                        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
            })
        })
        .ok_or(Error::Code)?;
    let message = message
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .ok_or(Error::Message)?;
    Ok(Refusal { code, message })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn accepts_observed_refusal_shapes() {
        let expected = Refusal {
            code: "unknown_tool",
            message: "Tool is unavailable",
        };
        assert_eq!(
            from_text("unknown_tool: Tool is unavailable"),
            Ok(Refusal { ..expected })
        );
        for additional in [json!({}), json!({"class":"input"})] {
            let mut error = additional;
            error["code"] = json!(expected.code);
            error["message"] = json!(expected.message);
            assert_eq!(
                from_json(&json!({"error":error})),
                Ok(Refusal { ..expected })
            );
        }
    }

    #[test]
    fn rejects_missing_fields_and_bad_codes() {
        assert_eq!(
            from_json(&json!({"error":{"message":"missing"}})),
            Err(Error::Code)
        );
        for code in [
            "Unknown_tool",
            "unknown-tool",
            "unknown__tool",
            "_unknown",
            "unknown_",
        ] {
            assert_eq!(from_text(&format!("{code}: bad")), Err(Error::Code));
        }
        assert_eq!(from_text("unknown_tool:  "), Err(Error::Message));
    }
}
