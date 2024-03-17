use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use tracing::info;

use crate::{
    game_core::core::{deal_cards, init_playing_phase, Game, Phase, Team},
    AppState, GAME_ID,
};

pub(crate) async fn start_game(app_state: State<AppState>) -> impl IntoResponse {
    let game_id = GAME_ID;

    let game_store = app_state.game_store.clone();

    let game = game_store
        .lock()
        .expect("failed locking game_store")
        .get(game_id)
        .expect("Game should exist at this stage")
        .clone();

    if !validate_teams(&game) {
        return (StatusCode::BAD_REQUEST, "Invalid teams").into_response();
    }

    if deal_cards(game_id, game_store.clone()).is_err() {
        return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to deal cards").into_response();
    }

    let io = app_state.io.clone();
    let game_store = app_state.game_store.clone();
    let mut game = game_store.lock().unwrap().get_mut(game_id).unwrap().clone();

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
        info!("game_phase: {:?}", phase);
        io.to(game_id).emit("game-phase", phase).unwrap();
    }

    start_none_blocking_exchange_loop(game_id.to_string(), app_state.clone());

    (StatusCode::OK, "Game started").into_response()
}

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

        let mut game = game_store
            .lock()
            .unwrap()
            .get_mut(&game_id)
            .unwrap()
            .clone();

        if game.players.values().all(|p| p.exchange.is_some()) {
            let io = app_state.io.clone();
            let phase = Phase::Playing;
            game.phase = Some(phase.clone());
            init_playing_phase(&game_id, game_store);
            io.to(game_id).emit("game-phase", phase).unwrap();
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
    username: String,
    team: Team,
}

//TODO: switch to socket.io
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
            let player = game.players.get_mut(&socket_id).unwrap();
            player.team = Some(team.clone());

            app_state
                .io
                .to(game_id)
                .emit("team-joined", (player.username.clone(), team))
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

            let player = game.players.get_mut(&socket_id).unwrap();

            player.team = Some(team.clone());

            app_state
                .io
                .to(game_id)
                .emit("team-joined", (player.username.clone(), team))
                .unwrap();
        }
    };

    (StatusCode::OK, "Joined team").into_response()
}
