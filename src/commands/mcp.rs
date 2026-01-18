//! MCP server command implementation

use anyhow::Result;
use cctakt::McpServer;

/// Run cctakt as an MCP server
pub fn run_mcp() -> Result<()> {
    let mut server = McpServer::new()?;
    server.run()
}
