//! Core workflow support tests (Wave 0 stubs)
//! These tests verify agtx-core functions used by the workflow engine
//! for FLOW-02 (plugin resolution), FLOW-03 (skill deployment), FLOW-04 (command resolution).

// FLOW-02: Plugin resolution precedence
#[test]
fn plugin_resolution_loads_bundled_plugin() {
    // load_bundled_plugin("agtx") should return Some(WorkflowPlugin)
    // with expected fields populated.
    let plugin = agtx_core::skills::load_bundled_plugin("agtx");
    assert!(plugin.is_some(), "bundled 'agtx' plugin should exist");
}

#[test]
fn plugin_resolution_returns_none_for_unknown() {
    let plugin = agtx_core::skills::load_bundled_plugin("nonexistent-plugin-xyz");
    assert!(plugin.is_none(), "unknown plugin should return None");
}

// FLOW-03: Skill deployment
#[test]
fn skill_deployment_agent_native_dirs_exist_for_all_agents() {
    // Each supported agent should have a native skill directory mapping.
    for agent in &["claude", "codex", "gemini", "opencode", "copilot"] {
        let dir = agtx_core::skills::agent_native_skill_dir(agent);
        assert!(
            dir.is_some(),
            "agent '{agent}' should have native skill dir"
        );
    }
}

// FLOW-04: Command translation per agent
#[test]
fn command_translation_preserves_claude_format() {
    // Claude commands should pass through unchanged.
    let result = agtx_core::skills::transform_plugin_command("/agtx:plan", "claude");
    assert_eq!(result, Some("/agtx:plan".to_string()));
}

#[test]
fn command_translation_converts_for_codex() {
    // Codex transforms /ns:command to $ns-command.
    let result = agtx_core::skills::transform_plugin_command("/agtx:plan", "codex");
    assert_eq!(result, Some("$agtx-plan".to_string()));
}

#[test]
fn command_translation_converts_for_opencode() {
    // OpenCode transforms /ns:command to /ns-command.
    let result = agtx_core::skills::transform_plugin_command("/agtx:plan", "opencode");
    assert_eq!(result, Some("/agtx-plan".to_string()));
}
