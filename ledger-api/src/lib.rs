//! HTTP API for the ledger, built with axum.
//!
//! [`app`] builds the router, so tests can call it in-process and `main` stays tiny.

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use ledger_core::error::LedgerError;
use ledger_core::idempotent::{IdempotentLedger, Outcome};
use ledger_core::money::Money;
use ledger_core::transaction::{Posting, Transaction};

type AppState = Arc<Mutex<IdempotentLedger>>;

#[derive(Serialize)]
struct BalanceResponse {
    account: String,
    cents: i64,
}

async fn balance(
    State(state): State<AppState>,
    Path(account): Path<String>,
) -> Result<Json<BalanceResponse>, ApiError> {
    let book = state.lock().map_err(|_| ApiError::Internal)?;
    let cents = book.ledger().balance(&account).0;
    Ok(Json(BalanceResponse { account, cents }))
}

#[derive(Deserialize)]
struct TransactionRequest {
    postings: Vec<PostingDto>,
}

#[derive(Deserialize)]
struct PostingDto {
    account: String,
    cents: i64,
}

#[derive(Serialize)]
struct TransactionResponse {
    outcome: &'static str,
}

enum ApiError {
    MissingKey,
    Internal,
    Ledger(LedgerError),
}

impl From<LedgerError> for ApiError {
    fn from(e: LedgerError) -> ApiError {
        ApiError::Ledger(e)
    }
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::MissingKey => (
                StatusCode::BAD_REQUEST,
                "missing Idempotency-Key header".to_string(),
            ),
            ApiError::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal error".to_string(),
            ),
            ApiError::Ledger(e) => {
                let status = match e {
                    LedgerError::KeyConflict => StatusCode::CONFLICT,
                    LedgerError::InsufficientFunds { .. }
                    | LedgerError::Unbalanced { .. }
                    | LedgerError::Empty
                    | LedgerError::Overflow => StatusCode::UNPROCESSABLE_ENTITY,
                };
                (status, e.to_string())
            }
        };
        (status, Json(ErrorBody { error: message })).into_response()
    }
}

async fn post_transaction(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<TransactionRequest>,
) -> Result<(StatusCode, Json<TransactionResponse>), ApiError> {
    let key = headers
        .get("idempotency-key")
        .and_then(|v| v.to_str().ok())
        .ok_or(ApiError::MissingKey)?;

    let postings: Vec<Posting> = req
        .postings
        .into_iter()
        .map(|p| Posting {
            account: p.account,
            amount: Money(p.cents),
        })
        .collect();
    let tx = Transaction::new(postings)?;

    let outcome = state
        .lock()
        .map_err(|_| ApiError::Internal)?
        .submit(key, tx)?;

    Ok(match outcome {
        Outcome::Applied => (
            StatusCode::CREATED,
            Json(TransactionResponse { outcome: "applied" }),
        ),
        Outcome::Replayed => (
            StatusCode::OK,
            Json(TransactionResponse {
                outcome: "replayed",
            }),
        ),
    })
}

/// Builds the HTTP router with a fresh, empty in-memory ledger.
pub fn app() -> Router {
    let state: AppState = Arc::new(Mutex::new(IdempotentLedger::new()));

    Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/accounts/{account}/balance", get(balance))
        .route("/transactions", post(post_transaction))
        .with_state(state)
}
