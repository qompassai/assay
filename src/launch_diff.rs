//! Compare tool contracts across launch variants.

use crate::probe::ProbeResult;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Diff {
    pub added: BTreeSet<String>,
    pub removed: BTreeSet<String>,
    pub definition_changed: BTreeSet<String>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum Error {
    #[error("probe tools must be an array of uniquely named definitions")]
    InvalidTools,
    #[error("missing launch for profile {0}")]
    MissingProfile(String),
    #[error("unexpected launch for profile {0}")]
    UnexpectedProfile(String),
    #[error("profile {profile} has missing tools {missing:?} and unexpected tools {unexpected:?}")]
    WrongTools {
        profile: String,
        missing: BTreeSet<String>,
        unexpected: BTreeSet<String>,
    },
}

fn definitions(probe: &ProbeResult) -> Result<BTreeMap<&str, &Value>, Error> {
    let mut definitions = BTreeMap::new();
    for tool in probe.tools.as_array().ok_or(Error::InvalidTools)? {
        let name = tool["name"]
            .as_str()
            .filter(|name| !name.is_empty())
            .ok_or(Error::InvalidTools)?;
        if definitions.insert(name, tool).is_some() {
            return Err(Error::InvalidTools);
        }
    }
    Ok(definitions)
}

/// Report tool additions, removals and full definition changes from before to after.
pub fn compare(before: &ProbeResult, after: &ProbeResult) -> Result<Diff, Error> {
    let before = definitions(before)?;
    let after = definitions(after)?;
    let mut diff = Diff::default();
    for (&name, &definition) in &before {
        match after.get(name) {
            None => {
                diff.removed.insert(name.into());
            }
            Some(&other) if definition != other => {
                diff.definition_changed.insert(name.into());
            }
            Some(_) => {}
        }
    }
    for &name in after.keys().filter(|name| !before.contains_key(*name)) {
        diff.added.insert(name.into());
    }
    Ok(diff)
}

/// Require exactly the expected launches and exact tool names in each launch.
pub fn assert_profiles(
    launched: &BTreeMap<&str, ProbeResult>,
    expected: &BTreeMap<&str, BTreeSet<String>>,
) -> Result<(), Error> {
    for (&profile, expected_names) in expected {
        let result = launched
            .get(profile)
            .ok_or_else(|| Error::MissingProfile(profile.into()))?;
        let actual: BTreeSet<_> = definitions(result)?
            .into_keys()
            .map(str::to_owned)
            .collect();
        if &actual != expected_names {
            return Err(Error::WrongTools {
                profile: profile.into(),
                missing: (expected_names - &actual),
                unexpected: (&actual - expected_names),
            });
        }
    }
    if let Some(&profile) = launched
        .keys()
        .find(|profile| !expected.contains_key(*profile))
    {
        return Err(Error::UnexpectedProfile(profile.into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn compares_added_and_removed_names() {
        let before = ProbeResult {
            server_info: json!({}),
            tools: json!([{"name":"old"}]),
        };
        let after = ProbeResult {
            server_info: json!({}),
            tools: json!([{"name":"new"}]),
        };
        let diff = compare(&before, &after).unwrap();
        assert_eq!(diff.added, BTreeSet::from(["new".into()]));
        assert_eq!(diff.removed, BTreeSet::from(["old".into()]));
        assert!(diff.definition_changed.is_empty());
    }

    #[test]
    fn same_name_does_not_hide_a_definition_change() {
        let before = ProbeResult {
            server_info: json!({}),
            tools: json!([{"name":"tool","description":"old"}]),
        };
        let after = ProbeResult {
            server_info: json!({}),
            tools: json!([{"name":"tool","description":"new"}]),
        };
        let diff = compare(&before, &after).unwrap();
        assert_eq!(diff.definition_changed, BTreeSet::from(["tool".into()]));
        assert!(diff.added.is_empty() && diff.removed.is_empty());
    }
}
