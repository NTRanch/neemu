//! Launch-time tool profiles (D-934 surface diet).
//!
//! A profile deterministically scopes the tool catalog a launched
//! neemu process serves: a tool outside the profile is unknown to
//! that process (not merely unlisted), because both `tools/list`
//! and `tools/call` in `server` consult the same profile. Nothing
//! is deleted, renamed, or schema-changed — the full surface is
//! one explicit `--profile full` launch away.
//!
//! Keying: neemu's served descriptors carry no lane label (the
//! census of record measured `_meta` absent on all 15 tools), so
//! the pack table below keys on tool name, and each tool's home is
//! justified by its dispatch handler's stage ownership (SPEC §7):
//! S1 intake handlers are `Neemu` methods in `lib` (register /
//! validate / screen), the S1 image lane lives in `tools`
//! (segment / faces / dialogue), S2 handlers live in `s2`
//! (enrich / acts / understand), S3 handlers live in `s3`
//! (voice), and the registry spine (list / export) lives in
//! `tools` and is always served as core. A unit test over the
//! real catalog (`tools::TOOL_NAMES`) enforces the coverage law:
//! every tool belongs to exactly one pack or to core, so a future
//! tool cannot ship without a pack decision.
//!
//! Resolution precedence (highest first): `--profile` argument →
//! `NEEMU_TOOL_PROFILE` environment variable → `tool_profile` in
//! the registry root's `neemu.toml` → default `core`. Unknown pack
//! names, empty segments, duplicates, and `full` combined with
//! anything else are typed startup errors (fail closed, listing
//! the valid packs) — never a silent fall back to full.

use std::fmt;

/// The standing operator surface, served by every profile: record
/// listing/status plus the tier-checked delivery export (the
/// registry spine an operator needs on every launch).
pub const CORE_TOOLS: [&str; 2] = ["neemu_list_records", "neemu_export"];

/// Pack table: pack name → the tool names it adds beyond core.
/// Alphabetical by pack name; tool lists follow catalog order.
/// Measured per-tool costs (cl100k over the served tool object,
/// census of record 2026-11-04): acts 2,570; image 1,445;
/// intake 938; voice 2,343; core itself 539.
pub const PACKS: [(&str, &[&str]); 4] = [
    (
        "acts",
        &[
            "neemu_enrich",
            "neemu_extract_acts",
            "neemu_understand_scene",
        ],
    ),
    (
        "image",
        &[
            "neemu_segment_scene",
            "neemu_extract_faces",
            "neemu_extract_dialogue",
        ],
    ),
    (
        "intake",
        &[
            "neemu_register_work",
            "neemu_validate_ingest",
            "neemu_screen_work",
        ],
    ),
    (
        "voice",
        &[
            "neemu_analyze_voice",
            "neemu_assess_voice_clonability",
            "neemu_derive_voice",
            "neemu_emit_torori_cue",
        ],
    ),
];

/// The valid selector names, for fail-closed error messages.
pub const VALID_SELECTORS: &str = "core, full, acts, image, intake, voice";

/// A typed profile error: startup fails closed with this text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileError {
    message: String,
}

impl ProfileError {
    fn new(message: String) -> Self {
        ProfileError { message }
    }
}

impl fmt::Display for ProfileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ProfileError {}

fn invalid(message: String) -> ProfileError {
    ProfileError::new(format!("invalid tool profile: {message}"))
}

/// A resolved tool profile: the full surface, or core plus a set
/// of packs (stored as pack names in canonical table order).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolProfile {
    Full,
    Scoped(Vec<&'static str>),
}

impl Default for ToolProfile {
    fn default() -> Self {
        ToolProfile::core()
    }
}

impl ToolProfile {
    /// The default profile: core only.
    pub fn core() -> Self {
        ToolProfile::Scoped(Vec::new())
    }

    /// Parse a profile spec: `core`, `full`, or a comma-separated
    /// pack list (a `core` segment is accepted and is a no-op —
    /// core is always served). Exact, case-sensitive matching;
    /// anything else fails closed.
    pub fn parse(spec: &str) -> Result<Self, ProfileError> {
        let spec = spec.trim();
        if spec.is_empty() {
            return Err(invalid(format!(
                "profile spec is empty (valid: {VALID_SELECTORS})"
            )));
        }
        let mut packs: Vec<&'static str> = Vec::new();
        let mut seen: Vec<&str> = Vec::new();
        let mut full = false;
        for segment in spec.split(',') {
            let name = segment.trim();
            if name.is_empty() {
                return Err(invalid(format!(
                    "empty segment in profile spec '{spec}' (valid: {VALID_SELECTORS})"
                )));
            }
            if seen.contains(&name) {
                return Err(invalid(format!(
                    "duplicate segment '{name}' in profile spec '{spec}'"
                )));
            }
            seen.push(name);
            match name {
                "core" => {}
                "full" => full = true,
                other => match PACKS.iter().find(|(pack, _)| *pack == other) {
                    Some((pack, _)) => packs.push(pack),
                    None => {
                        return Err(invalid(format!(
                            "unknown pack '{other}' (valid: {VALID_SELECTORS})"
                        )));
                    }
                },
            }
        }
        if full {
            if seen.len() > 1 {
                return Err(invalid(
                    "'full' must be the only segment in a profile spec".to_string(),
                ));
            }
            return Ok(ToolProfile::Full);
        }
        // Canonical order is the pack table's order, not input order.
        packs.sort_by_key(|pack| {
            PACKS
                .iter()
                .position(|(name, _)| name == pack)
                .unwrap_or(usize::MAX)
        });
        Ok(ToolProfile::Scoped(packs))
    }

    /// Resolve the launch profile by precedence: explicit argument
    /// → environment value → config-file value → default `core`.
    /// An empty environment value counts as unset (the same
    /// convention `NEEMU_HOME` follows in `main`); an empty
    /// argument or config value fails closed in [`Self::parse`].
    pub fn resolve(
        argument: Option<&str>,
        environment: Option<&str>,
        config: Option<&str>,
    ) -> Result<Self, ProfileError> {
        if let Some(spec) = argument {
            return Self::parse(spec);
        }
        if let Some(spec) = environment {
            if !spec.trim().is_empty() {
                return Self::parse(spec);
            }
        }
        if let Some(spec) = config {
            return Self::parse(spec);
        }
        Ok(ToolProfile::core())
    }

    /// Whether this profile serves the named tool. Core tools are
    /// served by every profile; `full` serves everything.
    pub fn serves_tool(&self, name: &str) -> bool {
        match self {
            ToolProfile::Full => true,
            ToolProfile::Scoped(packs) => {
                CORE_TOOLS.contains(&name)
                    || packs.iter().any(|pack| {
                        PACKS
                            .iter()
                            .find(|(pack_name, _)| pack_name == pack)
                            .is_some_and(|(_, tools)| tools.contains(&name))
                    })
            }
        }
    }

    /// The canonical spec string, as reported in-band and in logs:
    /// `full`, `core`, or `core,<pack>,...` in table order.
    pub fn spec_string(&self) -> String {
        match self {
            ToolProfile::Full => "full".to_string(),
            ToolProfile::Scoped(packs) => {
                if packs.is_empty() {
                    "core".to_string()
                } else {
                    format!("core,{}", packs.join(","))
                }
            }
        }
    }
}

/// Extract the `tool_profile` value from `neemu.toml` contents.
/// Strict by contract: blank lines and `#` comments are skipped;
/// the one recognised key must appear at most once, as
/// `tool_profile = "<spec>"` with nothing after the closing
/// quote. Any other key is ignored (the file may grow other
/// settings later); a malformed `tool_profile` line fails closed.
pub fn config_tool_profile(contents: &str) -> Result<Option<String>, ProfileError> {
    let mut found: Option<String> = None;
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some(rest) = line.strip_prefix("tool_profile") else {
            continue;
        };
        let rest = rest.trim_start();
        let Some(rest) = rest.strip_prefix('=') else {
            return Err(invalid(format!(
                "malformed tool_profile line in neemu.toml: '{line}'"
            )));
        };
        let rest = rest.trim();
        let Some(value) = rest
            .strip_prefix('"')
            .and_then(|inner| inner.strip_suffix('"'))
        else {
            return Err(invalid(format!(
                "tool_profile in neemu.toml must be a quoted string: '{line}'"
            )));
        };
        if value.contains('"') {
            return Err(invalid(format!(
                "malformed tool_profile line in neemu.toml: '{line}'"
            )));
        }
        if found.is_some() {
            return Err(invalid(
                "duplicate tool_profile key in neemu.toml".to_string(),
            ));
        }
        found = Some(value.to_string());
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::TOOL_NAMES;

    // ---- validation ----

    #[test]
    fn core_is_the_default_and_serves_only_the_spine() {
        let profile = ToolProfile::default();
        assert_eq!(profile.spec_string(), "core");
        for name in TOOL_NAMES {
            assert_eq!(
                profile.serves_tool(name),
                CORE_TOOLS.contains(&name),
                "{name}"
            );
        }
    }

    #[test]
    fn full_serves_everything() {
        let profile = ToolProfile::parse("full").expect("full parses");
        assert_eq!(profile.spec_string(), "full");
        for name in TOOL_NAMES {
            assert!(profile.serves_tool(name), "{name}");
        }
    }

    #[test]
    fn packs_combine_and_canonicalise_order() {
        let profile = ToolProfile::parse("voice,intake").expect("packs parse");
        assert_eq!(profile.spec_string(), "core,intake,voice");
        assert!(profile.serves_tool("neemu_register_work"));
        assert!(profile.serves_tool("neemu_derive_voice"));
        assert!(profile.serves_tool("neemu_list_records"));
        assert!(!profile.serves_tool("neemu_extract_faces"));
    }

    #[test]
    fn core_segment_is_an_accepted_no_op() {
        assert_eq!(
            ToolProfile::parse("core,voice").expect("parses"),
            ToolProfile::parse("voice").expect("parses")
        );
    }

    #[test]
    fn resolve_precedence_is_argument_then_env_then_config() {
        let argument =
            ToolProfile::resolve(Some("image"), Some("voice"), Some("acts")).expect("resolves");
        assert_eq!(argument.spec_string(), "core,image");
        let env = ToolProfile::resolve(None, Some("voice"), Some("acts")).expect("resolves");
        assert_eq!(env.spec_string(), "core,voice");
        let config = ToolProfile::resolve(None, None, Some("acts")).expect("resolves");
        assert_eq!(config.spec_string(), "core,acts");
        let default = ToolProfile::resolve(None, None, None).expect("resolves");
        assert_eq!(default.spec_string(), "core");
        // An empty env value is unset, so config wins under it.
        let under_empty_env =
            ToolProfile::resolve(None, Some("  "), Some("acts")).expect("resolves");
        assert_eq!(under_empty_env.spec_string(), "core,acts");
    }

    #[test]
    fn config_extracts_a_quoted_tool_profile() {
        let contents = "# neemu config\nother_key = 1\ntool_profile = \"voice\"\n";
        assert_eq!(
            config_tool_profile(contents).expect("extracts"),
            Some("voice".to_string())
        );
        assert_eq!(
            config_tool_profile("other = true\n").expect("extracts"),
            None
        );
    }

    #[test]
    fn coverage_law_every_tool_has_exactly_one_home() {
        for name in TOOL_NAMES {
            let in_core = CORE_TOOLS.contains(&name);
            let homes: Vec<&str> = PACKS
                .iter()
                .filter(|(_, tools)| tools.contains(&name))
                .map(|(pack, _)| *pack)
                .collect();
            assert_eq!(
                usize::from(in_core) + homes.len(),
                1,
                "{name}: core={in_core}, packs={homes:?}"
            );
        }
        // And no pack names a tool the catalog does not serve.
        for (pack, tools) in PACKS {
            for tool in tools {
                assert!(TOOL_NAMES.contains(tool), "{pack}: {tool}");
            }
        }
    }

    #[test]
    fn pack_table_is_alphabetical_and_disjoint() {
        let names: Vec<&str> = PACKS.iter().map(|(name, _)| *name).collect();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        assert_eq!(names, sorted);
        assert!(!names.contains(&"core") && !names.contains(&"full"));
    }

    // ---- adversarial ----

    #[test]
    fn unknown_pack_fails_closed_and_lists_valid_packs() {
        let error = ToolProfile::parse("ghidraa").expect_err("must fail");
        assert_eq!(
            error.to_string(),
            "invalid tool profile: unknown pack 'ghidraa' (valid: core, full, acts, image, intake, voice)"
        );
    }

    #[test]
    fn malformed_specs_fail_closed() {
        for spec in ["", " ", ",", "intake,,voice", "intake,", ",voice"] {
            assert!(ToolProfile::parse(spec).is_err(), "{spec:?}");
        }
    }

    #[test]
    fn matching_is_case_exact_and_inner_space_is_not_a_name() {
        for spec in ["Voice", "FULL", "Core", "in take", "core, core"] {
            assert!(ToolProfile::parse(spec).is_err(), "{spec:?}");
        }
    }

    #[test]
    fn full_combined_with_anything_fails_closed() {
        for spec in ["full,voice", "voice,full", "full,core", "full,full"] {
            assert!(ToolProfile::parse(spec).is_err(), "{spec:?}");
        }
    }

    #[test]
    fn duplicate_pack_fails_closed() {
        assert!(ToolProfile::parse("voice,voice").is_err());
    }

    #[test]
    fn resolve_propagates_errors_from_the_winning_source() {
        assert!(ToolProfile::resolve(Some("nope"), Some("voice"), None).is_err());
        assert!(ToolProfile::resolve(None, Some("nope"), Some("voice")).is_err());
        assert!(ToolProfile::resolve(None, None, Some("nope")).is_err());
    }

    #[test]
    fn config_malformed_lines_fail_closed() {
        for contents in [
            "tool_profile = voice\n",
            "tool_profile \"voice\"\n",
            "tool_profile = \"voice\" trailing\n",
            "tool_profile = \"a\"\ntool_profile = \"b\"\n",
        ] {
            assert!(config_tool_profile(contents).is_err(), "{contents:?}");
        }
    }
}
