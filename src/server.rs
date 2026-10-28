//! The rmcp server: tool surface and handler dispatch.
//!
//! Domain logic does not move in the WS2 migration — the eleven
//! tool handlers are the hand-rolled server's, reached through the
//! same `tools::call_tool` dispatch the baseline's front end ran,
//! with the baseline's own error split preserved across the seam:
//! gate refusals are in-band tool results (`isError` + the typed
//! `error.kind` JSON, SPEC §5), while malformed-argument failures
//! stay protocol errors (`-32602` with the baseline's message —
//! including the "unknown tool" cases, which the baseline also
//! answered as protocol errors). Only the framing changes sides of
//! the seam: rmcp owns the envelope, the era guard (see `guard.rs`)
//! owns the stdio front door, and this type owns the surface.
//!
//! Two era translations live here, both forced by the modern
//! protocol's statelessness and both invisible on the wire:
//!
//! - **No handshake state.** The baseline's `initialize` echo and
//!   its lifecycle-free dispatch both reduce to per-request
//!   handling; nothing in the domain ever read handshake state, so
//!   nothing is re-keyed.
//! - **Dispatch serialization.** The baseline's front end was a
//!   sequential loop — one call at a time, the file-backed registry
//!   store behind it. rmcp dispatches concurrently, so dispatch
//!   here takes a mutex for the whole call, preserving the
//!   baseline's ordering over the shared store.
//!
//! Modern-only posture lives in two places: the era guard refuses
//! `initialize` before the SDK sees it, and
//! [`ServerHandler::supported_protocol_versions`] pins this handler
//! to 2026-07-28, so `server/discover` advertises exactly one
//! version and the SDK validates every request's `_meta` against it.

use std::borrow::Cow;
use std::sync::Arc;

use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, Implementation,
    InitializeResult, ListToolsResult, PaginatedRequestParams, ProtocolVersion, ServerCapabilities,
    Tool, ToolAnnotations,
};
use rmcp::service::RequestContext;
use rmcp::{ErrorData, RoleServer, ServerHandler};
use serde_json::Value;
use tokio::sync::Mutex;

use crate::tools;
use crate::{CallError, Neemu};

/// The tool catalog: names, descriptions, input/output schemas, and
/// annotations are the baseline's own definitions, byte-for-byte —
/// rmcp accepts raw schemas, so no schemars translation stands
/// between the baseline and the wire. (This is also what keeps the
/// served `tools/list` payload inside the D-147 EFLO waiver: the
/// payload is the baseline's, plus the one annotation delta the
/// waiver job was commissioned to land — `neemu_export`'s
/// readOnlyHint, see `tools.rs`.)
fn tool_definitions() -> Vec<Tool> {
    let definitions = tools::tool_definitions();
    let entries = definitions.as_array().expect("definitions are an array");
    entries
        .iter()
        .map(|entry| {
            let name = entry["name"].as_str().expect("tool name").to_string();
            let description = entry["description"]
                .as_str()
                .expect("tool description")
                .to_string();
            let schema = entry["inputSchema"]
                .as_object()
                .expect("input schema is an object")
                .clone();
            let mut tool = Tool::new(name, description, schema);
            tool.output_schema = Some(Arc::new(
                entry["outputSchema"]
                    .as_object()
                    .expect("output schema is an object")
                    .clone(),
            ));
            let annotations = &entry["annotations"];
            let hint = |key: &str| annotations[key].as_bool();
            tool.annotations = Some(ToolAnnotations::from_raw(
                None,
                hint("readOnlyHint"),
                hint("destructiveHint"),
                hint("idempotentHint"),
                hint("openWorldHint"),
            ));
            tool
        })
        .collect()
}

fn text_result(text: String, is_error: bool) -> CallToolResult {
    let content = vec![ContentBlock::text(text)];
    if is_error {
        CallToolResult::error(content)
    } else {
        CallToolResult::success(content)
    }
}

/// The server handler. Owns the registry handle the baseline's
/// front end owned (opened in `main` before the serve starts, with
/// the baseline's fail-fast exit on an unopenable registry). A
/// clonable handle over shared state, so the serve can own one
/// clone while `main` keeps another.
#[derive(Clone)]
pub struct NeemuServer {
    neemu: Arc<Neemu>,
    dispatch: Arc<Mutex<()>>,
}

impl NeemuServer {
    pub fn new(neemu: Neemu) -> Self {
        NeemuServer {
            neemu: Arc::new(neemu),
            dispatch: Arc::new(Mutex::new(())),
        }
    }
}

impl ServerHandler for NeemuServer {
    fn get_info(&self) -> InitializeResult {
        // Both model types are non-exhaustive: start from the
        // default and set exactly the fields this server reports.
        let mut server_info = Implementation::default();
        server_info.name = "neemu".to_string();
        server_info.version = env!("CARGO_PKG_VERSION").to_string();
        // The baseline declared `tools.listChanged: false` (the
        // catalog is fixed for the process's life); keep the
        // declaration truthful on the modern envelope.
        let mut capabilities = ServerCapabilities::builder().enable_tools().build();
        if let Some(tools) = capabilities.tools.as_mut() {
            tools.list_changed = Some(false);
        }
        let mut info = InitializeResult::default();
        info.protocol_version = ProtocolVersion::V_2026_07_28;
        info.capabilities = capabilities;
        info.server_info = server_info;
        info
    }

    fn supported_protocol_versions(&self) -> Cow<'static, [ProtocolVersion]> {
        Cow::Borrowed(&[ProtocolVersion::V_2026_07_28])
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult {
            tools: tool_definitions(),
            ..Default::default()
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let name = request.name.to_string();
        let arguments = Value::Object(request.arguments.unwrap_or_default());
        // The baseline served sequentially; hold the dispatch mutex
        // for the whole call so the file-backed registry store sees
        // the same ordering.
        let _serialized = self.dispatch.lock().await;
        let result = match tools::call_tool(&self.neemu, &name, &arguments) {
            Ok(value) => text_result(serde_json::to_string(&value).unwrap_or_default(), false),
            Err(CallError::Gate(error)) => text_result(
                serde_json::to_string(&error.to_json()).unwrap_or_default(),
                true,
            ),
            // The baseline's malformed-argument category: a protocol
            // error carrying the dispatch's own message ("unknown
            // tool: …", "missing required field …", …).
            Err(CallError::Args(message)) => return Err(ErrorData::invalid_params(message, None)),
        };
        Ok(result.into())
    }
}

/// The wire-shape smoke: the catalog the handler serves is the
/// baseline's definitions, in order. (Kept beside the conversion it
/// guards; the integration suite checks the same over the wire.)
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_matches_the_baseline_definitions() {
        let served: Vec<String> = tool_definitions()
            .iter()
            .map(|tool| tool.name.to_string())
            .collect();
        assert_eq!(served, tools::TOOL_NAMES);
        // 15 since the S3 build (four voice tools; was 11 in S1+S2).
        assert_eq!(served.len(), 15);
        assert_eq!(served[0], "neemu_register_work");
        assert_eq!(served[14], "neemu_export");
    }

    #[test]
    fn catalog_schemas_and_annotations_are_the_baselines_own() {
        let served = serde_json::json!(tool_definitions());
        let baseline = tools::tool_definitions();
        let served_entries = served.as_array().expect("array");
        let baseline_entries = baseline.as_array().expect("array");
        for (served, baseline) in served_entries.iter().zip(baseline_entries) {
            assert_eq!(served["inputSchema"], baseline["inputSchema"]);
            assert_eq!(served["outputSchema"], baseline["outputSchema"]);
            assert_eq!(served["description"], baseline["description"]);
            assert_eq!(served["annotations"], baseline["annotations"]);
        }
    }

    #[test]
    fn export_is_served_as_a_write() {
        // The D-147 honesty fix, pinned at the served surface:
        // neemu_export writes its bundle into the registry store,
        // so its readOnlyHint must be false (it was true in S1/S2).
        let export = tool_definitions()
            .into_iter()
            .find(|tool| tool.name == "neemu_export")
            .expect("export tool");
        let annotations = export.annotations.expect("annotations");
        assert_eq!(annotations.read_only_hint, Some(false));
    }
}
