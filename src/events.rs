use serde_json::Value;
use socketioxide::{
    extract::{Data, SocketRef, State},
    socket::Sid,
};
use tracing::info;

use crate::{
    game_client::client::{connect_lobby, create_lobby},
    game_core::core::{Action, Cards, GameStore, Turn},
};

pub fn on_connect(socket: SocketRef, Data(_): Data<Value>) {
    info!("Socket.IO connected: {:?} {:?}", socket.ns(), socket.id);

    socket.on(
        "connect-lobby",
        |socket: SocketRef, Data::<Value>(data), game_store: State<GameStore>| {
            info!("Connecting to lobby: {:?}", data);
            _ = connect_lobby(socket, data, game_store.clone());
        },
    );

    socket.on(
        "create-lobby",
        |socket: SocketRef, Data::<String>(username), game_store: State<GameStore>| {
            _ = create_lobby(socket, username, game_store.clone());
        },
    );

    socket.on(
        "player-swap-team",
        |socket: SocketRef,
         Data::<PlayerSwapTeam>(player_swap_team),
         game_store: State<GameStore>| {
            info!("Swapping team: {:?}", player_swap_team);
            let game_id = player_swap_team.game_id;
            let game_store = game_store.clone();
            let ((team_player1, position_p1), (team_player2, position_p2)) = {
                let guard = game_store.lock().unwrap();
                let game = guard.get(&game_id).unwrap();
                let player1 = game
                    .players
                    .get(&player_swap_team.player1)
                    .expect("Player 1 not found");
                let player2 = game
                    .players
                    .get(&player_swap_team.player2)
                    .expect("Player 2 not found");

                let position_1 = player1.place;
                let position_2 = player2.place;
                (
                    (player1.team.clone(), position_1),
                    (player2.team.clone(), position_2),
                )
            };

            let mut guard = game_store.lock().unwrap();
            let game = guard.get_mut(&game_id).unwrap();
            game.players
                .get_mut(&player_swap_team.player1)
                .unwrap()
                .team = team_player2;
            game.players
                .get_mut(&player_swap_team.player2)
                .unwrap()
                .team = team_player1;

            game.players
                .get_mut(&player_swap_team.player1)
                .unwrap()
                .place = position_p2;
            game.players
                .get_mut(&player_swap_team.player2)
                .unwrap()
                .place = position_p1;

            let players = game.players.values().cloned().collect::<Vec<_>>();
            socket.emit("users-in-lobby", players).unwrap();
        },
    );

    socket.on(
        "play-turn",
        |socket: SocketRef, Data::<PlayTurn>(playturn), game_store: State<GameStore>| {
            info!("Playing turn: {:?}", playturn);
            let game_id = playturn.game_id;
            let game_store = game_store.clone();
            let mut guard = game_store.lock().unwrap();
            let game = guard.get_mut(&game_id).unwrap();

            let turn = Turn {
                player: socket.id,
                action: Action::Play,
                cards: Some(playturn.cards),
            };

            match game.play_turn(turn) {
                Ok(_) => {
                    //handle round end
                    socket
                        .emit(
                            "trick-played",
                            game.round.as_ref().unwrap().current_trick.clone(),
                        )
                        .unwrap();
                    let next_player = game.round.as_ref().unwrap().current_player;
                    socket.emit("next-player", next_player).unwrap();
                    info!(
                        "Next user: {:?}",
                        game.players.get(&next_player).unwrap().username
                    );
                }
                Err(err) => {
                    socket.emit("trick-error", format!("{}", err)).unwrap();
                }
            }
        },
    );
}

#[derive(Debug, serde::Deserialize)]
struct PlayTurn {
    game_id: String,
    cards: Vec<Cards>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlayerSwapTeam {
    game_id: String,
    player1: Sid,
    player2: Sid,
}
