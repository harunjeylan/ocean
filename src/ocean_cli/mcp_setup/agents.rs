use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentType {
    ClaudeDesktop,
    ClaudeCode,
    Codex,
    OpenCode,
    Antigravity,
    OpenClaw,
}

impl AgentType {
    pub fn name(&self) -> &'static str {
        match self {
            AgentType::ClaudeDesktop => "claude-desktop",
            AgentType::ClaudeCode => "claude-code",
            AgentType::Codex => "codex",
            AgentType::OpenCode => "opencode",
            AgentType::Antigravity => "antigravity",
            AgentType::OpenClaw => "openclaw",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            AgentType::ClaudeDesktop => "Claude Desktop",
            AgentType::ClaudeCode => "Claude Code CLI",
            AgentType::Codex => "Codex CLI",
            AgentType::OpenCode => "OpenCode",
            AgentType::Antigravity => "Google Antigravity",
            AgentType::OpenClaw => "OpenClaw",
        }
    }

    pub fn from_str(s: &str) -> Option<AgentType> {
        match s {
            "claude-desktop" => Some(AgentType::ClaudeDesktop),
            "claude-code" => Some(AgentType::ClaudeCode),
            "codex" => Some(AgentType::Codex),
            "opencode" => Some(AgentType::OpenCode),
            "antigravity" => Some(AgentType::Antigravity),
            "openclaw" => Some(AgentType::OpenClaw),
            _ => None,
        }
    }

    pub fn all() -> Vec<AgentType> {
        vec![
            AgentType::ClaudeDesktop,
            AgentType::ClaudeCode,
            AgentType::Codex,
            AgentType::OpenCode,
            AgentType::Antigravity,
            AgentType::OpenClaw,
        ]
    }

    pub fn known_names() -> Vec<&'static str> {
        Self::all().iter().map(|a| a.name()).collect()
    }
}

impl fmt::Display for AgentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigFormat {
    Json,
    Toml,
}

#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub agent_type: AgentType,
    pub config_path: Option<std::path::PathBuf>,
    pub installed: bool,
    pub config_format: ConfigFormat,
    pub config_key_description: &'static str,
}
