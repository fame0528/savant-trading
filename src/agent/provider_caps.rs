//! Provider Capabilities (FID-085 Phase 4, Item 16)
//!
//! Declarative per-model configuration for LLM provider quirks.
//! Supports DeepSeek (no tool_choice), MiniMax (reasoning_split), etc.

use std::collections::HashMap;

/// Capability flags for an LLM model.
#[derive(Debug, Clone)]
pub struct ModelCapabilities {
    /// Model identifier (e.g., "openai/gpt-4o", "deepseek/deepseek-chat")
    pub model_id: String,
    /// Whether the model supports tool_choice parameter
    pub supports_tool_choice: bool,
    /// Whether reasoning content needs special handling (e.g., MiniMax reasoning_split)
    pub requires_reasoning_roundtrip: bool,
    /// How to request structured output: "json_mode", "tool_choice", "native"
    pub structured_method: String,
    /// Whether the model supports cache_control ephemeral markers
    pub supports_cache_control: bool,
    /// Known model quirks as human-readable notes
    pub notes: Vec<String>,
}

impl ModelCapabilities {
    /// Get capabilities for a known model. Falls back to defaults for unknown models.
    pub fn for_model(model_id: &str) -> Self {
        // Check known models
        if let Some(caps) = Self::known_models().get(model_id) {
            return caps.clone();
        }
        // Default: assume modern OpenAI-compatible capabilities
        Self::default_for(model_id)
    }

    /// Build a registry of known model capabilities.
    fn known_models() -> HashMap<String, ModelCapabilities> {
        let mut m = HashMap::new();

        // DeepSeek: no tool_choice, reasoning roundtrip
        m.insert(
            "deepseek/deepseek-chat".to_string(),
            ModelCapabilities {
                model_id: "deepseek/deepseek-chat".to_string(),
                supports_tool_choice: false,
                requires_reasoning_roundtrip: true,
                structured_method: "json_mode".to_string(),
                supports_cache_control: false,
                notes: vec![
                    "No tool_choice support".to_string(),
                    "May wrap responses in <think> tags".to_string(),
                ],
            },
        );

        // MiniMax M1: reasoning split
        m.insert(
            "minimax/minimax-m1-80k".to_string(),
            ModelCapabilities {
                model_id: "minimax/minimax-m1-80k".to_string(),
                supports_tool_choice: true,
                requires_reasoning_roundtrip: true,
                structured_method: "json_mode".to_string(),
                supports_cache_control: false,
                notes: vec!["Reasoning split: content and reasoning are in separate fields".to_string()],
            },
        );

        // MiniMax M3: reasoning split (same as M1, 1M context, 512K output)
        m.insert(
            "MiniMax-M3".to_string(),
            ModelCapabilities {
                model_id: "MiniMax-M3".to_string(),
                supports_tool_choice: true,
                requires_reasoning_roundtrip: true,
                structured_method: "json_mode".to_string(),
                supports_cache_control: false,
                notes: vec![
                    "Reasoning split: content and reasoning are in separate fields".to_string(),
                    "1.05M context window, 512K max output".to_string(),
                ],
            },
        );

        // Anthropic: cache_control supported
        m.insert(
            "anthropic/claude-sonnet-4-20250514".to_string(),
            ModelCapabilities {
                model_id: "anthropic/claude-sonnet-4-20250514".to_string(),
                supports_tool_choice: true,
                requires_reasoning_roundtrip: false,
                structured_method: "tool_choice".to_string(),
                supports_cache_control: true,
                notes: vec!["Cache control via cache_control ephemeral markers".to_string()],
            },
        );

        // Google Gemini
        m.insert(
            "google/gemini-2.5-pro".to_string(),
            ModelCapabilities {
                model_id: "google/gemini-2.5-pro".to_string(),
                supports_tool_choice: true,
                requires_reasoning_roundtrip: false,
                structured_method: "json_mode".to_string(),
                supports_cache_control: true,
                notes: vec!["Supports context caching for long prompts".to_string()],
            },
        );

        m
    }

    fn default_for(model_id: &str) -> Self {
        Self {
            model_id: model_id.to_string(),
            supports_tool_choice: true,
            requires_reasoning_roundtrip: false,
            structured_method: "json_mode".to_string(),
            supports_cache_control: true,
            notes: vec!["Unknown model — using default OpenAI-compatible capabilities".to_string()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deepseek_caps() {
        let caps = ModelCapabilities::for_model("deepseek/deepseek-chat");
        assert!(!caps.supports_tool_choice);
        assert!(caps.requires_reasoning_roundtrip);
    }

    #[test]
    fn unknown_model_defaults() {
        let caps = ModelCapabilities::for_model("some-unknown/model");
        assert!(caps.supports_tool_choice);
        assert!(caps.supports_cache_control);
    }

    #[test]
    fn anthropic_cache_control() {
        let caps = ModelCapabilities::for_model("anthropic/claude-sonnet-4-20250514");
        assert!(caps.supports_cache_control);
        assert_eq!(caps.structured_method, "tool_choice");
    }
}
