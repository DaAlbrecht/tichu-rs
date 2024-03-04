use anyhow::Result;
use serde::Deserialize;
use serde_json::Value;
use socketioxide::extract::SocketRef;
use tracing::info;

use crate::{RoomStore, User};

#[derive(Debug, Deserialize)]
struct TempDto {
    user: User,
    room_id: String,
}

pub fn create_lobby(socket: SocketRef, user: User, store: RoomStore) -> Result<()> {
    info!("Creating lobby: {:?}", user);
    let room_id = user.id.to_string();

    if store.lock().unwrap().contains(&room_id) {
        info!("Lobby already exists");
        socket.emit("join-lobby", Value::String(room_id))?;
        return Ok(());
    }

    store.lock().unwrap().insert(room_id.clone());
    socket.join(room_id.clone())?;
    socket.emit("lobby-created", Value::String(room_id))?;
    Ok(())
}

pub fn connect_lobby(socket: SocketRef, data: Value, store: RoomStore) -> Result<()> {
    info!("Connecting to lobby: {:?}", data);
    let data: TempDto = serde_json::from_value(data)?;
    let lobby_id = data.room_id;

    if !store.lock().unwrap().contains(&lobby_id) {
        info!("Lobby does not exist");
        socket.emit("lobby-not-found", Value::String(lobby_id))?;
        return Ok(());
    }

    socket.join(lobby_id.clone())?;
    socket
        .to(lobby_id)
        .emit("user-joined", data.user)
        .expect("Failed to emit");

    Ok(())
}
