fn continuation_rejection() -> ServiceError {
    ServiceError::new(
        409,
        "account_binding_unavailable",
        "private failure details",
    )
}

#[tokio::test]
async fn native_lifecycle_first_websocket_turn_keeps_handshake_authorization() {
    let (mut tracker, host, activity, _stop) = unsent_fixture();
    // The first WebSocket create reuses the authorized handshake context;
    // only later creates establish a new context and authorize it again.
    tracker.idle();
    tracker.start_turn().unwrap();
    tracker.fail(&continuation_rejection()).await;
    drop(tracker);
    assert_no_dispatch_records(&host, &activity).await;
    assert_eq!(host.preparation_rejections.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn native_lifecycle_preparation_rejection_is_once_without_payload_capture_or_accounting() {
    let (mut tracker, host, activity, _stop) = unsent_fixture();
    tracker.capture_payloads = false;
    let request_id = tracker.diagnostics.as_ref().unwrap().request_id.clone();
    let started_at_ms = tracker.diagnostics.as_ref().unwrap().started_at_ms;
    select_candidate(&mut tracker, "never-sent");
    let error = continuation_rejection().with_retry_after(Some(Duration::from_secs(3)));
    tracker.fail(&error).await;
    tracker.fail(&error).await;
    drop(tracker);
    assert_no_dispatch_records(&host, &activity).await;
    assert!(host.quota_checks.lock().unwrap().is_empty());
    let events = host.preparation_rejections.lock().unwrap();
    assert_eq!(events.len(), 1);
    let event = &events[0];
    assert_eq!(event.request_id, request_id);
    assert!(uuid::Uuid::parse_str(&event.request_id).is_ok());
    assert_eq!(event.started_at_ms, started_at_ms);
    assert_eq!(event.status, 409);
    assert_eq!(event.code, "account_binding_unavailable");
    assert_eq!(event.retry_after_ms, Some(3000));
    assert!(event.thread_fingerprint.is_none());
    assert!(event.session_fingerprint.is_none());
}

#[tokio::test]
async fn native_lifecycle_preparation_rejection_serializes_only_safe_metadata() {
    for capture_payloads in [false, true] {
        let (mut tracker, host, activity, _stop) = unsent_fixture();
        let secret = "private-marker-never-log";
        let mut headers = axum::http::HeaderMap::new();
        for name in [
            "thread-id",
            "session-id",
            "x-request-id",
            "authorization",
            "x-codex-installation-id",
            "chatgpt-account-id",
            "x-cockpit-instance-id",
        ] {
            headers.insert(name, secret.parse().unwrap());
        }
        tracker.context(secret, secret, &headers, capture_payloads);
        let caller = Caller {
            id: secret.into(),
            label: secret.into(),
        };
        tracker.authorized(&caller, secret);
        let mut route = dispatch_route(secret);
        route.headers = headers.clone();
        route.installation_profile =
            Some(crate::modules::codex_api_identity::InstallationProfile {
                version: 1,
                scope: secret.into(),
                installation_id: secret.into(),
                revision: 1,
            });
        tracker.routed(&caller, &route, secret);
        tracker.payload(secret, secret, &headers, secret);
        let error = ServiceError::new(409, "account_binding_unavailable", secret)
            .with_error_details(json!({"secret":secret}));
        tracker.fail(&error).await;
        drop(tracker);
        assert_no_dispatch_records(&host, &activity).await;
        let events = host.preparation_rejections.lock().unwrap();
        assert_eq!(events.len(), 1);
        let event = &events[0];
        let serialized = serde_json::to_value(event).unwrap();
        let mut keys: Vec<_> = serialized
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
                "sessionFingerprint",
                "startedAtMs",
                "status",
                "threadFingerprint"
            ]
        );
        assert!(!serialized.to_string().contains(secret));
        for fingerprint in [&event.thread_fingerprint, &event.session_fingerprint] {
            let fingerprint = fingerprint.as_ref().unwrap();
            assert_eq!(fingerprint.len(), 64);
            assert!(fingerprint.bytes().all(|byte| byte.is_ascii_hexdigit()));
        }
        assert_ne!(
            event.thread_fingerprint, event.session_fingerprint,
            "the same raw ID must not link different identity domains"
        );
    }
}

#[tokio::test]
async fn native_lifecycle_preparation_rejection_requires_current_turn_authorization() {
    for previous_authorization in [false, true] {
        let (mut tracker, host, activity, _stop) = unsent_fixture();
        if previous_authorization {
            tracker.complete_local().await;
            tracker.start_turn().unwrap();
        }
        tracker.context(
            "POST",
            "/v1/responses",
            &axum::http::HeaderMap::new(),
            false,
        );
        tracker.fail(&continuation_rejection()).await;
        drop(tracker);
        assert_no_dispatch_records(&host, &activity).await;
        assert!(host.preparation_rejections.lock().unwrap().is_empty());
    }
}

#[tokio::test]
async fn native_lifecycle_preparation_rejection_ignores_drop_handshake_and_unlisted_codes() {
    for code in [
        "client_cancelled",
        "service_stopping",
        "upstream_timeout",
        "account_cooling",
        "quota_reserved",
        "no_available_account",
        "account_binding_unavailable_extra",
        "continuation_unknown",
        "authorization_changed",
    ] {
        let (mut tracker, host, activity, _stop) = unsent_fixture();
        tracker
            .fail(&ServiceError::new(503, code, "private text"))
            .await;
        drop(tracker);
        assert_no_dispatch_records(&host, &activity).await;
        assert!(
            host.preparation_rejections.lock().unwrap().is_empty(),
            "{code}"
        );
    }
    for stopping in [false, true] {
        let (tracker, host, activity, stop) = unsent_fixture();
        if stopping {
            stop.send(true).unwrap();
        }
        drop(tracker);
        assert_no_dispatch_records(&host, &activity).await;
        assert!(host.preparation_rejections.lock().unwrap().is_empty());
    }
    let (mut tracker, host, activity, _stop) = unsent_fixture();
    tracker.transport_fallback(&continuation_rejection()).await;
    tracker.fail(&continuation_rejection()).await;
    drop(tracker);
    assert_no_dispatch_records(&host, &activity).await;
    assert!(host.preparation_rejections.lock().unwrap().is_empty());
}

#[tokio::test]
async fn native_lifecycle_preparation_rejection_never_reclassifies_a_real_dispatch() {
    for prior_429 in [false, true] {
        let (mut tracker, host, _activity, _stop) = unsent_fixture();
        select_candidate(&mut tracker, "sent");
        tracker.begin_attempt("http");
        if prior_429 {
            let error = ServiceError::new(429, "upstream_rate_limited", "actual rejection")
                .with_retry_after(Some(Duration::from_secs(30)));
            tracker.attempt_error(&error);
            tracker.set_retry_after(error.retry_after);
            select_candidate(&mut tracker, "unsent-candidate");
        }
        tracker.fail(&continuation_rejection()).await;
        drop(tracker);
        let report = recorded(&host).await;
        assert_eq!(report.account_id, "sent");
        assert_eq!(report.status, if prior_429 { 429 } else { 409 });
        if prior_429 {
            assert_eq!(report.error.as_deref(), Some("upstream_rate_limited"));
            assert_eq!(report.retry_after, Some(Duration::from_secs(30)));
        }
        assert!(host.preparation_rejections.lock().unwrap().is_empty());
    }
}

#[tokio::test]
async fn native_lifecycle_preparation_rejection_resets_for_each_authorized_turn() {
    let (mut tracker, host, activity, _stop) = unsent_fixture();
    let mut headers = axum::http::HeaderMap::new();
    headers.insert("thread-id", "stable-thread".parse().unwrap());
    headers.insert("session-id", "stable-session".parse().unwrap());
    for _ in 0..2 {
        tracker.start_turn().unwrap();
        tracker.context("GET", "/v1/responses", &headers, false);
        tracker.authorized(
            &Caller {
                id: "caller".into(),
                label: "test".into(),
            },
            "model",
        );
        tracker.fail(&continuation_rejection()).await;
        tracker.fail(&continuation_rejection()).await;
    }
    drop(tracker);
    assert_no_dispatch_records(&host, &activity).await;
    let events = host.preparation_rejections.lock().unwrap();
    assert_eq!(events.len(), 2);
    assert_ne!(events[0].request_id, events[1].request_id);
    assert_eq!(events[0].thread_fingerprint, events[1].thread_fingerprint);
    assert_eq!(events[0].session_fingerprint, events[1].session_fingerprint);
}

#[tokio::test]
async fn native_lifecycle_preparation_rejection_omits_invalid_identity_and_bounds_retry_after() {
    let (mut tracker, host, activity, _stop) = unsent_fixture();
    let mut headers = axum::http::HeaderMap::new();
    headers.insert("thread-id", "x".repeat(4097).parse().unwrap());
    headers.insert("session-id", "session".parse().unwrap());
    tracker.context("POST", "/v1/responses", &headers, false);
    tracker.authorized(
        &Caller {
            id: "caller".into(),
            label: "test".into(),
        },
        "model",
    );
    tracker
        .fail(&continuation_rejection().with_retry_after(Some(Duration::MAX)))
        .await;
    drop(tracker);
    assert_no_dispatch_records(&host, &activity).await;
    let events = host.preparation_rejections.lock().unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].retry_after_ms, Some(u64::MAX));
    assert!(events[0].thread_fingerprint.is_none());
    assert!(events[0].session_fingerprint.is_none());
}
