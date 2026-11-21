//! `wc3-mcp`: an MCP server for Warcraft III's files, over stdio. A
//! client (Claude Code, say) starts it and sends JSON-RPC 2.0 messages, one
//! per line; the server answers each request on a line of its own. See
//! [`rpc`] for the protocol, [`slk`], [`mpq`], [`obj`] and [`wts`] for the
//! tools.

mod mpq;
mod obj;
mod rpc;
mod slk;
mod source;
mod w3c;
mod w3e;
mod w3i;
mod w3r;
mod wts;

use std::io::{BufRead, Write};

fn main() {
    let stdin = std::io::stdin();
    let mut out = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        if let Some(answer) = rpc::handle(&line) {
            // The client went away: nothing more to do.
            if writeln!(out, "{answer}").and_then(|_| out.flush()).is_err() {
                break;
            }
        }
    }
}
