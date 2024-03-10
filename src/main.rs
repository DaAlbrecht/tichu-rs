mod events;
mod game_client;
mod game_core;
mod handlers;

use std::sync::Arc;

use axum::routing::{get, patch};
use socketioxide::SocketIo;
use tracing::info;
use tracing_subscriber::FmtSubscriber;

use crate::{events::on_connect, game_core::core::GameStore, handlers::start_game};

struct State {
    io: SocketIo,
    game_store: GameStore,
}

type AppState = Arc<State>;

pub static GAME_ID: &str = if cfg!(debug_assertions) {
    "b4a0738b-6be2-4ede-bf37-48e5595f73e1"
} else {
    panic!("Game id should be set in debug mode");
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing::subscriber::set_global_default(FmtSubscriber::default())?;
    let game_store = GameStore::default();

    let (layer, io) = SocketIo::builder()
        .with_state(game_store.clone())
        .build_layer();

    io.ns("/", on_connect);

    let app_state: AppState = Arc::new(State { io, game_store });

    //TODO: map / protect requests -> users -> sockets.id
    let app = axum::Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route("/start", patch(start_game))
        .route("/show_hand", get(handlers::show_hand))
        .route("/join_team", patch(handlers::join_team))
        .route("/declare_exchange", patch(handlers::declare_exchange))
        .with_state(app_state)
        .layer(layer);

    info!("Starting server");

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
    Ok(())
}
