use axum::{extract::State, http::StatusCode, response::IntoResponse};
use tracing::info;

use crate::{game_core::core::deal_cards, AppState};

pub(crate) async fn start_game(app_state: State<AppState>) -> impl IntoResponse {
    //temp game_id
    let game_id = "b4a0738b-6be2-4ede-bf37-48e5595f73e1";
    //panic in release mode
    assert!(cfg!(debug_assertions));

    let game_store = app_state.game_store.clone();

    let mut game_lock = game_store.lock().unwrap();

    let game = game_lock
        .get_mut(game_id)
        .expect("If the user is able to start the game, the game should exist");

    let player_count = game.players.len();

    if player_count != 4 {
        return (StatusCode::BAD_REQUEST, "Not enough players").into_response();
    }

    let io = app_state.io.clone();

    //TODO: how to deal with network errors
    while !game.is_running {
        match io.to(game_id).emit("game-started", true) {
            Ok(_) => {
                game.is_running = true;
            }
            Err(_) => continue,
        }
    }
    drop(game_lock);

    while deal_cards(game_id.to_string(), game_store.clone()).is_err() {
        continue;
    }

    info!("game_store: {:?}", game_store);
    (StatusCode::OK, "Game started").into_response()
}

pub(crate) async fn show_hand(app_state: State<AppState>) -> impl IntoResponse {}
