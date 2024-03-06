use serde_json::Value;
use socketioxide::extract::{Data, SocketRef, State};
use tracing::info;

use crate::{
    game_client::client::{connect_lobby, create_lobby, start_game},
    LobbyStore, Player,
};

pub fn on_connect(socket: SocketRef, Data(_): Data<Value>) {
    info!("Socket.IO connected: {:?} {:?}", socket.ns(), socket.id);

    socket.on(
        "connect-lobby",
        |socket: SocketRef, Data::<Value>(data), lobby_store: State<LobbyStore>| {
            info!("Connecting to lobby: {:?}", data);
            _ = connect_lobby(socket, data, lobby_store.clone());
        },
    );

    socket.on(
        "create-lobby",
        |socket: SocketRef, Data::<Player>(user), lobby_store: State<LobbyStore>| {
            _ = create_lobby(socket, user, lobby_store.clone());
        },
    );

    socket.on(
        "start-game",
        |socket: SocketRef, Data::<String>(data), lobby_store: State<LobbyStore>| {
            start_game(socket, data, lobby_store.clone());
        },
    );
}
