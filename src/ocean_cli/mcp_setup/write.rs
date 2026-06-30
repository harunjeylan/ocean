use serde_json::{Map, Value};

use super::agents::{AgentConfig, AgentType};

pub fn generate_config_block(agent: &AgentConfig, binary: &str) -> String {
    match agent.agent_type {
        AgentType::ClaudeDesktop | AgentType::ClaudeCode | AgentType::Antigravity => {
            format_json_mcp_servers(binary)
        }
        AgentType::Codex => format_toml_mcp_servers(binary),
        AgentType::OpenCode => format_opencode_json(binary),
        AgentType::OpenClaw => format_openclaw_json(binary),
    }
}

pub fn generate_config_block_display(agent: &AgentConfig, binary: &str) -> String {
    let block = generate_config_block(agent, binary);
    let header = format!("=== {} ===", agent.agent_type.display_name());
    let path_info = match &agent.config_path {
        Some(p) => format!("Config file: {}", p.display()),
        None => format!("Config file: (not detected — create manually)"),
    };
    let key_desc = agent.config_key_description;
    format!(
        "{}\n{}\nAdd this block to \"{}\":\n{}",
        header, path_info, key_desc, block
    )
}

pub fn write_agent_config(agent: &AgentConfig, binary: &str) -> Result<(), String> {
    let path = match &agent.config_path {
        Some(p) => p.clone(),
        None => {
            let default_path = default_config_path(agent.agent_type);
            eprintln!("Config file not found for {}. Will create: {}", agent.agent_type.display_name(), default_path.display());
            default_path
        }
    };

    let content = std::fs::read_to_string(&path).unwrap_or_default();

    let (new_content, already_exists) = match agent.agent_type {
        AgentType::ClaudeDesktop | AgentType::ClaudeCode | AgentType::Antigravity => {
            merge_json_mcp_servers(&content, binary)?
        }
        AgentType::Codex => {
            merge_toml_mcp_servers(&content, binary)?
        }
        AgentType::OpenCode => {
            merge_opencode_json(&content, binary)?
        }
        AgentType::OpenClaw => {
            merge_openclaw_json(&content, binary)?
        }
    };

    if already_exists {
        eprintln!("ocean MCP server already configured for {}.", agent.agent_type.display_name());
        return Ok(());
    }

    if path.exists() {
        let backup_path = path.with_extension("json.bak");
        std::fs::copy(&path, &backup_path)
            .map_err(|e| format!("Failed to create backup at {}: {}", backup_path.display(), e))?;
        eprintln!("Backup created: {}", backup_path.display());
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory {}: {}", parent.display(), e))?;
    }

    std::fs::write(&path, &new_content)
        .map_err(|e| format!("Failed to write {}: {}", path.display(), e))?;

    eprintln!("Config written to {}", path.display());
    Ok(())
}

fn default_config_path(agent_type: AgentType) -> std::path::PathBuf {
    match agent_type {
        AgentType::ClaudeDesktop => {
            let appdata = std::env::var("APPDATA").unwrap_or_else(|_| {
                home_dir().map(|h| h.join("AppData").join("Roaming").to_string_lossy().to_string())
                    .unwrap_or_else(|| "C:\\Users\\Default\\AppData\\Roaming".to_string())
            });
            std::path::PathBuf::from(appdata).join("Claude").join("claude_desktop_config.json")
        }
        AgentType::ClaudeCode => {
            home_dir().map(|h| h.join(".claude.json")).unwrap_or_else(|| std::path::PathBuf::from(".claude.json"))
        }
        AgentType::Codex => {
            home_dir().map(|h| h.join(".codex").join("config.toml")).unwrap_or_else(|| std::path::PathBuf::from(".codex/config.toml"))
        }
        AgentType::OpenCode => {
            std::env::current_dir().unwrap_or_default().join("opencode.json")
        }
        AgentType::Antigravity => {
            home_dir().map(|h| h.join(".gemini").join("antigravity").join("mcp_config.json"))
                .unwrap_or_else(|| std::path::PathBuf::from(".gemini/antigravity/mcp_config.json"))
        }
        AgentType::OpenClaw => {
            home_dir().map(|h| h.join(".openclaw").join("openclaw.json"))
                .unwrap_or_else(|| std::path::PathBuf::from(".openclaw/openclaw.json"))
        }
    }
}

fn home_dir() -> Option<std::path::PathBuf> {
    std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .ok()
        .map(std::path::PathBuf::from)
}

fn format_json_mcp_servers(binary: &str) -> String {
    let config = serde_json::json!({
        "mcpServers": {
            "ocean": {
                "command": binary,
                "args": []
            }
        }
    });
    serde_json::to_string_pretty(&config).unwrap_or_default()
}

fn format_toml_mcp_servers(binary: &str) -> String {
    format!(
        r#"[mcp_servers.ocean]
command = "{}"
args = []
"#,
        binary
    )
}

fn format_opencode_json(binary: &str) -> String {
    let config = serde_json::json!({
        "mcp": {
            "ocean": {
                "type": "local",
                "command": [binary],
                "enabled": true
            }
        }
    });
    serde_json::to_string_pretty(&config).unwrap_or_default()
}

fn format_openclaw_json(binary: &str) -> String {
    let config = serde_json::json!({
        "mcpServers": {
            "ocean": {
                "command": binary,
                "args": []
            }
        }
    });
    serde_json::to_string_pretty(&config).unwrap_or_default()
}

pub(crate) fn merge_json_mcp_servers(existing: &str, binary: &str) -> Result<(String, bool), String> {
    let mut root: Value = if existing.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(existing).map_err(|e| format!("Failed to parse JSON: {}", e))?
    };

    let mcp_servers = root.get_mut("mcpServers");
    if let Some(Value::Object(servers)) = mcp_servers {
        if servers.contains_key("ocean") {
            return Ok((existing.to_string(), true));
        }
        servers.insert(
            "ocean".to_string(),
            serde_json::json!({ "command": binary, "args": [] }),
        );
    } else {
        let mut servers = Map::new();
        servers.insert(
            "ocean".to_string(),
            serde_json::json!({ "command": binary, "args": [] }),
        );
        root.as_object_mut()
            .unwrap()
            .insert("mcpServers".to_string(), Value::Object(servers));
    }

    let output = serde_json::to_string_pretty(&root)
        .map_err(|e| format!("Failed to serialize JSON: {}", e))?;
    Ok((output, false))
}

pub(crate) fn merge_opencode_json(existing: &str, binary: &str) -> Result<(String, bool), String> {
    let mut root: Value = if existing.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(existing).map_err(|e| format!("Failed to parse JSON: {}", e))?
    };

    let mcp = root.get_mut("mcp");
    if let Some(Value::Object(servers)) = mcp {
        if servers.contains_key("ocean") {
            return Ok((existing.to_string(), true));
        }
        servers.insert(
            "ocean".to_string(),
            serde_json::json!({
                "type": "local",
                "command": [binary],
                "enabled": true
            }),
        );
    } else {
        let mut servers = Map::new();
        servers.insert(
            "ocean".to_string(),
            serde_json::json!({
                "type": "local",
                "command": [binary],
                "enabled": true
            }),
        );
        root.as_object_mut()
            .unwrap()
            .insert("mcp".to_string(), Value::Object(servers));
    }

    let output = serde_json::to_string_pretty(&root)
        .map_err(|e| format!("Failed to serialize JSON: {}", e))?;
    Ok((output, false))
}

pub(crate) fn merge_toml_mcp_servers(existing: &str, binary: &str) -> Result<(String, bool), String> {
    let mut root: toml::Value = if existing.trim().is_empty() {
        toml::Value::Table(toml::value::Table::new())
    } else {
        existing.parse::<toml::Value>()
            .map_err(|e| format!("Failed to parse TOML: {}", e))?
    };

    let mcp_servers = root.get_mut("mcp_servers");
    if let Some(toml::Value::Table(servers)) = mcp_servers {
        if servers.contains_key("ocean") {
            return Ok((existing.to_string(), true));
        }
        servers.insert(
            "ocean".to_string(),
            toml::Value::Table({
                let mut t = toml::value::Table::new();
                t.insert("command".to_string(), toml::Value::String(binary.to_string()));
                t.insert("args".to_string(), toml::Value::Array(vec![]));
                t
            }),
        );
    } else {
        let mut servers = toml::value::Table::new();
        servers.insert(
            "ocean".to_string(),
            toml::Value::Table({
                let mut t = toml::value::Table::new();
                t.insert("command".to_string(), toml::Value::String(binary.to_string()));
                t.insert("args".to_string(), toml::Value::Array(vec![]));
                t
            }),
        );
        root.as_table_mut()
            .unwrap()
            .insert("mcp_servers".to_string(), toml::Value::Table(servers));
    }

    let output = toml::to_string_pretty(&root)
        .map_err(|e| format!("Failed to serialize TOML: {}", e))?;
    Ok((output, false))
}

pub(crate) fn merge_openclaw_json(existing: &str, binary: &str) -> Result<(String, bool), String> {
    let mut root: Value = if existing.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(existing).map_err(|e| format!("Failed to parse JSON: {}", e))?
    };

    let mcp = root.get_mut("mcp");
    if let Some(Value::Object(mcp_obj)) = mcp {
        let servers = mcp_obj.get_mut("servers");
        if let Some(Value::Object(srv)) = servers {
            if srv.contains_key("ocean") {
                return Ok((existing.to_string(), true));
            }
            srv.insert(
                "ocean".to_string(),
                serde_json::json!({ "command": binary, "args": [] }),
            );
        } else {
            let mut srv = Map::new();
            srv.insert(
                "ocean".to_string(),
                serde_json::json!({ "command": binary, "args": [] }),
            );
            mcp_obj.insert("servers".to_string(), Value::Object(srv));
        }
        return serde_json::to_string_pretty(&root)
            .map(|o| (o, false))
            .map_err(|e| format!("Failed to serialize JSON: {}", e));
    }

    let mut srv = Map::new();
    srv.insert(
        "ocean".to_string(),
        serde_json::json!({ "command": binary, "args": [] }),
    );
    let mut mcp_obj = Map::new();
    mcp_obj.insert("servers".to_string(), Value::Object(srv));
    root.as_object_mut()
        .unwrap()
        .insert("mcp".to_string(), Value::Object(mcp_obj));

    let output = serde_json::to_string_pretty(&root)
        .map_err(|e| format!("Failed to serialize JSON: {}", e))?;
    Ok((output, false))
}

#[cfg(test)]
use super::agents::ConfigFormat;

#[cfg(test)]
impl AgentConfig {
    pub fn test_config(agent_type: AgentType, path: Option<std::path::PathBuf>) -> Self {
        let config_format = match agent_type {
            AgentType::Codex => ConfigFormat::Toml,
            _ => ConfigFormat::Json,
        };
        let config_key_description = match agent_type {
            AgentType::OpenCode => "mcp",
            AgentType::OpenClaw => "mcp.servers",
            _ => "mcpServers",
        };
        AgentConfig {
            agent_type,
            config_path: path,
            installed: true,
            config_format,
            config_key_description,
        }
    }
}


