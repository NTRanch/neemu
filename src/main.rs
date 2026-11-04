#![forbid(unsafe_code)]
//! Neemu MCP server binary: newline-delimited JSON-RPC over stdio.
//!
//! Protocol faults (parse errors, unknown methods, malformed
//! arguments) are JSON-RPC errors; gate refusals from a valid tool
//! call are tool results with `isError` and a typed `error.kind`
//! (SPEC §5, Tiger MCP Rule 6) — the two categories never mix.
//!
//! Registry root: `--home <dir>`, else `$NEEMU_HOME`, else
//! `$XDG_DATA_HOME/neemu` or `~/.local/share/neemu`.
//!
//! Tool profile (D-934): `--profile <spec>`, else
//! `$NEEMU_TOOL_PROFILE`, else `tool_profile` in the registry
//! root's `neemu.toml`, else the default `core` profile. The
//! profile scopes the whole served catalog for the process; an
//! invalid spec is a startup error (fail closed), never a silent
//! fall back to the full surface.
//!
//! As of 0.2.0 (WS2 of the MCP standardization program, decision
//! D-113) this server runs on rmcp, the official Rust MCP SDK, and
//! speaks the modern protocol era only (2026-07-28): sessions open
//! with `server/discover`, and every request carries its protocol
//! version in `_meta` — there is no `initialize` handshake. Legacy
//! contact is refused at the stdio front door (see `guard`) with
//! the typed `-32022` unsupported-version error, naming 2026-07-28
//! as the one supported version. The registry, the tool surface,
//! and the gate law are unchanged.

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use neemu::guard;
use neemu::profile::{config_tool_profile, ToolProfile};
use neemu::server::NeemuServer;
use neemu::Neemu;
use rmcp::transport::async_rw::AsyncRwTransport;
use rmcp::ServiceExt;
use tokio::sync::Mutex;

/// Duplex buffer between the era guard and the SDK service. Request
/// lines can reach the 4 MiB line bound; the SDK consumes
/// incrementally, so this only needs to cover response bursts.
const DUPLEX_BUFFER_BYTES: usize = 1024 * 1024;

fn registry_root() -> PathBuf {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--home" {
            if let Some(dir) = args.next() {
                return PathBuf::from(dir);
            }
        }
    }
    if let Ok(home) = std::env::var("NEEMU_HOME") {
        if !home.is_empty() {
            return PathBuf::from(home);
        }
    }
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        if !xdg.is_empty() {
            return PathBuf::from(xdg).join("neemu");
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".local")
        .join("share")
        .join("neemu")
}

/// The `--profile` argument's value, if the flag is present.
/// A trailing flag with no value is a startup error, not a
/// silent default.
fn profile_argument() -> Result<Option<String>, String> {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--profile" {
            return match args.next() {
                Some(value) => Ok(Some(value)),
                None => Err("--profile requires a value".to_string()),
            };
        }
        if let Some(value) = arg.strip_prefix("--profile=") {
            return Ok(Some(value.to_string()));
        }
    }
    Ok(None)
}

/// Resolve the launch tool profile by precedence (argument →
/// `NEEMU_TOOL_PROFILE` → `tool_profile` in the registry root's
/// `neemu.toml` → default `core`). A present-but-unreadable or
/// malformed config file fails closed.
fn launch_profile(root: &std::path::Path) -> Result<ToolProfile, String> {
    let argument = profile_argument()?;
    let environment = std::env::var("NEEMU_TOOL_PROFILE").ok();
    let config_path = root.join("neemu.toml");
    let config = match std::fs::read_to_string(&config_path) {
        Ok(contents) => config_tool_profile(&contents).map_err(|error| error.to_string())?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => {
            return Err(format!("cannot read config at {config_path:?}: {error}"));
        }
    };
    ToolProfile::resolve(
        argument.as_deref(),
        environment.as_deref(),
        config.as_deref(),
    )
    .map_err(|error| error.to_string())
}

fn main() -> ExitCode {
    // The baseline opened the registry before reading a single
    // frame and failed fast when it could not; keep that contract.
    let root = registry_root();
    let profile = match launch_profile(&root) {
        Ok(profile) => profile,
        Err(error) => {
            eprintln!("neemu: {error}");
            return ExitCode::FAILURE;
        }
    };
    let neemu = match Neemu::open(root.clone()) {
        Ok(neemu) => neemu,
        Err(error) => {
            eprintln!("neemu: cannot open registry at {root:?}: {error}");
            return ExitCode::FAILURE;
        }
    };
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("neemu: cannot start the async runtime: {error}");
            return ExitCode::FAILURE;
        }
    };
    match runtime.block_on(run(neemu, profile)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("neemu: serve loop ended: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Wire the pieces together: the guard owns stdio, the SDK service
/// sits behind an in-process duplex, and a pump copies its responses
/// back to stdout through the same lock the guard writes with. When
/// the serve ends (stdin EOF), the process exits — the baseline
/// front end's exit contract.
async fn run(neemu: Neemu, profile: ToolProfile) -> std::io::Result<()> {
    let (client_side, server_side) = tokio::io::duplex(DUPLEX_BUFFER_BYTES);
    let (server_read, server_write) = tokio::io::split(server_side);
    let (client_read, client_write) = tokio::io::split(client_side);
    let stdout: guard::SharedStdout = Arc::new(Mutex::new(tokio::io::stdout()));

    let ordering = guard::Ordering::new();
    let handler = NeemuServer::new(neemu, profile);
    let serve = tokio::spawn(async move {
        let transport = AsyncRwTransport::new_server(server_read, server_write);
        match handler.serve(transport).await {
            Ok(running) => running
                .waiting()
                .await
                .map(|_| ())
                .map_err(|e| std::io::Error::other(e.to_string())),
            // The client closed the connection before selecting a
            // lifecycle (a probe, a spawn check, a client that only
            // ever wanted the refusal): the baseline treated bare
            // EOF as a clean end of session, and so do we.
            Err(rmcp::service::ServerInitializeError::ConnectionClosed(_)) => Ok(()),
            Err(e) => Err(std::io::Error::other(e.to_string())),
        }
    });
    let mut pump = tokio::spawn(guard::pump(
        client_read,
        Arc::clone(&stdout),
        Arc::clone(&ordering),
    ));

    // When stdin reaches EOF the guard shuts the duplex write side
    // down, which ends the SDK service; the pump ends when the
    // service's transport drops. The pump wait is bounded as a
    // backstop: a response still in flight when the service ends
    // gets its chance to drain, but a wedged pump cannot hang exit.
    let guard_result = guard::run(tokio::io::stdin(), client_write, stdout, ordering).await;
    let serve_result = match serve.await {
        Ok(result) => result,
        Err(e) => Err(std::io::Error::other(e.to_string())),
    };
    if tokio::time::timeout(std::time::Duration::from_secs(2), &mut pump)
        .await
        .is_err()
    {
        pump.abort();
    }
    guard_result.and(serve_result)
}
