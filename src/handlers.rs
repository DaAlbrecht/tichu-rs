use std::task::Wake;

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use tracing::info;

use crate::{
    game_core::core::{Game, Phase, Team},
    AppState,
};

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartGameBody {
    game_id: String,
}
pub(crate) async fn start_game(
    app_state: State<AppState>,
    Json(game_id): Json<StartGameBody>,
) -> impl IntoResponse {
    let game_store = app_state.game_store.clone();

    let mut guard = game_store.lock().unwrap();
    let game = guard
        .get_mut(&game_id.game_id)
        .expect("Game should exist at this stage");

    if !validate_teams(game) {
        return (StatusCode::BAD_REQUEST, "Invalid teams").into_response();
    }

    game.deal_cards();

    let io = app_state.io.clone();

    game.players
        .values()
        .for_each(|player| match io.get_socket(player.socket_id) {
            Some(socket) => {
                socket.emit("hand", player.hand.clone().unwrap()).unwrap();
            }
            None => {
                //TODO: what to do here?
                panic!("socket not found");
            }
        });

    if game.phase.is_none() {
        let phase = Phase::Exchanging;
        game.phase = Some(phase.clone());
        io.to(game_id.game_id.clone())
            .emit("game-phase", phase)
            .unwrap();
    }
    drop(guard);

    //start_none_blocking_exchange_loop(game_id.game_id.to_string(), app_state.clone());
    skip_exchange(game_id.game_id.to_string(), app_state.clone());

    (StatusCode::OK, "Game started").into_response()
}

fn skip_exchange(game_id: String, app_state: State<AppState>) {
    let game_store = app_state.game_store.clone();
    let mut guard = game_store.lock().unwrap();
    let game = guard.get_mut(&game_id).unwrap();
    game.start().expect("Game should start");

    let io = app_state.io.clone();
    let phase = Phase::Playing;

    let player_turn = game.round.as_ref().unwrap().current_player;

    let player_position = game
        .players
        .values()
        .find(|p| p.socket_id == player_turn)
        .unwrap()
        .place;

    game.phase = Some(phase.clone());

    io.to(game_id.clone()).emit("game-phase", phase).unwrap();
    io.to(game_id.clone()).emit("started", "").unwrap();
    io.to(game_id).emit("next-player", player_position).unwrap();
}

//TODO: refactor this nonsense
fn start_none_blocking_exchange_loop(game_id: String, app_state: State<AppState>) {
    let mut max_time = 2;
    let game_store = app_state.game_store.clone();
    std::thread::spawn(move || loop {
        info!("waiting for players to exchange");
        std::thread::sleep(std::time::Duration::from_secs(1));

        max_time -= 1;

        if max_time == 0 {
            let io = app_state.io.clone();
            io.to(game_id).emit("disconnect", "timeout").unwrap();
            break;
        }

        let players = {
            let guard = game_store.lock().unwrap();
            let game = guard.get(&game_id).unwrap();
            game.players.clone()
        };

        if players.values().all(|p| p.exchange.is_some()) {
            let mut guard = game_store.lock().unwrap();
            let game = guard.get_mut(&game_id).unwrap();

            game.start().expect("Game should start");

            let io = app_state.io.clone();
            let phase = Phase::Playing;

            let player_turn = game.round.as_ref().unwrap().current_player;

            game.phase = Some(phase.clone());

            io.to(game_id.clone()).emit("game-phase", phase).unwrap();
            info!(
                "username: {:?}",
                players.get(&player_turn).unwrap().username
            );
            io.to(game_id).emit("next-player", player_turn).unwrap();
            break;
        }
    });
}
fn validate_teams(game: &Game) -> bool {
    let player_count = game.players.len();

    if player_count != 4 {
        return false;
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
        return false;
    }
    true
}

#[derive(serde::Deserialize)]
pub(crate) struct JoinTeamBody {
    game_id: String,
    username: String,
    team: Team,
}

//TODO: switch to socket.io
pub(crate) async fn join_team(
    app_state: State<AppState>,
    Json(body): Json<JoinTeamBody>,
) -> impl IntoResponse {
    let game_id = body.game_id;
    let game_store = app_state.game_store.clone();
    let mut game_lock = game_store.lock().unwrap();
    let team = body.team;

    let game = game_lock
        .get_mut(&game_id)
        .expect("Game should exist at this stage");

    //testing purposes
    let socket_id = game
        .players
        .values()
        .find(|k| k.username == body.username)
        .unwrap()
        .socket_id;

    if let Some(player) = game.players.get(&socket_id) {
        if player.team == Some(team.clone()) {
            return (StatusCode::BAD_REQUEST, "Player already in team").into_response();
        }
    } else {
        return (StatusCode::BAD_REQUEST, "Player not found").into_response();
    }

    //unwraps are safe because we have already checked if the player exists
    match team {
        Team::Spectator => {
            let username = game
                .join_team(socket_id, team.clone())
                .expect("Player should join team");

            app_state
                .io
                .to(game_id)
                .emit("team-joined", (username, team))
                .unwrap();
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

            let username = game
                .join_team(socket_id, team.clone())
                .expect("Player should join team");

            app_state
                .io
                .to(game_id)
                .emit("team-joined", (username, team))
                .unwrap();
        }
    };

    (StatusCode::OK, "Joined team").into_response()
}
