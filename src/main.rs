mod game_client;
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

use axum::routing::get;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use socketioxide::{
    extract::{Data, SocketRef, State},
    SocketIo,
};
use tracing::info;
use tracing_subscriber::FmtSubscriber;
use uuid::Uuid;

use crate::game_client::client::{connect_lobby, create_lobby};

#[derive(Debug, Deserialize, Serialize)]
struct User {
    id: Uuid,
    username: String,
}

pub type RoomStore = Arc<Mutex<HashSet<String>>>;

fn on_connect(socket: SocketRef, Data(data): Data<Value>) {
    info!("Socket.IO connected: {:?} {:?}", socket.ns(), socket.id);
    socket.emit("auth", data).ok();
}

fn on_lobby(socket: SocketRef, Data(_): Data<Value>) {
    info!("Socket.IO connected: {:?} {:?}", socket.ns(), socket.id);

    socket.on(
        "connect-lobby",
        |socket: SocketRef, Data::<Value>(data), store: State<RoomStore>| {
            info!("Connecting to lobby: {:?}", data);
            _ = connect_lobby(socket, data, store.clone());
        },
    );

    socket.on(
        "create-lobby",
        |socket: SocketRef, Data::<User>(user), store: State<RoomStore>| {
            _ = create_lobby(socket, user, store.clone());
        },
    );
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing::subscriber::set_global_default(FmtSubscriber::default())?;

    let (layer, io) = SocketIo::builder()
        .with_state(RoomStore::default())
        .build_layer();

    io.ns("/", on_connect);
    io.ns("/lobby", on_lobby);

    let app = axum::Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .layer(layer);

    info!("Starting server");

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
    Ok(())
}
