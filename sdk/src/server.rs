use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use tokio::sync::OnceCell;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::ReceiverStream;

use crate::contract::EventData;
use crate::provider::{ChatRequest, ModelsResponse};
use crate::types::{Context, HookPoint};
use crate::{Plugin, PluginError};

// ---------------------------------------------------------------------------
// HTTP request bodies (private)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct ContextPayload {
    workspace_id: String,
    plugin_id: String,
    #[serde(default)]
    config: HashMap<String, String>,
    callback_url: String,
    callback_token: String,
}

impl From<ContextPayload> for Context {
    fn from(p: ContextPayload) -> Self {
        Self {
            workspace_id: p.workspace_id,
            plugin_id: p.plugin_id,
            config: p.config,
            callback_url: p.callback_url,
            callback_token: p.callback_token,
        }
    }
}

#[derive(Deserialize)]
struct WebhookHttpRequest {
    context: ContextPayload,
    route: String,
    #[serde(default)]
    headers: HashMap<String, String>,
    body: String,
}

#[derive(Deserialize)]
struct EventHttpRequest {
    context: ContextPayload,
    #[serde(default)]
    data: serde_json::Value,
    external_ref: Option<String>,
}

#[derive(Deserialize)]
struct HookHttpRequest {
    context: ContextPayload,
    point: HookPoint,
    #[serde(default)]
    data: serde_json::Value,
}

#[derive(Deserialize)]
struct ToolHttpRequest {
    context: ContextPayload,
    name: String,
    #[serde(default)]
    arguments: serde_json::Value,
}

// ---------------------------------------------------------------------------
// Error → HTTP status mapping
// ---------------------------------------------------------------------------

impl IntoResponse for PluginError {
    fn into_response(self) -> Response {
        let status = match &self {
            PluginError::InvalidParams(_) => StatusCode::BAD_REQUEST,
            PluginError::MethodNotFound(_) => StatusCode::NOT_FOUND,
            PluginError::SignatureInvalid => StatusCode::FORBIDDEN,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let body = serde_json::json!({ "error": self.to_string() });
        (status, Json(body)).into_response()
    }
}

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

struct AppState<P: Plugin> {
    plugin: P,
    manifest: OnceCell<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "ok": true }))
}

async fn manifest<P: Plugin>(
    State(state): State<Arc<AppState<P>>>,
) -> Result<Json<serde_json::Value>, Response> {
    let value = state
        .manifest
        .get_or_try_init(|| async {
            let content = tokio::fs::read_to_string("/grimoire.json")
                .await
                .map_err(|e| {
                    PluginError::Internal(format!("failed to read /grimoire.json: {e}"))
                })?;
            let parsed: serde_json::Value =
                serde_json::from_str(&content).map_err(PluginError::Json)?;
            Ok::<_, PluginError>(parsed)
        })
        .await
        .map_err(|e: PluginError| e.into_response())?;

    Ok(Json(value.clone()))
}

async fn handle_webhook<P: Plugin>(
    State(state): State<Arc<AppState<P>>>,
    Json(req): Json<WebhookHttpRequest>,
) -> Result<Json<serde_json::Value>, PluginError> {
    let ctx = Context::from(req.context);
    let result = state
        .plugin
        .handle_webhook(&ctx, &req.route, req.headers, &req.body)
        .await?;
    Ok(Json(
        serde_json::to_value(result).map_err(PluginError::Json)?,
    ))
}

async fn handle_event<P: Plugin>(
    State(state): State<Arc<AppState<P>>>,
    Json(req): Json<EventHttpRequest>,
) -> Result<Json<serde_json::Value>, PluginError> {
    let ctx = Context::from(req.context);
    let event_data: EventData =
        serde_json::from_value(req.data).map_err(|e| PluginError::InvalidParams(e.to_string()))?;
    state
        .plugin
        .handle_event(&ctx, &event_data, req.external_ref.as_deref())
        .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn handle_hook<P: Plugin>(
    State(state): State<Arc<AppState<P>>>,
    Json(req): Json<HookHttpRequest>,
) -> Result<Json<serde_json::Value>, PluginError> {
    let ctx = Context::from(req.context);
    let result = state.plugin.handle_hook(&ctx, req.point, req.data).await?;
    Ok(Json(
        serde_json::to_value(result).map_err(PluginError::Json)?,
    ))
}

async fn handle_tool<P: Plugin>(
    State(state): State<Arc<AppState<P>>>,
    Json(req): Json<ToolHttpRequest>,
) -> Result<Json<serde_json::Value>, PluginError> {
    let ctx = Context::from(req.context);
    let result = state
        .plugin
        .handle_tool_call(&ctx, &req.name, req.arguments)
        .await?;
    Ok(Json(serde_json::json!({ "result": result })))
}

// ---------------------------------------------------------------------------
// Provider handlers
// ---------------------------------------------------------------------------

async fn handle_models<P: Plugin>(
    State(state): State<Arc<AppState<P>>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, PluginError> {
    if !state.plugin.is_provider() {
        return Err(PluginError::MethodNotFound("models".into()));
    }

    let ctx = extract_context_from_header(&headers)?;
    let models = state.plugin.list_models(&ctx).await?;
    let response = ModelsResponse { models };
    Ok(Json(
        serde_json::to_value(response).map_err(PluginError::Json)?,
    ))
}

async fn handle_chat<P: Plugin>(
    State(state): State<Arc<AppState<P>>>,
    Json(req): Json<ChatRequest>,
) -> Result<Response, PluginError> {
    if !state.plugin.is_provider() {
        return Err(PluginError::MethodNotFound("chat".into()));
    }

    let ctx = extract_context_from_value(&req.context)?;

    if req.stream {
        handle_chat_stream(state, ctx, req)
    } else {
        let result = state.plugin.handle_chat(&ctx, &req).await?;
        let body = serde_json::to_value(result).map_err(PluginError::Json)?;
        Ok(Json(body).into_response())
    }
}

#[allow(clippy::unnecessary_wraps)]
fn handle_chat_stream<P: Plugin>(
    state: Arc<AppState<P>>,
    ctx: Context,
    req: ChatRequest,
) -> Result<Response, PluginError> {
    let (tx, rx) = tokio::sync::mpsc::channel(32);

    tokio::spawn(async move {
        if let Err(e) = state
            .plugin
            .handle_chat_stream(&ctx, &req, tx.clone())
            .await
        {
            tracing::error!(error = %e, "chat_stream error");
        }
        drop(tx);
    });

    let stream = ReceiverStream::new(rx).map(|chunk| {
        let data = serde_json::to_string(&chunk).unwrap_or_default();
        Ok::<_, Infallible>(Event::default().data(data))
    });

    // Append [DONE] sentinel after all chunks
    let done = tokio_stream::once(Ok::<_, Infallible>(Event::default().data("[DONE]")));
    let full_stream = stream.chain(done);

    Ok(Sse::new(full_stream)
        .keep_alive(KeepAlive::default())
        .into_response())
}

fn extract_context_from_header(headers: &HeaderMap) -> Result<Context, PluginError> {
    let encoded = headers
        .get("x-plugin-context")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| PluginError::InvalidParams("missing X-Plugin-Context header".into()))?;

    let decoded = base64_decode(encoded)
        .map_err(|e| PluginError::InvalidParams(format!("invalid base64 context: {e}")))?;

    serde_json::from_slice(&decoded).map_err(|e| PluginError::InvalidParams(e.to_string()))
}

fn extract_context_from_value(value: &serde_json::Value) -> Result<Context, PluginError> {
    serde_json::from_value(value.clone()).map_err(|e| PluginError::InvalidParams(e.to_string()))
}

fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(input)
        .map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Server entrypoint
// ---------------------------------------------------------------------------

/// Start the plugin HTTP server. This function never returns.
///
/// Listens on `0.0.0.0:<port>` where port is read from the `PLUGIN_PORT`
/// environment variable (default: 9999).
///
/// # Errors
///
/// Logs errors and exits the process if the TCP listener cannot bind
/// or the server encounters a fatal error.
pub async fn run<P: Plugin>(plugin: P) -> ! {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_target(false)
        .json()
        .init();

    let port: u16 = std::env::var("PLUGIN_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(9999);

    tracing::info!(name = plugin.name(), port, "plugin starting");

    let state = Arc::new(AppState {
        plugin,
        manifest: OnceCell::new(),
    });

    let app = Router::new()
        .route("/health", get(health))
        .route("/manifest", get(manifest::<P>))
        .route("/models", get(handle_models::<P>))
        .route("/webhook", post(handle_webhook::<P>))
        .route("/event", post(handle_event::<P>))
        .route("/hook", post(handle_hook::<P>))
        .route("/tool", post(handle_tool::<P>))
        .route("/chat", post(handle_chat::<P>))
        .with_state(state);

    let Ok(listener) = tokio::net::TcpListener::bind(("0.0.0.0", port)).await else {
        tracing::error!(port, "failed to bind TCP listener");
        std::process::exit(1);
    };

    tracing::info!("listening on 0.0.0.0:{port}");

    if let Err(e) = axum::serve(listener, app).await {
        tracing::error!(error = %e, "server error");
    }

    std::process::exit(1);
}
