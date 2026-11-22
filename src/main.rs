//! The `wc3-mcp` binary: the MCP server alone (the extension ships `wc3`,
//! whose `wc3 mcp` is the same: the library's `run`).

fn main() {
    wc3_mcp::run();
}
