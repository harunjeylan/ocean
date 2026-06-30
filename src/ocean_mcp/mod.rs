pub mod tools;
pub mod server;
pub mod config;
pub mod resources;

use rmcp::ServiceExt;

pub fn run() {
    crate::ocean_cli::config::load_env_files();
    let ocean_config = crate::ocean_cli::config::OceanConfig::load();
    let mcp_config = config::McpConfig::from_ocean_config(ocean_config.as_ref());
    let srv = server::OceanMcpServer::new(mcp_config);

    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    rt.block_on(async {
        eprintln!("ocean-mcp v{} starting on stdio transport", env!("CARGO_PKG_VERSION"));
        let transport = rmcp::transport::io::stdio();
        match srv.serve(transport).await {
            Ok(handle) => {
                if let Err(e) = handle.waiting().await {
                    eprintln!("Server exited: {}", e);
                }
            }
            Err(e) => {
                eprintln!("Failed to serve: {}", e);
            }
        }
    });
}
