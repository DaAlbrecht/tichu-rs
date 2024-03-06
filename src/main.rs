mod events;
mod game_client;
mod game_core;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use axum::routing::get;
use serde::{Deserialize, Serialize};
use socketioxide::SocketIo;
use tracing::info;
use tracing_subscriber::FmtSubscriber;
use uuid::Uuid;

use crate::events::on_connect;

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Player {
    id: Uuid,
    username: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Lobby {
    users: Vec<Player>,
}

pub type LobbyStore = Arc<Mutex<HashMap<String, Lobby>>>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing::subscriber::set_global_default(FmtSubscriber::default())?;

    let (layer, io) = SocketIo::builder()
        .with_state(LobbyStore::default())
        .build_layer();

    io.ns("/", on_connect);

    let app = axum::Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .layer(layer);

    info!("Starting server");

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
    Ok(())
}
