use anyhow::anyhow;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use anyhow::Context;

use rand::Rng;
use serde::{Deserialize, Serialize};
use socketioxide::socket::Sid;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Game {
    pub game_id: String,
    pub players: HashMap<Sid, Player>,
    pub phase: Option<Phase>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Phase {
    Exchanging,
    Playing,
}

pub type GameStore = Arc<Mutex<HashMap<String, Game>>>;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Player {
    pub socket_id: Sid,
    pub username: String,
    pub hand: Option<Hand>,
    pub team: Option<Team>,
    pub exchange: Option<HashMap<String, Cards>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hand {
    cards: Vec<Cards>,
}

#[derive(Debug, Clone, PartialEq, Ord, PartialOrd, Eq, Deserialize, Serialize)]
pub enum Cards {
    Dog(Card),
    Mahjong(Card),
    Two(Card),
    Three(Card),
    Four(Card),
    Five(Card),
    Six(Card),
    Seven(Card),
    Eight(Card),
    Nine(Card),
    Ten(Card),
    Jack(Card),
    Queen(Card),
    King(Card),
    Ace(Card),
    Phoenix(Card),
    Dragon(Card),
}

#[derive(Debug, Clone, PartialEq, Ord, PartialOrd, Eq, Deserialize, Serialize)]
pub struct Card {
    color: Option<Color>,
}

#[derive(Debug, Clone, PartialEq, Ord, PartialOrd, Eq, Deserialize, Serialize)]
enum Color {
    Black,
    Blue,
    Red,
    Green,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Team {
    One,
    Two,
    Spectator,
}

/*
enum LeadType {
    Single,
    Pair,
    Triple,
    FullHouse,
    Straight,
    Bomb,
}

enum Bomb {
    FourOfAKind,
    straightFlush,
}*/

#[derive(Debug, Clone, Deserialize)]
pub struct Turn {
    pub cards: Vec<Cards>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Exchange {
    pub game_id: String,
    pub player_card: HashMap<String, Cards>,
}

pub fn generate_hands() -> Vec<Hand> {
    let mut deck: Vec<Cards> = Vec::with_capacity(56);
    for color in [Color::Black, Color::Blue, Color::Red, Color::Green] {
        deck.push(Cards::Two(Card {
            color: Some(color.clone()),
        }));
        deck.push(Cards::Three(Card {
            color: Some(color.clone()),
        }));
        deck.push(Cards::Four(Card {
            color: Some(color.clone()),
        }));
        deck.push(Cards::Five(Card {
            color: Some(color.clone()),
        }));
        deck.push(Cards::Six(Card {
            color: Some(color.clone()),
        }));
        deck.push(Cards::Seven(Card {
            color: Some(color.clone()),
        }));
        deck.push(Cards::Eight(Card {
            color: Some(color.clone()),
        }));
        deck.push(Cards::Nine(Card {
            color: Some(color.clone()),
        }));
        deck.push(Cards::Ten(Card {
            color: Some(color.clone()),
        }));
        deck.push(Cards::Jack(Card {
            color: Some(color.clone()),
        }));
        deck.push(Cards::Queen(Card {
            color: Some(color.clone()),
        }));
        deck.push(Cards::King(Card {
            color: Some(color.clone()),
        }));
        deck.push(Cards::Ace(Card {
            color: Some(color.clone()),
        }));
    }
    deck.push(Cards::Dog(Card { color: None }));
    deck.push(Cards::Dragon(Card { color: None }));
    deck.push(Cards::Phoenix(Card { color: None }));
    deck.push(Cards::Mahjong(Card { color: None }));

    let mut hands: Vec<Hand> = Vec::with_capacity(4);

    let mut rng = rand::thread_rng();

    for _ in 0..4 {
        let mut hand: Hand = Hand {
            cards: Vec::with_capacity(14),
        };
        for _ in 0..14 {
            hand.cards.push(deck.remove(rng.gen_range(0..deck.len())));
        }
        hands.push(hand);
    }
    hands
}

pub fn deal_cards(game_id: String, game_store: GameStore) -> anyhow::Result<()> {
    let mut game_lock = game_store.lock().unwrap();
    let game = game_lock
        .get_mut(&game_id)
        .with_context(|| format!("Game {} not found", game_id))?;

    // clear hands
    for player in game.players.values_mut() {
        player.hand = None;
    }
    let hands = generate_hands();

    for (player, hand) in game.players.iter_mut().zip(hands.iter()) {
        player.1.hand = Some(hand.clone());
    }

    Ok(())
}

pub fn validate_exchange(
    player_id: Sid,
    exchange: Exchange,
    game_store: GameStore,
) -> anyhow::Result<Exchange> {
    let game_lock = game_store.lock().unwrap();
    let game = game_lock
        .get(&exchange.game_id)
        .with_context(|| format!("Game {} not found", exchange.game_id))?;

    let player = game
        .players
        .get(&player_id)
        .with_context(|| format!("failed getting player with socket_id {}", player_id))?;

    if exchange.player_card.contains_key(&player.username) {
        info!("cant exchange with yourself");
        return Err(anyhow!("cant exchange with yourself"));
    }

    let mut unique_cards = exchange.player_card.values().cloned().collect::<Vec<_>>();
    unique_cards.sort();
    unique_cards.dedup();

    if unique_cards.len() != 3 {
        info!("failed to exchange cards, must be 3 unique cards");
        return Err(anyhow!("failed to exchange cards"));
    }

    let player_hand = if let Some(hand) = &player.hand {
        hand
    } else {
        info!("failed to exchange cards, player has no hand");
        return Err(anyhow!("failed to exchange cards"));
    };

    if !player_owns_cards(player_hand, unique_cards.as_slice()) {
        info!("failed to exchange cards, player does not own all cards");
        return Err(anyhow!("failed to exchange cards"));
    }

    Ok(exchange)
}

fn player_owns_cards(hand: &Hand, selected_cards: &[Cards]) -> bool {
    selected_cards.iter().all(|card| hand.cards.contains(card))
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_generate_hands() {
        let hands = generate_hands();
        assert_eq!(hands.len(), 4);
        for hand in hands.clone() {
            assert_eq!(hand.cards.len(), 14);
        }
    }

    #[test]
    fn test_deal_cards() {
        let game_store = dummy_game_store();
        let game_id = "test_game".to_string();
        deal_cards(game_id.clone(), game_store.clone()).unwrap();
        let game = game_store.lock().unwrap().get(&game_id).cloned().unwrap();
        for player in game.players.values() {
            assert_eq!(player.hand.as_ref().unwrap().cards.len(), 14);
        }
    }

    #[test]
    fn test_validate_exchange() {
        let game_id = "test_game";
        let game_store = dummy_game_store();

        deal_cards(game_id.to_string(), game_store.clone()).unwrap();

        let game = game_store.lock().unwrap().get(game_id).cloned().unwrap();

        let usernames = ["0", "1", "2", "3"];

        for player in game.players.values() {
            let valid_usernames = usernames
                .iter()
                .filter_map(|u| {
                    if **u != player.username {
                        Some(u.to_string())
                    } else {
                        None
                    }
                })
                .collect::<Vec<String>>();

            let cards = player
                .hand
                .clone()
                .expect("player has no hand")
                .cards
                .iter()
                .take(3)
                .cloned()
                .collect::<Vec<_>>();

            let valid_player_card = valid_usernames
                .iter()
                .cloned()
                .zip(cards.iter().cloned())
                .collect::<HashMap<String, Cards>>();

            let exchange = Exchange {
                game_id: game_id.to_string(),
                player_card: valid_player_card,
            };

            let result = validate_exchange(player.socket_id, exchange, game_store.clone());
            assert!(result.is_ok());

            let identical_cards = [cards[0].clone(), cards[0].clone(), cards[0].clone()];

            let invalid_player_card = valid_usernames
                .iter()
                .cloned()
                .zip(identical_cards)
                .collect::<HashMap<String, Cards>>();

            let invalid_exchange = Exchange {
                game_id: game_id.to_string(),
                player_card: invalid_player_card,
            };

            let result = validate_exchange(player.socket_id, invalid_exchange, game_store.clone());
            assert!(result.is_err());

            let mut invalid_users = valid_usernames.clone();
            invalid_users[0] = player.username.clone();
            let invalid_player_card = invalid_users
                .iter()
                .cloned()
                .zip(cards.iter().cloned())
                .collect::<HashMap<String, Cards>>();

            let invalid_exchange = Exchange {
                game_id: game_id.to_string(),
                player_card: invalid_player_card,
            };

            let result = validate_exchange(player.socket_id, invalid_exchange, game_store.clone());
            assert!(result.is_err());
        }
    }

    fn dummy_game_store() -> GameStore {
        let mut players = HashMap::new();
        for i in 0..4 {
            let socket_id = Sid::new();
            players.insert(
                socket_id,
                Player {
                    socket_id,
                    username: i.to_string(),
                    ..Default::default()
                },
            );
        }
        let mut game_store = HashMap::new();
        game_store.insert(
            "test_game".to_string(),
            Game {
                game_id: "test_game".to_string(),
                players,
                ..Default::default()
            },
        );
        Arc::new(Mutex::new(game_store))
    }
}
