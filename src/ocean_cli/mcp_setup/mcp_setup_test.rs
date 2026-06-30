use crate::ocean_cli::mcp_setup::agents::{AgentConfig, AgentType, ConfigFormat};
use crate::ocean_cli::mcp_setup::detect;
use crate::ocean_cli::mcp_setup::write;

#[test]
fn test_agent_type_from_str() {
    assert_eq!(AgentType::from_str("claude-desktop"), Some(AgentType::ClaudeDesktop));
    assert_eq!(AgentType::from_str("claude-code"), Some(AgentType::ClaudeCode));
    assert_eq!(AgentType::from_str("codex"), Some(AgentType::Codex));
    assert_eq!(AgentType::from_str("opencode"), Some(AgentType::OpenCode));
    assert_eq!(AgentType::from_str("antigravity"), Some(AgentType::Antigravity));
    assert_eq!(AgentType::from_str("openclaw"), Some(AgentType::OpenClaw));
    assert_eq!(AgentType::from_str("unknown"), None);
}

#[test]
fn test_agent_type_name() {
    assert_eq!(AgentType::ClaudeDesktop.name(), "claude-desktop");
    assert_eq!(AgentType::Codex.name(), "codex");
}

#[test]
fn test_agent_type_display_name() {
    assert_eq!(AgentType::ClaudeDesktop.display_name(), "Claude Desktop");
    assert_eq!(AgentType::OpenCode.display_name(), "OpenCode");
}

#[test]
fn test_agent_type_all() {
    let all = AgentType::all();
    assert_eq!(all.len(), 6);
    assert!(all.contains(&AgentType::ClaudeDesktop));
    assert!(all.contains(&AgentType::OpenClaw));
}

#[test]
fn test_agent_type_known_names() {
    let names = AgentType::known_names();
    assert!(names.contains(&"claude-desktop"));
    assert!(names.contains(&"openclaw"));
}

#[test]
fn test_agent_type_display() {
    assert_eq!(format!("{}", AgentType::ClaudeDesktop), "claude-desktop");
}

#[test]
fn test_agent_config_creation() {
    let cfg = AgentConfig {
        agent_type: AgentType::ClaudeDesktop,
        config_path: None,
        installed: true,
        config_format: ConfigFormat::Json,
        config_key_description: "mcpServers",
    };
    assert_eq!(cfg.agent_type.name(), "claude-desktop");
    assert!(cfg.installed);
}

#[test]
fn test_detect_no_agents_found() {
    let configs = detect::detect_all();
    assert_eq!(configs.len(), 6);
    for cfg in &configs {
        assert!(cfg.installed);
    }
}

#[test]
fn test_generate_config_block_json_mcp_servers() {
    let cfg = AgentConfig {
        agent_type: AgentType::ClaudeDesktop,
        config_path: None,
        installed: true,
        config_format: ConfigFormat::Json,
        config_key_description: "mcpServers",
    };
    let block = write::generate_config_block(&cfg, "mcp");
    assert!(block.contains("\"command\": \"mcp\""));
    assert!(block.contains("\"args\": []"));
}

#[test]
fn test_generate_config_block_opencode() {
    let cfg = AgentConfig {
        agent_type: AgentType::OpenCode,
        config_path: None,
        installed: true,
        config_format: ConfigFormat::Json,
        config_key_description: "mcp",
    };
    let block = write::generate_config_block(&cfg, "mcp");
    assert!(block.contains("\"type\": \"local\""));
    assert!(block.contains("\"command\""));
    assert!(block.contains("\"mcp\""));
    assert!(block.contains("\"enabled\": true"));
}

#[test]
fn test_generate_config_block_codex() {
    let cfg = AgentConfig {
        agent_type: AgentType::Codex,
        config_path: None,
        installed: true,
        config_format: ConfigFormat::Toml,
        config_key_description: "[mcp_servers.ocean]",
    };
    let block = write::generate_config_block(&cfg, "mcp");
    assert!(block.contains("[mcp_servers.ocean]"));
    assert!(block.contains("command = \"mcp\""));
}

#[test]
fn test_generate_config_block_antigravity() {
    let cfg = AgentConfig {
        agent_type: AgentType::Antigravity,
        config_path: None,
        installed: true,
        config_format: ConfigFormat::Json,
        config_key_description: "mcpServers",
    };
    let block = write::generate_config_block(&cfg, "mcp");
    assert!(block.contains("\"command\": \"mcp\""));
}

#[test]
fn test_generate_config_block_openclaw() {
    let cfg = AgentConfig {
        agent_type: AgentType::OpenClaw,
        config_path: None,
        installed: true,
        config_format: ConfigFormat::Json,
        config_key_description: "mcp.servers",
    };
    let block = write::generate_config_block(&cfg, "mcp");
    assert!(block.contains("\"command\": \"mcp\""));
}

#[test]
fn test_merge_json_mcp_servers_empty() {
    let (result, already) = write::merge_json_mcp_servers("", "mcp").unwrap();
    assert!(!already);
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["mcpServers"]["ocean"]["command"], "mcp");
}

#[test]
fn test_merge_json_mcp_servers_existing() {
    let existing = r#"{"mcpServers": {"other": {"command": "foo"}}}"#;
    let (result, already) = write::merge_json_mcp_servers(existing, "mcp").unwrap();
    assert!(!already);
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["mcpServers"]["ocean"]["command"], "mcp");
    assert_eq!(parsed["mcpServers"]["other"]["command"], "foo");
}

#[test]
fn test_merge_json_mcp_servers_already_configured() {
    let existing = r#"{"mcpServers": {"ocean": {"command": "mcp"}}}"#;
    let (result, already) = write::merge_json_mcp_servers(existing, "mcp").unwrap();
    assert!(already);
    assert_eq!(result, existing);
}

#[test]
fn test_merge_opencode_json_empty() {
    let (result, already) = write::merge_opencode_json("", "mcp").unwrap();
    assert!(!already);
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["mcp"]["ocean"]["type"], "local");
    assert_eq!(parsed["mcp"]["ocean"]["command"][0], "mcp");
    assert_eq!(parsed["mcp"]["ocean"]["enabled"], true);
}

#[test]
fn test_merge_opencode_json_already_configured() {
    let existing = r#"{"mcp": {"ocean": {"type": "local", "command": ["mcp"], "enabled": true}}}"#;
    let (result, already) = write::merge_opencode_json(existing, "mcp").unwrap();
    assert!(already);
    assert_eq!(result, existing);
}

#[test]
fn test_merge_opencode_json_preserves_other() {
    let existing = r#"{"mcp": {"other-agent": {"command": "foo"}}}"#;
    let (result, already) = write::merge_opencode_json(existing, "mcp").unwrap();
    assert!(!already);
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed["mcp"].as_object().unwrap().contains_key("ocean"));
    assert!(parsed["mcp"].as_object().unwrap().contains_key("other-agent"));
}

#[test]
fn test_merge_openclaw_json_empty() {
    let (result, already) = write::merge_openclaw_json("", "mcp").unwrap();
    assert!(!already);
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["mcp"]["servers"]["ocean"]["command"], "mcp");
}

#[test]
fn test_merge_openclaw_json_existing_mcp_structure() {
    let existing = r#"{"mcp": {"servers": {"other": {"command": "foo"}}}}"#;
    let (result, already) = write::merge_openclaw_json(existing, "mcp").unwrap();
    assert!(!already);
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["mcp"]["servers"]["ocean"]["command"], "mcp");
    assert_eq!(parsed["mcp"]["servers"]["other"]["command"], "foo");
}

#[test]
fn test_merge_toml_mcp_servers_empty() {
    let (result, already) = write::merge_toml_mcp_servers("", "mcp").unwrap();
    assert!(!already);
    assert!(result.contains("[mcp_servers.ocean]"));
    assert!(result.contains("command = \"mcp\""));
}

#[test]
fn test_merge_toml_mcp_servers_already_configured() {
    let existing = r#"[mcp_servers.ocean]
command = "mcp"
args = []
"#;
    let already = write::merge_toml_mcp_servers(existing, "mcp").unwrap().1;
    assert!(already);
}

#[test]
fn test_merge_toml_mcp_servers_preserves_other() {
    let existing = r#"[mcp_servers.other]
command = "foo"
"#;
    let (result, already) = write::merge_toml_mcp_servers(existing, "mcp").unwrap();
    assert!(!already);
    let parsed: toml::Value = result.parse().unwrap();
    assert!(parsed["mcp_servers"].as_table().unwrap().contains_key("ocean"));
    assert!(parsed["mcp_servers"].as_table().unwrap().contains_key("other"));
}

#[test]
fn test_generate_config_block_display() {
    let cfg = AgentConfig {
        agent_type: AgentType::ClaudeDesktop,
        config_path: Some(std::path::PathBuf::from("test.json")),
        installed: true,
        config_format: ConfigFormat::Json,
        config_key_description: "mcpServers",
    };
    let display = write::generate_config_block_display(&cfg, "mcp");
    assert!(display.contains("Claude Desktop"));
    assert!(display.contains("test.json"));
    assert!(display.contains("\"command\": \"mcp\""));
}

#[test]
fn test_merge_json_mcp_servers_malformed() {
    let result = write::merge_json_mcp_servers("not-json", "mcp");
    assert!(result.is_err());
}

#[test]
fn test_merge_toml_mcp_servers_malformed() {
    let result = write::merge_toml_mcp_servers("not-toml{{{", "mcp");
    assert!(result.is_err());
}

#[test]
fn test_resolve_mcp_binary_not_found() {
    let result = crate::ocean_cli::mcp_setup::cmd_mcp_setup(Some("unknown-agent".to_string()), false);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("Unknown agent"));
}
