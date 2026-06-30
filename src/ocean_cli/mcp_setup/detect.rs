use std::path::PathBuf;

use super::agents::{AgentConfig, AgentType, ConfigFormat};

pub fn detect_agent(agent_type: AgentType) -> AgentConfig {
    match agent_type {
        AgentType::ClaudeDesktop => detect_claude_desktop(),
        AgentType::ClaudeCode => detect_claude_code(),
        AgentType::Codex => detect_codex(),
        AgentType::OpenCode => detect_opencode(),
        AgentType::Antigravity => detect_antigravity(),
        AgentType::OpenClaw => detect_openclaw(),
    }
}

pub fn detect_all() -> Vec<AgentConfig> {
    AgentType::all().into_iter().map(detect_agent).collect()
}

fn detect_claude_desktop() -> AgentConfig {
    let path = std::env::var("APPDATA")
        .ok()
        .map(|a| PathBuf::from(a).join("Claude").join("claude_desktop_config.json"))
        .filter(|p| p.exists());
    AgentConfig {
        agent_type: AgentType::ClaudeDesktop,
        config_path: path,
        installed: true,
        config_format: ConfigFormat::Json,
        config_key_description: "mcpServers",
    }
}

fn detect_claude_code() -> AgentConfig {
    let path = std::env::current_dir()
        .ok()
        .map(|cwd| cwd.join(".mcp.json"))
        .filter(|p| p.exists())
        .or_else(|| {
            home_dir().map(|h| h.join(".claude.json")).filter(|p| p.exists())
        });
    AgentConfig {
        agent_type: AgentType::ClaudeCode,
        config_path: path,
        installed: true,
        config_format: ConfigFormat::Json,
        config_key_description: "mcpServers",
    }
}

fn detect_codex() -> AgentConfig {
    let path = home_dir()
        .map(|h| h.join(".codex").join("config.toml"))
        .filter(|p| p.exists());
    AgentConfig {
        agent_type: AgentType::Codex,
        config_path: path,
        installed: true,
        config_format: ConfigFormat::Toml,
        config_key_description: "[mcp_servers.ocean]",
    }
}

fn detect_opencode() -> AgentConfig {
    let path = std::env::current_dir()
        .ok()
        .map(|cwd| cwd.join("opencode.json"))
        .filter(|p| p.exists())
        .or_else(|| {
            home_dir()
                .map(|h| h.join(".config").join("opencode").join("opencode.json"))
                .filter(|p| p.exists())
        });
    AgentConfig {
        agent_type: AgentType::OpenCode,
        config_path: path,
        installed: true,
        config_format: ConfigFormat::Json,
        config_key_description: "mcp",
    }
}

fn detect_antigravity() -> AgentConfig {
    let path = home_dir()
        .map(|h| h.join(".gemini").join("antigravity").join("mcp_config.json"))
        .filter(|p| p.exists());
    AgentConfig {
        agent_type: AgentType::Antigravity,
        config_path: path,
        installed: true,
        config_format: ConfigFormat::Json,
        config_key_description: "mcpServers",
    }
}

fn detect_openclaw() -> AgentConfig {
    let path = home_dir()
        .map(|h| h.join(".openclaw").join("openclaw.json"))
        .filter(|p| p.exists());
    AgentConfig {
        agent_type: AgentType::OpenClaw,
        config_path: path,
        installed: true,
        config_format: ConfigFormat::Json,
        config_key_description: "mcp.servers",
    }
}

fn home_dir() -> Option<PathBuf> {
    std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .ok()
        .map(PathBuf::from)
}
