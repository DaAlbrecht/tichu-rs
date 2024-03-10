use std::collections::HashMap;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use tracing::info;

use crate::{
    game_core::core::{deal_cards, Team},
    AppState, GAME_ID,
};

pub(crate) async fn start_game(app_state: State<AppState>) -> impl IntoResponse {
    let game_id = GAME_ID;

    let game_store = app_state.game_store.clone();

    let mut game_lock = game_store.lock().unwrap();

    let game = game_lock
        .get_mut(game_id)
        .expect("If the user is able to start the game, the game should exist");

    let player_count = game.players.len();

    if player_count != 4 {
        return (StatusCode::BAD_REQUEST, "Not enough players").into_response();
    }

    let team_1_count = game
        .players
        .values()
        .filter(|p| p.team == Some(Team::One))
        .count();

    let team_2_count = game
        .players
        .values()
        .filter(|p| p.team == Some(Team::Two))
        .count();

    if team_1_count != 2 || team_2_count != 2 {
        return (
            StatusCode::BAD_REQUEST,
            "Teams are not balanced, both teams need two players",
        )
            .into_response();
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

    //TODO: iterate over players and send them their hands as an event

    info!("game_store: {:?}", game_store);
    (StatusCode::OK, "Game started").into_response()
}

pub(crate) async fn show_hand(
    app_state: State<AppState>,
    Query(username): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let game_id = GAME_ID;
    let game_store = app_state.game_store.clone();
    let game_lock = game_store.lock().unwrap();

    let username = if let Some(username) = username.get("username") {
        username
    } else {
        return (StatusCode::BAD_REQUEST, "Username not found").into_response();
    };

    let game = if let Some(game) = game_lock.get(game_id) {
        game
    } else {
        return (StatusCode::BAD_REQUEST, "Game not found").into_response();
    };

    let player = if let Some(player) = game.players.get(username) {
        player
    } else {
        //this should not really happen
        return (StatusCode::BAD_REQUEST, "Player not found").into_response();
    };

    (
        StatusCode::OK,
        Json(
            player
                .hand
                .clone()
                .expect("player always has a hand at this stage"),
        ),
    )
        .into_response()
}

#[derive(serde::Deserialize)]
pub(crate) struct JoinTeamBody {
    username: String,
    team: Team,
}

pub(crate) async fn join_team(
    app_state: State<AppState>,
    Json(body): Json<JoinTeamBody>,
) -> impl IntoResponse {
    let game_id = GAME_ID;
    let game_store = app_state.game_store.clone();
    let mut game_lock = game_store.lock().unwrap();
    let team = body.team;

    let game = game_lock
        .get_mut(game_id)
        .expect("Game should exist at this stage");

    if let Some(player) = game.players.get(&body.username) {
        if player.team == Some(team.clone()) {
            return (StatusCode::BAD_REQUEST, "Player already in team").into_response();
        }
    } else {
        return (StatusCode::BAD_REQUEST, "Player not found").into_response();
    }

    //unwraps are safe because we have already checked if the player exists
    match team {
        Team::Spectator => {
            let player = game.players.get_mut(&body.username).unwrap();
            player.team = Some(team);
        }
        _ => {
            let team_count = game
                .players
                .values()
                .filter(|p| p.team == Some(team.clone()))
                .count();

            if team_count >= 2 {
                return (StatusCode::BAD_REQUEST, "Team is full").into_response();
            }

            let player = game.players.get_mut(&body.username).unwrap();

            player.team = Some(team);
        }
    };

    (StatusCode::OK, "Joined team").into_response()
}

pub(crate) async fn declare_exchange(app_state: State<AppState>) -> impl IntoResponse {
    todo!()
}
