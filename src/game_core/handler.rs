use std::collections::HashMap;

use serde::Deserialize;
use socketioxide::extract::SocketRef;
use tracing::info;

use super::core::{Cards, GameStore};

#[derive(Debug, Clone, Deserialize)]
pub struct Exchange {
    pub player_id: String,
    pub player_card: HashMap<String, Cards>,
}

pub fn exchange_cards(socket: SocketRef, exchange: Exchange) {}

pub fn show_cards(socket: SocketRef, game_id: String, game_store: GameStore) {
    let game_lock = game_store.lock().unwrap();
    let game = game_lock.get(&game_id).unwrap();
    let player = game
        .players
        .iter()
        .find(|p| p.socket_id == socket.id.clone().to_string());

    if player.is_none() {
        info!("Player not found");
        return;
    }

    let player = player.unwrap();

    _ = socket.emit("show-cards", player.hand.as_ref().unwrap());
}
