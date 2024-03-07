use anyhow::Result;
use serde::Deserialize;
use serde_json::Value;
use socketioxide::extract::SocketRef;
use tracing::info;

use crate::{Lobby, LobbyStore, User};

#[derive(Debug, Deserialize)]
struct TempDto {
    user: User,
    room_id: String,
}

pub fn create_lobby(socket: SocketRef, user: User, lobby_store: LobbyStore) -> Result<()> {
    info!("Creating lobby: {:?}", user);
    let room_id = user.id.to_string();

    if lobby_store.lock().unwrap().contains_key(&room_id) {
        info!("Lobby already exists");
        socket.emit("join-lobby", Value::String(room_id))?;
        return Ok(());
    }

    lobby_store.lock().unwrap().insert(
        room_id.clone(),
        Lobby {
            users: vec![user.clone()],
        },
    );
    socket.join(room_id.clone())?;
    socket.emit("lobby-created", Value::String(room_id))?;
    Ok(())
}

pub fn connect_lobby(socket: SocketRef, data: Value, lobby_store: LobbyStore) -> Result<()> {
    info!("Connecting to lobby: {:?}", data);
    let data: TempDto = serde_json::from_value(data)?;
    let lobby_id = data.room_id;

    if !lobby_store.lock().unwrap().contains_key(&lobby_id) {
        info!("Lobby does not exist");
        socket.emit("lobby-not-found", Value::String(lobby_id))?;
        return Ok(());
    }

    socket.join(lobby_id.clone())?;

    lobby_store
        .lock()
        .unwrap()
        .get_mut(&lobby_id)
        .unwrap()
        .users
        .push(data.user.clone());

    socket
        .to(lobby_id.clone())
        .emit("user-joined", data.user.clone())
        .expect("Failed to emit");

    //emit new user to all users in the lobby
    let lobby_guard = lobby_store.lock().unwrap();

    let lobby = lobby_guard.get(&lobby_id);

    if let Some(lobby) = lobby {
        let users = lobby
            .users
            .iter()
            .filter(|u| u.id != data.user.id)
            .map(|u| serde_json::to_value(u).unwrap())
            .collect::<Vec<Value>>();

        socket.emit("users-in-lobby", Value::Array(users))?;
    }

    Ok(())
}

pub fn start_game(socket: SocketRef, lobby_id: String, lobby_store: LobbyStore) {
    info!("starting game for id : {:?}", lobby_id);
    lobby_store
        .lock()
        .unwrap()
        .contains_key(&lobby_id)
        .then(|| {
            socket
                .to(lobby_id.clone())
                .emit("game-started", Value::String(lobby_id))
        });
}
