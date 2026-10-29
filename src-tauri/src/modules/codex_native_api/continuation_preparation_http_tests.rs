use super::{contract::*, diagnostics::DiagnosticContext, start, Options};
use axum::{extract::State, http::HeaderMap, routing::post, Json, Router};
use futures_util::future::BoxFuture;
use serde_json::{json, Value};
use std::{
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

const CLIENT_THREAD: &str = "synthetic-private-thread";
const CLIENT_SESSION: &str = "synthetic-private-session";
const CLIENT_TOKEN: &str = "synthetic-private-bearer";
const CLIENT_PROMPT: &str = "synthetic private prompt must not enter preflight metadata";
const REJECTION_MESSAGE: &str =
    "The connection's account is no longer available; reconnect with full input";

struct PreparationHost {
    base_url: String,
    owner_available: AtomicBool,
    reports: Mutex<Vec<RequestReport>>,
    diagnostics: Mutex<Vec<DiagnosticContext>>,
    preparation_rejections: Mutex<Vec<ContinuationPreparationRejection>>,
}

impl Host for PreparationHost {
    fn authorize<'a>(&'a self, _: &'a str) -> BoxFuture<'a, Result<Caller, ServiceError>> {
        Box::pin(async {
            Ok(Caller {
                id: "synthetic-caller".into(),
                label: "synthetic".into(),
            })
        })
    }

    fn route<'a>(
        &'a self,
        _: &'a Caller,
        request: RouteRequest,
    ) -> BoxFuture<'a, Result<Route, ServiceError>> {
        Box::pin(async move {
            if !self.owner_available.load(Ordering::SeqCst) {
                return Err(ServiceError::new(
                    409,
                    "account_binding_unavailable",
                    REJECTION_MESSAGE,
                )
                .with_retry_after(Some(Duration::from_millis(1500))));
            }
            Ok(Route {
                account_id: "synthetic-account".into(),
                continuity_account_id: None,
                owner_id: "synthetic-owner".into(),
                installation_profile: None,
                transport_identity: None,
                official_authorization_revision: None,
                official_registration_id: None,
                official_fallback_registration_id: None,
                official_fallback_authorization_revision: None,
                authorization_scope_fingerprint: None,
                official_fallback_failure: None,
                base_url: self.base_url.clone(),
                model: request.model,
                headers: HeaderMap::new(),
                client: reqwest::Client::builder().no_proxy().build().unwrap(),
                proxy_url: None,
                proxy_route: None,
                chatgpt: false,
                chatgpt_plan: false,
                wire_api: WireApi::Responses,
                compact_endpoint: None,
                websocket_endpoint: None,
                default_service_tier: None,
                permit: None,
            })
        })
    }

    fn execution_policy(&self) -> BoxFuture<'_, Result<ExecutionPolicy, ServiceError>> {
        Box::pin(async {
            Ok(ExecutionPolicy {
                account_switch_rebuild: false,
                credential_retries: 1,
                status_retries: 0,
                send_retries: 0,
                bootstrap_retries: 0,
                total_timeout: Duration::from_secs(6),
                open_timeout: Duration::from_secs(3),
                idle_timeout: Duration::from_secs(3),
                ..Default::default()
            })
        })
    }

    fn models<'a>(&'a self, _: &'a Caller) -> BoxFuture<'a, Result<Value, ServiceError>> {
        Box::pin(async { Ok(json!({"data": []})) })
    }

    fn record(&self, report: RequestReport) -> BoxFuture<'_, ()> {
        Box::pin(async move {
            self.reports.lock().unwrap().push(report);
        })
    }

    fn diagnostic(&self, context: DiagnosticContext) -> BoxFuture<'_, ()> {
        Box::pin(async move {
            self.diagnostics.lock().unwrap().push(context);
        })
    }

    fn continuation_preparation_rejected(&self, event: ContinuationPreparationRejection) {
        self.preparation_rejections.lock().unwrap().push(event);
    }
}

async fn synthetic_upstream(State(requests): State<Arc<AtomicUsize>>) -> Json<Value> {
    requests.fetch_add(1, Ordering::SeqCst);
    Json(json!({
        "id": "synthetic-success",
        "status": "completed",
        "output": [],
        "usage": {"input_tokens": 1, "output_tokens": 1}
    }))
}

fn assert_fingerprint(value: Option<&str>, raw: &str) {
    let value = value.expect("valid client identity must have a fingerprint");
    assert_eq!(value.len(), 64);
    assert!(value.bytes().all(|byte| byte.is_ascii_hexdigit()));
    assert_ne!(value, raw);
}

#[tokio::test]
async fn continuation_preparation_http_rejection_is_metadata_only_and_same_request_recovers() {
    let upstream_requests = Arc::new(AtomicUsize::new(0));
    let upstream = Router::new()
        .route("/v1/responses", post(synthetic_upstream))
        .with_state(upstream_requests.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base_url = format!("http://{}/v1", listener.local_addr().unwrap());
    let upstream_task = tokio::spawn(async move {
        axum::serve(listener, upstream).await.unwrap();
    });
    let host = Arc::new(PreparationHost {
        base_url,
        owner_available: AtomicBool::new(false),
        reports: Mutex::new(Vec::new()),
        diagnostics: Mutex::new(Vec::new()),
        preparation_rejections: Mutex::new(Vec::new()),
    });
    let dir = tempfile::tempdir().unwrap();
    let mut handle = start(
        "127.0.0.1:0".parse().unwrap(),
        host.clone(),
        dir.path().join("identity.sqlite"),
        Options::default(),
    )
    .await
    .unwrap();
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(6))
        .build()
        .unwrap();
    let body = json!({
        "model": "synthetic-model",
        "stream": false,
        "input": [{"role": "user", "content": CLIENT_PROMPT}]
    });
    let request = || {
        client
            .post(format!("http://{}/v1/responses", handle.addr))
            .bearer_auth(CLIENT_TOKEN)
            .header("thread-id", CLIENT_THREAD)
            .header("session-id", CLIENT_SESSION)
            .json(&body)
    };

    let rejected = request().send().await.unwrap();
    assert_eq!(rejected.status(), 409);
    let rejected_body: Value = rejected.json().await.unwrap();
    assert_eq!(
        rejected_body["error"]["code"],
        "account_binding_unavailable"
    );
    assert_eq!(rejected_body["error"]["message"], REJECTION_MESSAGE);
    assert_eq!(upstream_requests.load(Ordering::SeqCst), 0);
    assert!(host.reports.lock().unwrap().is_empty());
    assert!(host.diagnostics.lock().unwrap().is_empty());

    let events = host.preparation_rejections.lock().unwrap().clone();
    assert_eq!(events.len(), 1);
    let event = &events[0];
    assert!(!event.request_id.is_empty());
    assert!(event.started_at_ms > 0);
    assert_eq!(event.status, 409);
    assert_eq!(event.code, "account_binding_unavailable");
    assert_eq!(event.retry_after_ms, Some(1500));
    assert_fingerprint(event.thread_fingerprint.as_deref(), CLIENT_THREAD);
    assert_fingerprint(event.session_fingerprint.as_deref(), CLIENT_SESSION);
    let metadata = serde_json::to_value(event).unwrap();
    let mut keys: Vec<_> = metadata
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "code",
            "elapsedMs",
            "requestId",
            "retryAfterMs",
            "sessionFingerprint",
            "startedAtMs",
            "status",
            "threadFingerprint",
        ]
    );
    let serialized = metadata.to_string();
    for private_value in [
        CLIENT_THREAD,
        CLIENT_SESSION,
        CLIENT_TOKEN,
        CLIENT_PROMPT,
        REJECTION_MESSAGE,
    ] {
        assert!(!serialized.contains(private_value));
    }

    // Only the synthetic owner's availability changes. This is not a quota-reset
    // or cross-account replay test; the caller sends exactly the same request.
    host.owner_available.store(true, Ordering::SeqCst);
    let recovered = request().send().await.unwrap();
    assert_eq!(recovered.status(), 200);
    let recovered_body: Value = recovered.json().await.unwrap();
    assert_eq!(recovered_body["id"], "synthetic-success");
    assert_eq!(upstream_requests.load(Ordering::SeqCst), 1);
    assert_eq!(host.preparation_rejections.lock().unwrap().len(), 1);

    // Shutdown drains ordinary persistence before checking the recovered request.
    handle.shutdown(Duration::from_secs(2)).await.unwrap();
    {
        let reports = host.reports.lock().unwrap();
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].status, 200);
        assert!(reports[0].success);
    }
    assert_eq!(host.diagnostics.lock().unwrap().len(), 1);
    upstream_task.abort();
    let _ = upstream_task.await;
}
