use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use anyhow::{anyhow, Result};

use rand::Rng;
use serde::{Deserialize, Serialize};
use tracing::info;

use super::handler::Exchange;

pub struct Game {
    pub game_id: String,
    pub players: Vec<Player>,
}

pub type GameStore = Arc<Mutex<HashMap<String, Game>>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub socket_id: String,
    pub username: String,
    pub hand: Option<Hand>,
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

pub fn deal_cards(game_id: String, game_store: GameStore) {
    let mut game_lock = game_store.lock().unwrap();
    let game = game_lock.get_mut(&game_id).unwrap();
    let hands = generate_hands();
    for (i, player) in game.players.iter_mut().enumerate() {
        player.hand = Some(hands[i].clone());
    }
}

pub fn validate_exchange(player: Player, exchange: Exchange) -> Result<Exchange> {
    if exchange.player_card.contains_key(&player.socket_id) {
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

    if !player_owns_cards(&player, unique_cards.as_slice()) {
        info!("failed to exchange cards, player does not own all cards");
        return Err(anyhow!("failed to exchange cards"));
    }

    Ok(exchange)
}

fn player_owns_cards(player: &Player, cards: &[Cards]) -> bool {
    cards.iter().all(|card| {
        player
            .hand
            .as_ref()
            .expect("player should always have cards at this stage")
            .cards
            .contains(card)
    })
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
}
