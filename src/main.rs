mod events;
mod game_client;
mod game_core;
mod handlers;

use std::sync::Arc;

use axum::routing::patch;
use socketioxide::SocketIo;
use tower_http::cors::CorsLayer;
use tracing::info;
use tracing_subscriber::FmtSubscriber;

use crate::{events::on_connect, game_core::core::GameStore, handlers::start_game};

struct State {
    io: SocketIo,
    game_store: GameStore,
}

type AppState = Arc<State>;

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
        .route("/start", patch(start_game))
        .route("/join_team", patch(handlers::join_team))
        .with_state(app_state)
        .layer(layer)
        .layer(CorsLayer::permissive());

    info!("Starting server");

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
    Ok(())
}
