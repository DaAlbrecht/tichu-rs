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

pub(crate) use crate::game_core::types::*;

pub type GameStore = Arc<Mutex<HashMap<String, Game>>>;

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

impl Game {
    pub fn new(game_id: String, players: HashMap<Sid, Player>) -> Self {
        Game {
            game_id,
            players,
            ..Default::default()
        }
    }

    pub fn join_team(&mut self, player_id: Sid, team: Team) -> anyhow::Result<String> {
        let team_count = self
            .players
            .values()
            .filter(|p| p.team == Some(team.clone()))
            .count();

        if team_count >= 2 && team != Team::Spectator {
            return Err(anyhow!("team is full"));
        }

        let player = self
            .players
            .get_mut(&player_id)
            .with_context(|| format!("failed getting player with socket_id {}", player_id))?;
        player.team = Some(team);
        Ok(player.username.clone())
    }

    pub fn deal_cards(&mut self) {
        let hands = generate_hands();

        for (player, hand) in self.players.iter_mut().zip(hands.iter()) {
            player.1.hand = Some(hand.clone());
        }
    }

    pub fn validate_exchange(&self, exchange: &Exchange) -> anyhow::Result<()> {
        let player = self
            .players
            .get(&exchange.player)
            .context("failed getting player")?;

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

        Ok(())
    }

    pub fn start(&mut self) -> anyhow::Result<()> {
        let team_1 = self.players.values().filter(|p| p.team == Some(Team::One));

        let team_2 = self.players.values().filter(|p| p.team == Some(Team::Two));

        let turns = team_1
            .zip(team_2)
            .flat_map(|(p1, p2)| vec![p1.socket_id, p2.socket_id])
            .collect::<Vec<_>>();

        let turn_iterator = TurnIterator::from(turns);
        self.player_turn_iterator = Some(turn_iterator);

        let player_with_mahjong = self
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
            .context("failed getting player with mahjong")?;

        self.player_turn_iterator
            .as_mut()
            .context("failed getting player turn iterator")?
            .current_player = *player_with_mahjong;
        Ok(())
    }
}

fn generate_hands() -> Vec<Hand> {
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

fn player_owns_cards(hand: &Hand, selected_cards: &[Cards]) -> bool {
    selected_cards.iter().all(|card| hand.cards.contains(card))
}

#[cfg(test)]
mod tests {

    use super::*;

    fn dummy_game() -> Game {
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
        Game::new("test_game".to_string(), players)
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
        let mut game = dummy_game();
        game.deal_cards();
        for player in game.players.values() {
            assert_eq!(player.hand.as_ref().unwrap().cards.len(), 14);
        }
    }

    #[test]
    fn test_validate_exchange() {
        let mut game = dummy_game();
        game.deal_cards();

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
                player: player.socket_id,
                player_card: valid_player_card,
            };

            let result = game.validate_exchange(&exchange);
            assert!(result.is_ok());

            let identical_cards = [cards[0].clone(), cards[0].clone(), cards[0].clone()];

            let invalid_player_card = valid_usernames
                .iter()
                .cloned()
                .zip(identical_cards)
                .collect::<HashMap<String, Cards>>();

            let invalid_exchange = Exchange {
                player: player.socket_id,
                player_card: invalid_player_card,
            };

            let result = game.validate_exchange(&invalid_exchange);
            assert!(result.is_err());

            let mut invalid_users = valid_usernames.clone();
            invalid_users[0] = player.username.clone();
            let invalid_player_card = invalid_users
                .iter()
                .cloned()
                .zip(cards.iter().cloned())
                .collect::<HashMap<String, Cards>>();

            let invalid_exchange = Exchange {
                player: player.socket_id,
                player_card: invalid_player_card,
            };

            let result = game.validate_exchange(&invalid_exchange);
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_turns() {
        let mut game = dummy_game();
        game.deal_cards();
        game.start().unwrap();

        assert_eq!(game.player_turn_iterator.is_some(), true);

        let mut turn_iterator = game.player_turn_iterator.unwrap();

        for _ in 0..4 {
            let turn = turn_iterator.next();
            assert!(turn.is_some());
        }
    }

    #[test]
    fn test_alternating_teams() {
        let mut game = dummy_game();
        game.deal_cards();
        game.start().unwrap();

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
        let mut game = dummy_game();
        game.deal_cards();
        game.start().unwrap();

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
    fn test_full_house_trick() {
        let full_house_trick_tests = vec![
            (
                vec![
                    Cards::Two(Color::Black),
                    Cards::Two(Color::Blue),
                    Cards::Two(Color::Red),
                    Cards::Three(Color::Black),
                    Cards::Three(Color::Blue),
                ],
                TrickType::FullHouse,
            ),
            (
                vec![
                    Cards::Two(Color::Black),
                    Cards::Two(Color::Blue),
                    Cards::Three(Color::Red),
                    Cards::Three(Color::Black),
                    Cards::Three(Color::Blue),
                ],
                TrickType::FullHouse,
            ),
            (
                vec![
                    Cards::Two(Color::Black),
                    Cards::Two(Color::Blue),
                    Cards::Three(Color::Black),
                    Cards::Three(Color::Blue),
                    Cards::Phoenix(Box::new(Phoenix { value: Some(3) })),
                ],
                TrickType::FullHouse,
            ),
        ];
        full_house_trick_tests.iter().for_each(|(cards, expected)| {
            let trick = TrickType::try_from(cards.clone());
            println!("{:?}", trick);
            println!("{:?}", cards);
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
            //hot take
            vec![
                Cards::Two(Color::Black),
                Cards::Two(Color::Blue),
                Cards::Two(Color::Red),
                Cards::Phoenix(Box::new(Phoenix { value: Some(2) })),
            ],
        ];

        invalid_phoenix_trick_tests
            .iter()
            .for_each(|cards| assert!(TrickType::try_from(cards.clone()).is_err()));
    }

    #[test]
    fn test_straight() {
        let straight_trick_tests = vec![
            (
                vec![
                    Cards::Two(Color::Black),
                    Cards::Three(Color::Blue),
                    Cards::Four(Color::Red),
                    Cards::Five(Color::Black),
                    Cards::Six(Color::Blue),
                ],
                TrickType::Straight,
            ),
            (
                vec![
                    Cards::Mahjong(Box::new(Mahjong { wish: None })),
                    Cards::Two(Color::Black),
                    Cards::Three(Color::Blue),
                    Cards::Four(Color::Red),
                    Cards::Five(Color::Black),
                ],
                TrickType::Straight,
            ),
            (
                vec![
                    Cards::Mahjong(Box::new(Mahjong { wish: None })),
                    Cards::Two(Color::Black),
                    Cards::Three(Color::Blue),
                    Cards::Four(Color::Red),
                    Cards::Phoenix(Box::new(Phoenix { value: Some(5) })),
                ],
                TrickType::Straight,
            ),
        ];

        straight_trick_tests.iter().for_each(|(cards, expected)| {
            assert_eq!(TrickType::try_from(cards.clone()).unwrap(), *expected)
        });
    }

    #[test]
    fn test_bomb() {
        let bomb_trick_test = vec![
            (
                vec![
                    Cards::Two(Color::Black),
                    Cards::Two(Color::Blue),
                    Cards::Two(Color::Red),
                    Cards::Two(Color::Green),
                ],
                TrickType::FourOfAKind,
            ),
            (
                vec![
                    Cards::Ten(Color::Black),
                    Cards::Ten(Color::Blue),
                    Cards::Ten(Color::Red),
                    Cards::Ten(Color::Green),
                ],
                TrickType::FourOfAKind,
            ),
        ];
        bomb_trick_test.iter().for_each(|(cards, expected)| {
            assert_eq!(TrickType::try_from(cards.clone()).unwrap(), *expected)
        });
    }

    #[test]
    fn test_straight_flush() {
        let straight_flush_trick_tests = vec![
            (
                vec![
                    Cards::Two(Color::Black),
                    Cards::Three(Color::Black),
                    Cards::Four(Color::Black),
                    Cards::Five(Color::Black),
                    Cards::Six(Color::Black),
                ],
                TrickType::StraightFlush,
            ),
            (
                vec![
                    Cards::Eight(Color::Green),
                    Cards::Nine(Color::Green),
                    Cards::Ten(Color::Green),
                    Cards::Jack(Color::Green),
                    Cards::Queen(Color::Green),
                    Cards::King(Color::Green),
                    Cards::Ace(Color::Green),
                ],
                TrickType::StraightFlush,
            ),
        ];

        straight_flush_trick_tests
            .iter()
            .for_each(|(cards, expected)| {
                assert_eq!(TrickType::try_from(cards.clone()).unwrap(), *expected)
            });
    }
}
