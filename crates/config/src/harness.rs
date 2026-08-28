//! Harness posture + profile config types (#3311).
//!
//! A *harness posture* is the agent-shaping policy (sub-agent cap, tool
//! surface, compaction/cache strategy, safety stance); a *harness profile*
//! binds a posture to a provider route + model pattern. Extracted verbatim
//! from lib.rs to separate this agent-posture domain from the rest of the
//! config schema; re-exported at the crate root so existing paths are
//! unchanged. Behavior is identical.

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::ProviderKind;

/// Kinds of built-in harness postures.
///
/// A posture names the runtime strategy GhostyCode should use for a
/// provider/model route: how much context to preload, how aggressively to lean
/// on sub-agents, and how to balance prompt-cache stability against quick
/// exploration. Runtime selection is wired in later v0.9 slices; this config
/// model intentionally keeps the policy data explicit first.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum HarnessPostureKind {
    /// Full-featured default: rich constitution, broad tool catalog, and normal
    /// sub-agent posture.
    #[default]
    Standard,
    /// Cache-heavy: deeper prompt layering and prefix-cache-oriented context.
    CacheHeavy,
    /// Lean: smaller starting context, faster compaction, and stronger
    /// exploration/delegation bias.
    Lean,
    /// User-defined posture assembled from explicit knobs below.
    Custom,
}

/// How this posture should approach compaction and prompt-cache stability.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum HarnessCompactionStrategy {
    #[default]
    Default,
    PrefixCache,
    Aggressive,
}

/// Which tool catalog shape this posture prefers.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum HarnessToolSurface {
    #[default]
    Full,
    ReadOnly,
    Auto,
}

/// Safety posture applied when the runtime consumes a harness profile.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum HarnessSafetyPosture {
    #[default]
    Standard,
    Strict,
    Permissive,
}

/// A concrete harness posture with policy knobs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct HarnessPosture {
    /// Named posture kind.
    #[serde(default)]
    pub kind: HarnessPostureKind,
    /// Maximum number of concurrent sub-agents (0 = runtime default).
    #[serde(default)]
    pub max_subagents: usize,
    /// Prefer search-based/on-demand context over always-on documentation.
    #[serde(default)]
    pub prefer_codebase_search: bool,
    /// Compaction and prompt-cache strategy.
    #[serde(default)]
    pub compaction_strategy: HarnessCompactionStrategy,
    /// Preferred tool catalog shape.
    #[serde(default)]
    pub tool_surface: HarnessToolSurface,
    /// Safety posture for runtime consumers.
    #[serde(default)]
    pub safety_posture: HarnessSafetyPosture,
}

impl Default for HarnessPosture {
    fn default() -> Self {
        Self {
            kind: HarnessPostureKind::Standard,
            max_subagents: 0,
            prefer_codebase_search: false,
            compaction_strategy: HarnessCompactionStrategy::default(),
            tool_surface: HarnessToolSurface::default(),
            safety_posture: HarnessSafetyPosture::default(),
        }
    }
}

impl HarnessPosture {
    /// A cache-heavy posture tuned for DeepSeek V4 / MiMo-style models.
    #[must_use]
    pub fn cache_heavy() -> Self {
        Self {
            kind: HarnessPostureKind::CacheHeavy,
            max_subagents: 10,
            prefer_codebase_search: false,
            compaction_strategy: HarnessCompactionStrategy::PrefixCache,
            tool_surface: HarnessToolSurface::Full,
            safety_posture: HarnessSafetyPosture::Standard,
        }
    }

    /// A lean posture for smaller-context or weaker tool-use models.
    #[must_use]
    pub fn lean() -> Self {
        Self {
            kind: HarnessPostureKind::Lean,
            max_subagents: 20,
            prefer_codebase_search: true,
            compaction_strategy: HarnessCompactionStrategy::Aggressive,
            tool_surface: HarnessToolSurface::Full,
            safety_posture: HarnessSafetyPosture::Standard,
        }
    }
}

/// A harness profile binds a posture to a provider route and model pattern.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct HarnessProfile {
    /// Provider route this profile applies to, e.g. "deepseek" or
    /// "xiaomi-mimo".
    pub provider_route: String,
    /// Regex or glob pattern for model names, e.g. "deepseek-v4.*".
    pub model_pattern: String,
    /// The posture to apply.
    #[serde(default)]
    pub posture: HarnessPosture,
}

impl HarnessProfile {
    /// Return true when this profile applies to the provider/model route.
    ///
    /// This is a pure config helper: matching a profile must not mutate runtime
    /// provider selection, prompts, auth, tools, context, or persisted config.
    #[must_use]
    pub fn matches_route(&self, provider_route: &str, model: &str) -> bool {
        provider_routes_equal(&self.provider_route, provider_route)
            && wildcard_pattern_matches(&self.model_pattern, model)
    }
}

/// Built-in profile seeds for common provider/model families.
///
/// User-configured profiles are always checked first; these seeds only provide
/// a stable resolver result when config has no narrower match.
#[must_use]
pub fn built_in_harness_profiles() -> &'static [HarnessProfile] {
    static PROFILES: OnceLock<Vec<HarnessProfile>> = OnceLock::new();
    PROFILES.get_or_init(|| {
        vec![
            HarnessProfile {
                provider_route: "deepseek".to_string(),
                model_pattern: "deepseek-v4*".to_string(),
                posture: HarnessPosture::cache_heavy(),
            },
            HarnessProfile {
                provider_route: "xiaomi-mimo".to_string(),
                model_pattern: "mimo-v2.5*".to_string(),
                posture: HarnessPosture::cache_heavy(),
            },
            HarnessProfile {
                provider_route: "arcee".to_string(),
                model_pattern: "trinity-large-thinking".to_string(),
                posture: HarnessPosture::cache_heavy(),
            },
            HarnessProfile {
                provider_route: "huggingface".to_string(),
                model_pattern: "*".to_string(),
                posture: HarnessPosture::lean(),
            },
            HarnessProfile {
                provider_route: "sglang".to_string(),
                model_pattern: "*".to_string(),
                posture: HarnessPosture::lean(),
            },
            HarnessProfile {
                provider_route: "vllm".to_string(),
                model_pattern: "*".to_string(),
                posture: HarnessPosture::lean(),
            },
            HarnessProfile {
                provider_route: "ollama".to_string(),
                model_pattern: "*".to_string(),
                posture: HarnessPosture::lean(),
            },
        ]
    })
}

fn provider_routes_equal(expected: &str, actual: &str) -> bool {
    match (ProviderKind::parse(expected), ProviderKind::parse(actual)) {
        (Some(expected), Some(actual)) => expected == actual,
        _ => expected.trim().eq_ignore_ascii_case(actual.trim()),
    }
}

fn wildcard_pattern_matches(pattern: &str, value: &str) -> bool {
    wildcard_chars_match(
        &pattern.chars().collect::<Vec<_>>(),
        &value.chars().collect::<Vec<_>>(),
    )
}

fn wildcard_chars_match(pattern: &[char], value: &[char]) -> bool {
    let (mut pattern_idx, mut value_idx) = (0, 0);
    let mut star_idx: Option<usize> = None;
    let mut star_value_idx = 0;

    while value_idx < value.len() {
        if pattern_idx < pattern.len()
            && (pattern[pattern_idx] == '?' || pattern[pattern_idx] == value[value_idx])
        {
            pattern_idx += 1;
            value_idx += 1;
        } else if pattern_idx < pattern.len() && pattern[pattern_idx] == '*' {
            star_idx = Some(pattern_idx);
            pattern_idx += 1;
            star_value_idx = value_idx;
        } else if let Some(star) = star_idx {
            pattern_idx = star + 1;
            star_value_idx += 1;
            value_idx = star_value_idx;
        } else {
            return false;
        }
    }

    pattern[pattern_idx..].iter().all(|ch| *ch == '*')
}
