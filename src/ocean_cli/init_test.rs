use crate::ocean_cli::init::*;

#[test]
fn test_default_model_ollama() {
    assert_eq!(default_model("ollama"), "nomic-embed-text");
}

#[test]
fn test_default_model_openai() {
    assert_eq!(default_model("openai"), "text-embedding-3-small");
}

#[test]
fn test_default_model_gemini() {
    assert_eq!(default_model("gemini"), "gemini-embedding-001");
}

#[test]
fn test_default_model_anthropic() {
    assert_eq!(default_model("anthropic"), "cohere-embed-multilingual-v3");
}

#[test]
fn test_default_model_unknown() {
    assert_eq!(default_model("unknown"), "nomic-embed-text");
}

#[test]
fn test_default_dimension_ollama() {
    assert_eq!(default_dimension("ollama", "nomic-embed-text"), 768);
}

#[test]
fn test_default_dimension_openai_small() {
    assert_eq!(default_dimension("openai", "text-embedding-3-small"), 1536);
}

#[test]
fn test_default_dimension_openai_large() {
    assert_eq!(default_dimension("openai", "text-embedding-3-large"), 3072);
}

#[test]
fn test_default_dimension_gemini() {
    assert_eq!(default_dimension("gemini", "gemini-embedding-001"), 3072);
}

#[test]
fn test_default_dimension_anthropic() {
    assert_eq!(default_dimension("anthropic", "cohere-embed-multilingual-v3"), 768);
}

#[test]
fn test_default_base_url_ollama() {
    assert_eq!(default_base_url("ollama"), "http://localhost:11434");
}

#[test]
fn test_default_base_url_openai() {
    assert_eq!(default_base_url("openai"), "https://api.openai.com/v1");
}

#[test]
fn test_default_base_url_anthropic() {
    assert_eq!(default_base_url("anthropic"), "https://api.anthropic.com/v1");
}

#[test]
fn test_default_base_url_gemini() {
    assert_eq!(default_base_url("gemini"), "");
}

#[test]
fn test_section_exists_found() {
    let content = "# Header\n\n## Ocean CLI\n\nSome content\n";
    assert!(section_exists(content, "## Ocean CLI"));
}

#[test]
fn test_section_exists_not_found() {
    let content = "# Header\n\n## Other Section\n\nSome content\n";
    assert!(!section_exists(content, "## Ocean CLI"));
}

#[test]
fn test_section_exists_empty() {
    assert!(!section_exists("", "## Ocean CLI"));
}

#[test]
fn test_write_config_creates_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_path_buf();

    write_config(&path, "ollama", "nomic-embed-text", 768, &None, "http://localhost:11434", false, false).unwrap();

    let config_path = path.join(".ocean").join("config.json");
    assert!(config_path.exists());
    let content = std::fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("ollama"));
    assert!(content.contains("nomic-embed-text"));
    assert!(content.contains("768"));
}

#[test]
fn test_write_config_with_api_key() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_path_buf();

    write_config(&path, "openai", "text-embedding-3-small", 1536, &Some("sk-test".to_string()), "https://api.openai.com/v1", false, false).unwrap();

    let config_path = path.join(".ocean").join("config.json");
    assert!(config_path.exists());
    let content = std::fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("sk-test"));
}

#[test]
fn test_ensure_claude_md_appends() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_path_buf();

    ensure_claude_md(&path).unwrap();

    let claude_path = path.join("CLAUDE.md");
    assert!(claude_path.exists());
    let content = std::fs::read_to_string(&claude_path).unwrap();
    assert!(content.contains("## Ocean CLI"));
}

#[test]
fn test_ensure_claude_md_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_path_buf();

    ensure_claude_md(&path).unwrap();
    ensure_claude_md(&path).unwrap();

    let content = std::fs::read_to_string(&path.join("CLAUDE.md")).unwrap();
    let count = content.matches("## Ocean CLI").count();
    assert_eq!(count, 1, "section should appear only once");
}

#[test]
fn test_ensure_agents_md_appends() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_path_buf();

    ensure_agents_md(&path, false, false).unwrap();

    let agents_path = path.join("AGENTS.md");
    assert!(agents_path.exists());
    let content = std::fs::read_to_string(&agents_path).unwrap();
    assert!(content.contains("## Ocean CLI Usage"));
}

#[test]
fn test_ensure_agents_md_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_path_buf();

    ensure_agents_md(&path, false, false).unwrap();
    ensure_agents_md(&path, false, false).unwrap();

    let content = std::fs::read_to_string(&path.join("AGENTS.md")).unwrap();
    let count = content.matches("## Ocean CLI Usage").count();
    assert_eq!(count, 1, "section should appear only once");
}

#[test]
fn test_ensure_ocean_cli_skill_creates() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_path_buf();

    ensure_ocean_cli_skill(&path, false, false).unwrap();

    let skill_path = path.join(".agents").join("skills").join("ocean-cli").join("SKILL.md");
    assert!(skill_path.exists());
    let content = std::fs::read_to_string(&skill_path).unwrap();
    assert!(content.contains("ocean init"));
}

#[test]
fn test_ensure_ocean_cli_skill_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_path_buf();

    ensure_ocean_cli_skill(&path, false, false).unwrap();
    ensure_ocean_cli_skill(&path, false, false).unwrap();

    let skill_path = path.join(".agents").join("skills").join("ocean-cli").join("SKILL.md");
    assert!(skill_path.exists());
}

#[test]
fn test_cmd_init_nonexistent_dir() {
    let result = cmd_init(Some("C:\\nonexistent_dir_12345".to_string()));
    assert!(result.is_err());
}

#[test]
fn test_cmd_init_valid_dir_no_interactive() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_path_buf();

    std::fs::write(path.join("CLAUDE.md"), "# Existing\n").unwrap();
    std::fs::write(path.join("AGENTS.md"), "# Existing\n").unwrap();

    write_config(&path, "ollama", "nomic-embed-text", 768, &None, "http://localhost:11434", false, false).unwrap();
    ensure_claude_md(&path).unwrap();
    ensure_agents_md(&path, false, false).unwrap();
    ensure_ocean_cli_skill(&path, false, false).unwrap();

    assert!(path.join(".ocean").join("config.json").exists());
    assert!(path.join("CLAUDE.md").exists());
    assert!(path.join("AGENTS.md").exists());
    assert!(path.join(".agents").join("skills").join("ocean-cli").join("SKILL.md").exists());
}
