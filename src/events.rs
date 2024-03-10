use serde_json::Value;
use socketioxide::extract::{Data, SocketRef, State};
use tracing::info;

use crate::{
    game_client::client::{connect_lobby, create_lobby},
    game_core::core::GameStore,
};

pub fn on_connect(socket: SocketRef, Data(_): Data<Value>) {
    info!("Socket.IO connected: {:?} {:?}", socket.ns(), socket.id);

    socket.on(
        "connect-lobby",
        |socket: SocketRef, Data::<Value>(data), game_store: State<GameStore>| {
            info!("Connecting to lobby: {:?}", data);
            _ = connect_lobby(socket, data, game_store.clone());
        },
    );

    socket.on(
        "create-lobby",
        |socket: SocketRef, Data::<String>(username), game_store: State<GameStore>| {
            _ = create_lobby(socket, username, game_store.clone());
        },
    );
}
