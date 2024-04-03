use serde_json::Value;
use socketioxide::extract::{Data, SocketRef, State};
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
