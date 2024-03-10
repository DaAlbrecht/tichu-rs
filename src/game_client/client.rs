use anyhow::Result;
use serde::Deserialize;
use serde_json::Value;
use socketioxide::extract::SocketRef;
use tracing::info;

use crate::{
    game_core::core::{Game, GameStore, Player},
    GAME_ID,
};

#[derive(Debug, Deserialize)]
struct JoinLobbyDto {
    game_id: String,
    username: String,
}

pub fn create_lobby(socket: SocketRef, username: String, game_store: GameStore) -> Result<()> {
    let game_id = GAME_ID.to_string();
    //let game_id = uuid::Uuid::new_v4().to_string();

    let new_player = Player {
        socket_id: socket.id.clone().to_string(),
        username,
        ..Default::default()
    };

    if game_store.lock().unwrap().contains_key(&game_id) {
        info!("Lobby already exists");
        socket.emit("join-lobby", game_id)?;
        return Ok(());
    }
    let mut player_map = std::collections::HashMap::new();

    player_map.insert(new_player.username.clone(), new_player.clone());

    game_store.lock().unwrap().insert(
        game_id.clone(),
        Game {
            game_id: game_id.clone(),
            players: player_map,
            ..Default::default()
        },
    );
    socket.join(game_id.clone())?;
    socket.emit("lobby-created", game_id)?;
    Ok(())
}

pub fn connect_lobby(socket: SocketRef, data: Value, game_store: GameStore) -> Result<()> {
    info!("Connecting to lobby: {:?}", data);
    let data: JoinLobbyDto = serde_json::from_value(data)?;
    let game_id = data.game_id;

    let new_player = Player {
        socket_id: socket.id.clone().to_string(),
        username: data.username,
        ..Default::default()
    };

    if !game_store.lock().unwrap().contains_key(&game_id) {
        info!("Lobby does not exist");
        socket.emit("lobby-not-found", game_id)?;
        return Ok(());
    }

    socket.join(game_id.clone())?;

    game_store
        .lock()
        .unwrap()
        .get_mut(&game_id)
        .unwrap()
        .players
        .insert(new_player.username.clone(), new_player.clone());

    // emit to all users in the new user that joined
    socket
        .to(game_id.clone())
        .emit("user-joined", &new_player.username)
        .expect("Failed to emit");

    //emit to the new user all the users in the lobby
    let game_guard = game_store.lock().unwrap();

    let game = game_guard.get(&game_id);

    if let Some(game) = game {
        let players = game
            .players
            .values()
            .filter(|u| u.socket_id != new_player.socket_id)
            .map(|u| u.username.as_str())
            .collect::<Vec<&str>>();

        socket.emit("users-in-lobby", players)?;
    }

    Ok(())
}
