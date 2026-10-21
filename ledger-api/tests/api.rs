//! HTTP-level tests. The router is called in-process, so no network port is needed.

use axum::{
    Router,
    body::Body,
    http::{Method, Request, StatusCode},
};
use http_body_util::BodyExt;
use tower::ServiceExt;

const BUY: &str =
    r#"{"postings":[{"account":"cash","cents":-1000},{"account":"etf:VWRL","cents":1000}]}"#;
const SMALLER_BUY: &str =
    r#"{"postings":[{"account":"cash","cents":-5},{"account":"etf:VWRL","cents":5}]}"#;
const UNBALANCED: &str =
    r#"{"postings":[{"account":"cash","cents":-5},{"account":"etf:VWRL","cents":4}]}"#;

async fn call(
    app: &Router,
    method: Method,
    uri: &str,
    key: Option<&str>,
    body: &str,
) -> (StatusCode, String) {
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json");
    if let Some(key) = key {
        request = request.header("idempotency-key", key);
    }
    let request = request.body(Body::from(body.to_string())).unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8(bytes.to_vec()).unwrap())
}

async fn post(app: &Router, key: Option<&str>, body: &str) -> (StatusCode, String) {
    call(app, Method::POST, "/transactions", key, body).await
}

async fn balance(app: &Router, account: &str) -> String {
    let uri = format!("/accounts/{account}/balance");
    call(app, Method::GET, &uri, None, "").await.1
}

#[tokio::test]
async fn health_returns_ok() {
    let app = ledger_api::app();
    let (status, body) = call(&app, Method::GET, "/health", None, "").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "ok");
}

#[tokio::test]
async fn first_request_is_applied_and_a_retry_is_replayed() {
    let app = ledger_api::app();

    let (status, body) = post(&app, Some("k1"), BUY).await;
    assert_eq!(status, StatusCode::CREATED);
    assert!(body.contains("applied"));

    let (status, body) = post(&app, Some("k1"), BUY).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("replayed"));

    // Applied once, not twice.
    assert!(balance(&app, "etf:VWRL").await.contains(r#""cents":1000"#));
}

#[tokio::test]
async fn same_key_with_a_different_body_is_a_conflict() {
    let app = ledger_api::app();
    post(&app, Some("k1"), BUY).await;

    let (status, body) = post(&app, Some("k1"), SMALLER_BUY).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert!(body.contains("different request"));
    assert!(balance(&app, "etf:VWRL").await.contains(r#""cents":1000"#));
}

#[tokio::test]
async fn unbalanced_transaction_is_rejected() {
    let app = ledger_api::app();
    let (status, body) = post(&app, Some("k1"), UNBALANCED).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(body.contains("does not balance"));
}

#[tokio::test]
async fn missing_idempotency_key_is_a_bad_request() {
    let app = ledger_api::app();
    let (status, _) = post(&app, None, BUY).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn malformed_body_is_rejected_before_reaching_the_ledger() {
    let app = ledger_api::app();
    let (status, _) = post(&app, Some("k1"), r#"{"postings":[{"account":"cash"}]}"#).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn unknown_account_has_a_zero_balance() {
    let app = ledger_api::app();
    assert!(balance(&app, "nobody").await.contains(r#""cents":0"#));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn parallel_requests_with_one_key_apply_exactly_once() {
    let app = ledger_api::app();

    let handles: Vec<_> = (0..50)
        .map(|_| {
            let app = app.clone();
            tokio::spawn(async move { post(&app, Some("burst"), BUY).await.0 })
        })
        .collect();

    let mut created = 0;
    for handle in handles {
        if handle.await.unwrap() == StatusCode::CREATED {
            created += 1;
        }
    }

    assert_eq!(created, 1);
    assert!(balance(&app, "etf:VWRL").await.contains(r#""cents":1000"#));
}
