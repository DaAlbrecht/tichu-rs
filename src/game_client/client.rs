use anyhow::Result;
use serde::Deserialize;
use serde_json::Value;
use socketioxide::extract::SocketRef;
use tracing::info;

use crate::game_core::core::{Game, GameStore, Player};

#[derive(Debug, Deserialize)]
struct JoinLobbyDto {
    game_id: String,
    username: String,
}

pub fn create_lobby(socket: SocketRef, username: String, game_store: GameStore) -> Result<()> {
    //in debug mode use GAME_ID to test otherwise generate a new game_id
    let game_id = uuid::Uuid::new_v4().to_string();

    let new_player = Player {
        socket_id: socket.id,
        username,
        is_host: true,
        ..Default::default()
    };

    if game_store.lock().unwrap().contains_key(&game_id) {
        info!("Lobby already exists");
        socket.emit("join-lobby", game_id)?;
        return Ok(());
    }
    let mut player_map = std::collections::HashMap::new();

    player_map.insert(socket.id, new_player.clone());

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

    if !game_store.lock().unwrap().contains_key(&game_id) {
        info!("Lobby does not exist");
        socket.emit("lobby-not-found", game_id)?;
        return Ok(());
    }

    socket.join(game_id.clone())?;

    let player_count = game_store
        .lock()
        .unwrap()
        .get(&game_id)
        .unwrap()
        .players
        .len() as u8;

    let new_player = Player {
        socket_id: socket.id,
        username: data.username,
        place: player_count + 1,
        ..Default::default()
    };
    game_store
        .lock()
        .unwrap()
        .get_mut(&game_id)
        .unwrap()
        .players
        .insert(socket.id, new_player.clone());

    // emit to all users in the new user that joined
    socket
        .to(game_id.clone())
        .emit("user-joined", &new_player)
        .expect("Failed to emit");

    //emit to the new user all the users in the lobby
    let game_guard = game_store.lock().unwrap();

    let game = game_guard.get(&game_id);

    if let Some(game) = game {
        let players = game.players.values().collect::<Vec<_>>();

        socket.emit("users-in-lobby", players)?;
    }

    Ok(())
}
