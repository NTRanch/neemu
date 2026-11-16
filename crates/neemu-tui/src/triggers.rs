//! The per-member trigger engine (DESIGN D4), rung 1.
//!
//! Rules are data parsed from `triggers.toml`. The engine evaluates
//! typed events, applies the bounded predicate (one named field, one
//! comparison — no expression language), enforces idempotency by
//! event hash and the depth cap, and appends one audit line per
//! evaluation outcome. Firing produces a *candidate*; rung 1 has no
//! torori transport, and the shipped rule is plan-mode, so a
//! candidate is surfaced for a human and never executed here. That
//! is stated in the Fleet panel and in the audit line itself.

use std::collections::HashSet;

use serde_json::{json, Value};

use crate::events::TriggerEvent;
use crate::AUDIT_LINES_MAX;

/// One declared trigger rule.
#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    /// Stable rule id (idempotency keys include it).
    pub id: String,
    /// BLOCH lane stamp, from birth.
    pub origin_lane: String,
    /// BLOCH content stamp, from birth.
    pub content_label: String,
    /// Event name this rule listens for.
    pub on_event: String,
    /// The single named event field the predicate reads.
    pub where_field: String,
    /// Comparison: `eq` or `gt` (numeric).
    pub where_op: String,
    /// Value the field is compared against (string form).
    pub where_value: String,
    /// Target MCP server name.
    pub target_mcp: String,
    /// Target tool on that server.
    pub target_tool: String,
    /// `plan` (human approves) or `auto`.
    pub mode: String,
    /// Rules do not fire for events deeper than this.
    pub max_depth: u32,
    /// Panel-controlled enable state; disabling needs no restart.
    pub enabled: bool,
    /// Honest seam note shown in the Fleet panel.
    pub note: String,
}

/// A firing that matched: a candidate for the Fleet panel.
#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    /// Rule that fired.
    pub rule_id: String,
    /// Target MCP server.
    pub target_mcp: String,
    /// Target tool.
    pub target_tool: String,
    /// `plan` or `auto` (rung 1 never executes either).
    pub mode: String,
    /// Event fields the call arguments would be derived from.
    pub args_from: Value,
    /// Idempotency key (hash of rule id + event payload).
    pub key: String,
    /// Seam note copied from the rule.
    pub note: String,
    /// Set once a human approves the candidate in the panel. The
    /// approval is recorded; rung 1 still does not place the call.
    pub approved: bool,
}

/// Parse the flat `[[rule]]` shape of `triggers.toml`. Deliberately
/// small: string and bare values only, unknown keys ignored, a rule
/// missing any required field is dropped and reported in `errors`.
pub fn parse_rules(text: &str) -> (Vec<Rule>, Vec<String>) {
    let mut rules = Vec::new();
    let mut errors = Vec::new();
    let mut current: Vec<(String, String)> = Vec::new();
    let mut in_rule = false;
    let flush =
        |current: &Vec<(String, String)>, rules: &mut Vec<Rule>, errors: &mut Vec<String>| {
            if current.is_empty() {
                return;
            }
            let get = |key: &str| {
                current
                    .iter()
                    .find(|(k, _)| k == key)
                    .map(|(_, v)| v.clone())
            };
            let required = [
                "id",
                "origin_lane",
                "content_label",
                "on_event",
                "where_field",
                "where_op",
                "where_value",
                "target_mcp",
                "target_tool",
                "mode",
            ];
            if let Some(missing) = required.iter().find(|key| get(key).is_none()) {
                errors.push(format!("rule dropped: missing field {missing}"));
                return;
            }
            rules.push(Rule {
                id: get("id").unwrap_or_default(),
                origin_lane: get("origin_lane").unwrap_or_default(),
                content_label: get("content_label").unwrap_or_default(),
                on_event: get("on_event").unwrap_or_default(),
                where_field: get("where_field").unwrap_or_default(),
                where_op: get("where_op").unwrap_or_default(),
                where_value: get("where_value").unwrap_or_default(),
                target_mcp: get("target_mcp").unwrap_or_default(),
                target_tool: get("target_tool").unwrap_or_default(),
                mode: get("mode").unwrap_or_default(),
                max_depth: get("max_depth")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(1)
                    .min(2),
                enabled: get("enabled").is_none_or(|v| v != "false"),
                note: get("note").unwrap_or_default(),
            });
        };
    for line in text.lines() {
        let line = line.trim();
        if line == "[[rule]]" {
            if in_rule {
                flush(&current, &mut rules, &mut errors);
                current = Vec::new();
            }
            in_rule = true;
            continue;
        }
        if !in_rule || line.starts_with('#') || line.is_empty() {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let value = value.trim().trim_matches('"').to_string();
            current.push((key.trim().to_string(), value));
        }
    }
    if in_rule {
        flush(&current, &mut rules, &mut errors);
    }
    (rules, errors)
}

/// The bundled default rules (the repo's `triggers.toml`).
pub fn default_rules() -> Vec<Rule> {
    parse_rules(include_str!("../triggers.toml")).0
}

fn predicate_matches(rule: &Rule, event: &TriggerEvent) -> bool {
    let Some(field) = event.fields.get(&rule.where_field) else {
        return false;
    };
    match rule.where_op.as_str() {
        "eq" => match field {
            Value::String(s) => *s == rule.where_value,
            Value::Bool(b) => b.to_string() == rule.where_value,
            Value::Number(n) => n.to_string() == rule.where_value,
            _ => false,
        },
        "gt" => field
            .as_f64()
            .zip(rule.where_value.parse::<f64>().ok())
            .is_some_and(|(a, b)| a > b),
        _ => false,
    }
}

/// The rule engine: rules + fired keys + audit lines + candidates.
#[derive(Debug, Default)]
pub struct Engine {
    /// Declared rules, in file order.
    pub rules: Vec<Rule>,
    fired_keys: HashSet<String>,
    /// Bounded JSON audit lines (oldest dropped past the cap).
    pub audit: Vec<String>,
    /// Candidates produced so far, in firing order.
    pub candidates: Vec<Candidate>,
}

impl Engine {
    /// Build an engine from parsed rules.
    pub fn new(rules: Vec<Rule>) -> Engine {
        Engine {
            rules,
            ..Engine::default()
        }
    }

    fn audit_line(&mut self, line: Value) {
        self.audit.push(line.to_string());
        while self.audit.len() > AUDIT_LINES_MAX {
            self.audit.remove(0);
        }
    }

    /// Evaluate one event against every rule. Returns the candidates
    /// produced by this event (usually none).
    pub fn evaluate(&mut self, event: &TriggerEvent) -> Vec<Candidate> {
        let mut produced = Vec::new();
        // Rules are cloned so audit/candidate mutation inside the
        // loop does not fight the borrow of the rule list.
        for rule in self.rules.clone().iter() {
            if rule.on_event != event.name {
                continue;
            }
            let base = json!({
                "rule": rule.id,
                "event": event.name,
                "origin_lane": rule.origin_lane,
                "content_label": rule.content_label,
            });
            if !rule.enabled {
                let mut line = base.clone();
                line["outcome"] = json!("skipped_disabled");
                self.audit_line(line);
                continue;
            }
            if event.depth > rule.max_depth {
                let mut line = base.clone();
                line["outcome"] = json!("dropped_depth");
                line["depth"] = json!(event.depth);
                self.audit_line(line);
                continue;
            }
            if !predicate_matches(rule, event) {
                let mut line = base.clone();
                line["outcome"] = json!("no_match");
                self.audit_line(line);
                continue;
            }
            let key = neemu::sha256_str(&format!(
                "{}:{}",
                rule.id,
                serde_json::to_string(&event.fields).unwrap_or_default()
            ));
            if self.fired_keys.contains(&key) {
                let mut line = base.clone();
                line["outcome"] = json!("dropped_duplicate");
                line["key"] = json!(key);
                self.audit_line(line);
                continue;
            }
            self.fired_keys.insert(key.clone());
            let candidate = Candidate {
                rule_id: rule.id.clone(),
                target_mcp: rule.target_mcp.clone(),
                target_tool: rule.target_tool.clone(),
                mode: rule.mode.clone(),
                args_from: event.fields.clone(),
                key,
                note: rule.note.clone(),
                approved: false,
            };
            let mut line = base.clone();
            line["outcome"] = json!("candidate");
            line["target"] = json!(format!("{}.{}", rule.target_mcp, rule.target_tool));
            line["mode"] = json!(rule.mode);
            line["live_firing"] = json!("not wired in rung 1 (plan-mode candidate only)");
            self.audit_line(line);
            self.candidates.push(candidate.clone());
            produced.push(candidate);
        }
        produced
    }

    /// Toggle one rule's enable state (Fleet panel; no restart).
    pub fn set_enabled(&mut self, rule_id: &str, enabled: bool) {
        for rule in &mut self.rules {
            if rule.id == rule_id {
                rule.enabled = enabled;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn cue_event() -> TriggerEvent {
        TriggerEvent {
            name: "extraction.completed".to_string(),
            fields: json!({"tool": "neemu_emit_torori_cue", "work_id": "work-a"}),
            depth: 0,
        }
    }

    #[test]
    fn bundled_rule_parses_with_lane_stamps() {
        let rules = default_rules();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].origin_lane, "ntranch");
        assert_eq!(rules[0].content_label, "nsfw");
        assert_eq!(rules[0].target_tool, "torori_bundle_ingest");
        assert_eq!(rules[0].mode, "plan");
    }

    #[test]
    fn matching_event_produces_one_candidate() {
        let mut engine = Engine::new(default_rules());
        let produced = engine.evaluate(&cue_event());
        assert_eq!(produced.len(), 1);
        assert_eq!(produced[0].target_mcp, "torori");
        assert!(!engine.audit.is_empty());
    }

    #[test]
    fn duplicate_event_never_refires() {
        let mut engine = Engine::new(default_rules());
        assert_eq!(engine.evaluate(&cue_event()).len(), 1);
        assert_eq!(engine.evaluate(&cue_event()).len(), 0);
        assert!(engine
            .audit
            .iter()
            .any(|line| line.contains("dropped_duplicate")));
    }

    #[test]
    fn non_cue_extraction_does_not_fire() {
        let mut engine = Engine::new(default_rules());
        let event = TriggerEvent {
            name: "extraction.completed".to_string(),
            fields: json!({"tool": "neemu_extract_faces", "work_id": "work-a"}),
            depth: 0,
        };
        assert!(engine.evaluate(&event).is_empty());
    }

    #[test]
    fn depth_beyond_cap_does_not_fire() {
        let mut engine = Engine::new(default_rules());
        let mut event = cue_event();
        event.depth = 2;
        assert!(engine.evaluate(&event).is_empty());
    }

    #[test]
    fn disabled_rule_does_not_fire() {
        let mut engine = Engine::new(default_rules());
        engine.set_enabled("neemu-extraction-to-torori-cue", false);
        assert!(engine.evaluate(&cue_event()).is_empty());
    }

    #[test]
    fn malformed_rule_is_dropped_not_fatal() {
        let (rules, errors) = parse_rules("[[rule]]\nid = \"x\"\n");
        assert!(rules.is_empty());
        assert_eq!(errors.len(), 1);
    }
}
