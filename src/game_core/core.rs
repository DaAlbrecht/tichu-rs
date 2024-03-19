use anyhow::anyhow;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    u8, usize,
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
    pub score_t1: u16,
    pub score_t2: u16,
    pub player_turn_iterator: Option<TurnIterator>,
    pub current_trick: Vec<Cards>,
    pub current_trick_type: Option<TrickType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnIterator {
    pub turn_sequence: HashMap<Sid, Sid>,
    pub current_player: Sid,
}

impl From<Vec<Sid>> for TurnIterator {
    fn from(players: Vec<Sid>) -> Self {
        let mut turn_sequence = HashMap::new();
        let mut previous_player = players.last().unwrap();
        let first_player = players.first().unwrap();
        for current_player in players.iter() {
            turn_sequence.insert(*previous_player, *current_player);
            previous_player = current_player;
        }
        TurnIterator {
            turn_sequence,
            current_player: *first_player,
        }
    }
}

impl Iterator for TurnIterator {
    type Item = Sid;

    fn next(&mut self) -> Option<Self::Item> {
        let next_player = self.turn_sequence.get(&self.current_player);
        if let Some(player) = next_player {
            self.current_player = *player;
            Some(*player)
        } else {
            None
        }
    }
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
    Dog,
    Mahjong(Box<Mahjong>),
    Two(Color),
    Three(Color),
    Four(Color),
    Five(Color),
    Six(Color),
    Seven(Color),
    Eight(Color),
    Nine(Color),
    Ten(Color),
    Jack(Color),
    Queen(Color),
    King(Color),
    Ace(Color),
    Phoenix(Box<Phoenix>),
    Dragon,
}

#[derive(Debug, Clone, PartialEq, Ord, PartialOrd, Eq, Deserialize, Serialize)]
pub struct Mahjong {
    pub wish: Option<Cards>,
}

#[derive(Debug, Clone, PartialEq, Ord, PartialOrd, Eq, Deserialize, Serialize)]
pub struct Phoenix {
    pub value: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Ord, PartialOrd, Eq, Deserialize, Serialize)]
pub enum Color {
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

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub enum TrickType {
    Single,
    Pair,
    Triple,
    FullHouse,
    Straight,
    SequenceOfPairs,
    FourOfAKind,
    StraightFlush,
}

impl Cards {
    fn get_card_digit(&self) -> Option<u8> {
        match self {
            Cards::Two(_) => Some(2),
            Cards::Three(_) => Some(3),
            Cards::Four(_) => Some(4),
            Cards::Five(_) => Some(5),
            Cards::Six(_) => Some(6),
            Cards::Seven(_) => Some(7),
            Cards::Eight(_) => Some(8),
            Cards::Nine(_) => Some(9),
            Cards::Ten(_) => Some(10),
            Cards::Jack(_) => Some(11),
            Cards::Queen(_) => Some(12),
            Cards::King(_) => Some(13),
            Cards::Ace(_) => Some(14),
            Cards::Phoenix(p) => p.value,
            _ => None,
        }
    }
    fn get_color(&self) -> Option<Color> {
        match self {
            Cards::Two(c) => Some(c.clone()),
            Cards::Three(c) => Some(c.clone()),
            Cards::Four(c) => Some(c.clone()),
            Cards::Five(c) => Some(c.clone()),
            Cards::Six(c) => Some(c.clone()),
            Cards::Seven(c) => Some(c.clone()),
            Cards::Eight(c) => Some(c.clone()),
            Cards::Nine(c) => Some(c.clone()),
            Cards::Ten(c) => Some(c.clone()),
            Cards::Jack(c) => Some(c.clone()),
            Cards::Queen(c) => Some(c.clone()),
            Cards::King(c) => Some(c.clone()),
            Cards::Ace(c) => Some(c.clone()),
            _ => None,
        }
    }
}

impl TryFrom<Vec<Cards>> for TrickType {
    type Error = anyhow::Error;

    fn try_from(cards: Vec<Cards>) -> anyhow::Result<Self> {
        fn all_equal(cards: &[Cards]) -> bool {
            let mut card_types = cards
                .iter()
                .filter_map(|c| c.get_card_digit())
                .collect::<Vec<u8>>();
            card_types.sort();
            card_types.dedup();
            card_types.len() == 1
        }

        fn is_sequence(cards: &[Cards]) -> bool {
            let mut card_digits = cards
                .iter()
                .filter_map(|c| c.get_card_digit())
                .collect::<Vec<u8>>();
            card_digits.sort();
            card_digits.windows(2).all(|w| w[0] + 1 == w[1])
        }

        fn is_sequence_of_pairs(cards: &[Cards]) -> bool {
            let mut card_digits = cards
                .iter()
                .filter_map(|c| c.get_card_digit())
                .collect::<Vec<u8>>();
            card_digits.sort();
            card_digits
                .windows(4)
                .all(|w| w[0] == w[1] && w[2] == w[3] && w[0] + 1 == w[2])
        }

        fn is_full_house(cards: &[Cards]) -> bool {
            let mut card_types = cards
                .iter()
                .filter_map(|c| c.get_card_digit())
                .collect::<Vec<u8>>();
            card_types.sort();
            card_types.dedup();

            if card_types.len() != 2 {
                return false;
            }

            let types_count = card_types
                .iter()
                .map(|t| card_types.iter().filter(|&c| c == t).count())
                .collect::<Vec<usize>>();

            matches!(types_count.as_slice(), [2, 3] | [3, 2])
        }

        match cards.len() {
            1 => Ok(TrickType::Single),
            2 if all_equal(&cards) => Ok(TrickType::Pair),
            3 if all_equal(&cards) => Ok(TrickType::Triple),
            4 if all_equal(&cards) => Ok(TrickType::FourOfAKind),
            5 if is_full_house(&cards) => Ok(TrickType::FullHouse),
            4..=14 if is_sequence_of_pairs(&cards) => Ok(TrickType::SequenceOfPairs),
            5..=14 if is_sequence(&cards) => {
                if cards.iter().filter_map(|c| c.get_color()).count() == 1 {
                    Ok(TrickType::StraightFlush)
                } else {
                    Ok(TrickType::Straight)
                }
            }
            _ => Err(anyhow!("invalid trick")),
        }
    }
}

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
        deck.push(Cards::Two(color.clone()));
        deck.push(Cards::Three(color.clone()));
        deck.push(Cards::Four(color.clone()));
        deck.push(Cards::Five(color.clone()));
        deck.push(Cards::Six(color.clone()));
        deck.push(Cards::Seven(color.clone()));
        deck.push(Cards::Eight(color.clone()));
        deck.push(Cards::Nine(color.clone()));
        deck.push(Cards::Ten(color.clone()));
        deck.push(Cards::Jack(color.clone()));
        deck.push(Cards::Queen(color.clone()));
        deck.push(Cards::King(color.clone()));
        deck.push(Cards::Ace(color.clone()));
    }
    deck.push(Cards::Phoenix(Box::new(Phoenix { value: None })));
    deck.push(Cards::Mahjong(Box::new(Mahjong { wish: None })));
    deck.push(Cards::Dragon);
    deck.push(Cards::Dog);

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

pub fn deal_cards(game_id: &str, game_store: GameStore) -> anyhow::Result<()> {
    let mut game_lock = game_store.lock().unwrap();
    let game = game_lock
        .get_mut(game_id)
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

pub fn init_playing_phase(game_id: &str, game_store: GameStore) {
    let mut game_lock = game_store.lock().unwrap();
    let game = game_lock.get_mut(game_id).unwrap();

    let team_1 = game.players.values().filter(|p| p.team == Some(Team::One));

    let team_2 = game.players.values().filter(|p| p.team == Some(Team::Two));

    let turns = team_1
        .zip(team_2)
        .flat_map(|(p1, p2)| vec![p1.socket_id, p2.socket_id])
        .collect::<Vec<_>>();

    let turn_iterator = TurnIterator::from(turns);
    game.player_turn_iterator = Some(turn_iterator);

    let player_with_mahjong = game
        .players
        .iter()
        .find(|(_, p)| {
            if let Some(hand) = &p.hand {
                hand.cards.iter().any(|c| matches!(c, Cards::Mahjong(_)))
            } else {
                false
            }
        })
        .map(|(sid, _)| sid)
        .expect("one player should have mahjong");

    game.player_turn_iterator
        .as_mut()
        .expect("failed getting turn iterator")
        .current_player = *player_with_mahjong;
}

fn new_round(game_id: &str, trick: &[Cards], game_store: GameStore) -> anyhow::Result<()> {
    let playing_player = {
        let guard = game_store.lock().unwrap();
        let game = guard.get(game_id).unwrap().clone();
        game.player_turn_iterator.unwrap().current_player
    };

    todo!()
}

#[cfg(test)]
mod tests {

    use super::*;

    fn dummy_game_store() -> GameStore {
        let mut players = HashMap::new();
        for i in 0..4 {
            let socket_id = Sid::new();
            if i < 2 {
                players.insert(
                    socket_id,
                    Player {
                        socket_id,
                        username: i.to_string(),
                        team: Some(Team::One),
                        ..Default::default()
                    },
                );
                continue;
            } else {
                players.insert(
                    socket_id,
                    Player {
                        socket_id,
                        username: i.to_string(),
                        team: Some(Team::Two),
                        ..Default::default()
                    },
                );
            }
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
        let game_id = "test_game";
        deal_cards(game_id, game_store.clone()).unwrap();
        let game = game_store.lock().unwrap().get(game_id).cloned().unwrap();
        for player in game.players.values() {
            assert_eq!(player.hand.as_ref().unwrap().cards.len(), 14);
        }
    }

    #[test]
    fn test_validate_exchange() {
        let game_id = "test_game";
        let game_store = dummy_game_store();

        deal_cards(game_id, game_store.clone()).unwrap();

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

    #[test]
    fn test_turns() {
        let game_id = "test_game";
        let game_store = dummy_game_store();
        deal_cards(game_id, game_store.clone()).unwrap();
        init_playing_phase(game_id, game_store.clone());

        let game = game_store.lock().unwrap().get(game_id).cloned().unwrap();

        assert_eq!(game.player_turn_iterator.is_some(), true);

        let mut turn_iterator = game.player_turn_iterator.unwrap();

        for _ in 0..4 {
            let turn = turn_iterator.next();
            assert!(turn.is_some());
        }
    }

    #[test]
    fn test_alternating_teams() {
        let game_id = "test_game";
        let game_store = dummy_game_store();
        deal_cards(game_id, game_store.clone()).unwrap();
        init_playing_phase(game_id, game_store.clone());

        let game = game_store.lock().unwrap().get(game_id).cloned().unwrap();

        assert_eq!(game.player_turn_iterator.is_some(), true);

        let turn_sequence = game.player_turn_iterator.unwrap().turn_sequence;

        for (previous, current) in turn_sequence.iter() {
            let prev = previous.clone();
            let curr = current.clone();
            let team_previous = game.players.get(&prev).unwrap().team.clone();
            let team_current = game.players.get(&curr).unwrap().team.clone();
            assert_ne!(team_previous, team_current);
        }
    }

    #[test]
    fn test_starting_player() {
        let game_id = "test_game";
        let game_store = dummy_game_store();
        deal_cards(game_id, game_store.clone()).unwrap();
        init_playing_phase(game_id, game_store.clone());

        let game = game_store.lock().unwrap().get(game_id).cloned().unwrap();

        assert_eq!(game.player_turn_iterator.is_some(), true);

        let players_turn = game.player_turn_iterator.unwrap().current_player;

        let player_has_mahjong = game
            .players
            .get(&players_turn)
            .unwrap()
            .hand
            .as_ref()
            .unwrap()
            .cards
            .iter()
            .any(|c| matches!(c, Cards::Mahjong(_)));

        assert_eq!(player_has_mahjong, true);

        for player in game.players.values() {
            if player.socket_id != players_turn {
                let player_has_mahjong = player
                    .hand
                    .as_ref()
                    .unwrap()
                    .cards
                    .iter()
                    .any(|c| matches!(c, Cards::Mahjong(_)));
                assert_eq!(player_has_mahjong, false);
            }
        }
    }

    #[test]
    fn test_single_trick() {
        let single_trick_tests = vec![
            (vec![Cards::Two(Color::Black)], TrickType::Single),
            (vec![Cards::Three(Color::Black)], TrickType::Single),
            (vec![Cards::Four(Color::Black)], TrickType::Single),
            (vec![Cards::Five(Color::Black)], TrickType::Single),
            (vec![Cards::Six(Color::Black)], TrickType::Single),
            (vec![Cards::Seven(Color::Black)], TrickType::Single),
            (vec![Cards::Eight(Color::Black)], TrickType::Single),
            (vec![Cards::Nine(Color::Black)], TrickType::Single),
            (vec![Cards::Ten(Color::Black)], TrickType::Single),
            (vec![Cards::Jack(Color::Black)], TrickType::Single),
            (vec![Cards::Queen(Color::Black)], TrickType::Single),
            (vec![Cards::King(Color::Black)], TrickType::Single),
            (vec![Cards::Ace(Color::Black)], TrickType::Single),
            (
                vec![Cards::Phoenix(Box::new(Phoenix { value: Some(1) }))],
                TrickType::Single,
            ),
            (vec![Cards::Dragon], TrickType::Single),
            (vec![Cards::Dog], TrickType::Single),
        ];
        single_trick_tests.iter().for_each(|(cards, expected)| {
            assert_eq!(TrickType::try_from(cards.clone()).unwrap(), *expected)
        });
    }

    #[test]
    fn test_pair_trick() {
        let pair_trick_tests = vec![
            (
                vec![Cards::Two(Color::Black), Cards::Two(Color::Blue)],
                TrickType::Pair,
            ),
            (
                vec![Cards::Three(Color::Black), Cards::Three(Color::Blue)],
                TrickType::Pair,
            ),
            (
                vec![Cards::Four(Color::Black), Cards::Four(Color::Blue)],
                TrickType::Pair,
            ),
            (
                vec![Cards::Five(Color::Black), Cards::Five(Color::Blue)],
                TrickType::Pair,
            ),
            (
                vec![Cards::Six(Color::Black), Cards::Six(Color::Blue)],
                TrickType::Pair,
            ),
            (
                vec![Cards::Seven(Color::Black), Cards::Seven(Color::Blue)],
                TrickType::Pair,
            ),
            (
                vec![Cards::Eight(Color::Black), Cards::Eight(Color::Blue)],
                TrickType::Pair,
            ),
            (
                vec![Cards::Nine(Color::Black), Cards::Nine(Color::Blue)],
                TrickType::Pair,
            ),
            (
                vec![Cards::Ten(Color::Black), Cards::Ten(Color::Blue)],
                TrickType::Pair,
            ),
            (
                vec![Cards::Jack(Color::Black), Cards::Jack(Color::Blue)],
                TrickType::Pair,
            ),
            (
                vec![Cards::Queen(Color::Black), Cards::Queen(Color::Blue)],
                TrickType::Pair,
            ),
            (
                vec![Cards::King(Color::Black), Cards::King(Color::Blue)],
                TrickType::Pair,
            ),
            (
                vec![Cards::Ace(Color::Black), Cards::Ace(Color::Blue)],
                TrickType::Pair,
            ),
            (
                vec![
                    Cards::Phoenix(Box::new(Phoenix { value: Some(2) })),
                    Cards::Two(Color::Black),
                ],
                TrickType::Pair,
            ),
        ];
        pair_trick_tests.iter().for_each(|(cards, expected)| {
            println!("{:?}", cards);
            assert_eq!(TrickType::try_from(cards.clone()).unwrap(), *expected)
        });
    }

    #[test]
    fn test_trio_trick() {
        let trio_trick_tests = vec![
            (
                vec![
                    Cards::Two(Color::Black),
                    Cards::Two(Color::Blue),
                    Cards::Two(Color::Red),
                ],
                TrickType::Triple,
            ),
            (
                vec![
                    Cards::Three(Color::Black),
                    Cards::Three(Color::Blue),
                    Cards::Three(Color::Red),
                ],
                TrickType::Triple,
            ),
            (
                vec![
                    Cards::Four(Color::Black),
                    Cards::Four(Color::Blue),
                    Cards::Four(Color::Red),
                ],
                TrickType::Triple,
            ),
            (
                vec![
                    Cards::Five(Color::Black),
                    Cards::Five(Color::Blue),
                    Cards::Five(Color::Red),
                ],
                TrickType::Triple,
            ),
            (
                vec![
                    Cards::Six(Color::Black),
                    Cards::Six(Color::Blue),
                    Cards::Six(Color::Red),
                ],
                TrickType::Triple,
            ),
            (
                vec![
                    Cards::Seven(Color::Black),
                    Cards::Seven(Color::Blue),
                    Cards::Seven(Color::Red),
                ],
                TrickType::Triple,
            ),
            (
                vec![
                    Cards::Eight(Color::Black),
                    Cards::Eight(Color::Blue),
                    Cards::Eight(Color::Red),
                ],
                TrickType::Triple,
            ),
            (
                vec![
                    Cards::Nine(Color::Black),
                    Cards::Nine(Color::Blue),
                    Cards::Nine(Color::Red),
                ],
                TrickType::Triple,
            ),
            (
                vec![
                    Cards::Ten(Color::Black),
                    Cards::Ten(Color::Blue),
                    Cards::Ten(Color::Red),
                ],
                TrickType::Triple,
            ),
            (
                vec![
                    Cards::Jack(Color::Black),
                    Cards::Jack(Color::Blue),
                    Cards::Jack(Color::Red),
                ],
                TrickType::Triple,
            ),
            (
                vec![
                    Cards::Queen(Color::Black),
                    Cards::Queen(Color::Blue),
                    Cards::Queen(Color::Red),
                ],
                TrickType::Triple,
            ),
            (
                vec![
                    Cards::King(Color::Black),
                    Cards::King(Color::Blue),
                    Cards::King(Color::Red),
                ],
                TrickType::Triple,
            ),
            (
                vec![
                    Cards::Ace(Color::Black),
                    Cards::Ace(Color::Blue),
                    Cards::Ace(Color::Red),
                ],
                TrickType::Triple,
            ),
            (
                vec![
                    Cards::Phoenix(Box::new(Phoenix { value: Some(2) })),
                    Cards::Two(Color::Black),
                    Cards::Two(Color::Blue),
                ],
                TrickType::Triple,
            ),
        ];

        trio_trick_tests.iter().for_each(|(cards, expected)| {
            assert_eq!(TrickType::try_from(cards.clone()).unwrap(), *expected)
        });
    }

    #[test]
    fn test_invalid_phoenix_trick() {
        let invalid_phoenix_trick_tests = vec![
            vec![
                Cards::Two(Color::Black),
                Cards::Phoenix(Box::new(Phoenix { value: Some(3) })),
            ],
            vec![
                Cards::Two(Color::Black),
                Cards::Two(Color::Blue),
                Cards::Phoenix(Box::new(Phoenix { value: Some(6) })),
            ],
            vec![
                Cards::Three(Color::Black),
                Cards::Three(Color::Blue),
                Cards::Three(Color::Red),
                Cards::Phoenix(Box::new(Phoenix { value: Some(6) })),
            ],
        ];

        invalid_phoenix_trick_tests
            .iter()
            .for_each(|cards| assert!(TrickType::try_from(cards.clone()).is_err()));
    }
}
