// This is our last exercise. Let's go down a more unstructured path!
// Try writing an **asynchronous REST API** to expose the functionality
// of the ticket management system we built throughout the course.
// It should expose endpoints to:
//  - Create a ticket
//  - Retrieve ticket details
//  - Patch a ticket
//
// Use Rust's package registry, crates.io, to find the dependencies you need
// (if any) to build this system.

mod ticket;

use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::routing::patch;
use axum::{
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use std::sync::{Arc, Mutex};
use ticket::{Status, TicketDescription, TicketDraft, TicketId, TicketStore, TicketTitle};

type SharedStore = Arc<Mutex<TicketStore>>;

#[tokio::main]
async fn main() {
    let store: SharedStore = Arc::new(Mutex::new(TicketStore::new()));
    let app = Router::new()
        .route("/tickets", post(create_ticket))
        .route("/tickets/{id}", get(get_ticket))
        .route("/tickets/{id}", patch(patch_ticket))
        .with_state(store.clone());

    let addr = "0.0.0.0:3000";
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    println!("Listening on {}", addr);
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

#[derive(Deserialize)]
struct CreateTicketRequest {
    title: String,
    description: String,
}

async fn create_ticket(
    State(store): State<SharedStore>,
    Json(req): Json<CreateTicketRequest>,
) -> impl IntoResponse {
    let title = match TicketTitle::try_from(req.title) {
        Ok(t) => t,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(format!("Invalid title: {}", e)),
            )
                .into_response();
        }
    };

    let description = match TicketDescription::try_from(req.description) {
        Ok(d) => d,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(format!("Invalid description: {}", e)),
            )
                .into_response();
        }
    };

    let draft = TicketDraft { title, description };

    let mut db = store.lock().unwrap();
    let id = db.add_ticket(draft);
    let ticket = db.get(id).cloned().unwrap();

    (StatusCode::CREATED, Json(ticket)).into_response()
}

async fn get_ticket(State(store): State<SharedStore>, Path(id): Path<u64>) -> impl IntoResponse {
    let db = store.lock().unwrap();
    let tid = TicketId(id);
    let Some(ticket) = db.get(tid) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    Json(ticket.clone()).into_response()
}

#[derive(Deserialize)]
struct PatchTicketRequest {
    title: Option<String>,
    description: Option<String>,
    status: Option<Status>,
}

async fn patch_ticket(
    State(store): State<SharedStore>,
    Path(id): Path<u64>,
    Json(req): Json<PatchTicketRequest>,
) -> impl IntoResponse {
    let mut db = store.lock().unwrap();
    let tid = TicketId(id);

    let Some(ticket) = db.get_mut(tid) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    if let Some(title) = req.title {
        let new_title = match TicketTitle::try_from(title) {
            Ok(t) => t,
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(format!("Invalid description: {}", e)),
                )
                    .into_response();
            }
        };

        ticket.title = new_title;
    }

    if let Some(description) = req.description {
        let new_desc = match TicketDescription::try_from(description) {
            Ok(d) => d,
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(format!("Invalid description: {}", e)),
                )
                    .into_response();
            }
        };

        ticket.description = new_desc
    }

    if let Some(status) = req.status {
        ticket.status = status;
    }

    Json(ticket.clone()).into_response()
}
