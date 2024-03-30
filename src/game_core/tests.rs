#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use socketioxide::socket::Sid;

    use crate::game_core::core::{
        compare_tricks, generate_hands, Action, Cards, Color, Exchange, Game, Hand, Mahjong,
        Phoenix, Player, Team, TrickType, Turn,
    };

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

        assert_eq!(game.round.is_some(), true);

        let mut turn_iterator = game.round.unwrap();

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

        assert_eq!(game.round.is_some(), true);

        let turn_sequence = game.round.unwrap().prev_next_player;

        for (previous, current) in turn_sequence.iter() {
            let prev = previous.clone();
            let curr = current.clone();
            let team_previous = game.players.get(&prev).unwrap().team.clone();
            let team_current = game.players.get(&curr.socket_id).unwrap().team.clone();
            assert_ne!(team_previous, team_current);
        }
    }

    #[test]
    fn test_starting_player() {
        let mut game = dummy_game();
        game.deal_cards();
        game.start().unwrap();

        assert_eq!(game.round.is_some(), true);

        let players_turn = game.round.unwrap().current_player;

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
            assert_eq!(TrickType::try_from(cards.as_slice()).unwrap(), *expected)
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
            assert_eq!(TrickType::try_from(cards.as_slice()).unwrap(), *expected)
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
            assert_eq!(TrickType::try_from(cards.as_slice()).unwrap(), *expected)
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
            let trick = TrickType::try_from(cards.as_slice());
            assert_eq!(TrickType::try_from(cards.as_slice()).unwrap(), *expected)
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
            .for_each(|cards| assert!(TrickType::try_from(cards.as_slice()).is_err()));
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
            assert_eq!(TrickType::try_from(cards.as_slice()).unwrap(), *expected)
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
            assert_eq!(TrickType::try_from(cards.as_slice()).unwrap(), *expected)
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
                assert_eq!(TrickType::try_from(cards.as_slice()).unwrap(), *expected)
            });
    }

    #[test]
    fn test_compare_single_tricks() {
        let single_trick_tests = vec![
            (
                vec![Cards::Two(Color::Black)],
                vec![Cards::Three(Color::Black)],
                true,
            ),
            (
                vec![Cards::Jack(Color::Black)],
                vec![Cards::Ace(Color::Red)],
                true,
            ),
            (
                vec![Cards::Two(Color::Black)],
                vec![Cards::Two(Color::Blue)],
                false,
            ),
            (
                vec![Cards::Phoenix(Box::new(Phoenix { value: Some(1) }))],
                vec![Cards::Two(Color::Black)],
                true,
            ),
            (
                vec![Cards::Phoenix(Box::new(Phoenix { value: Some(2) }))],
                vec![Cards::Phoenix(Box::new(Phoenix { value: Some(2) }))],
                true,
            ),
            (vec![Cards::Two(Color::Black)], vec![Cards::Dog], false),
            (
                vec![Cards::Mahjong(Box::new(Mahjong { wish: None }))],
                vec![Cards::Two(Color::Black)],
                true,
            ),
            (
                vec![Cards::Mahjong(Box::new(Mahjong { wish: None }))],
                vec![Cards::Dog],
                false,
            ),
            (vec![Cards::Dragon], vec![Cards::Two(Color::Black)], false),
            (vec![Cards::Ace(Color::Black)], vec![Cards::Dragon], true),
            (
                vec![Cards::Ace(Color::Black)],
                vec![Cards::Phoenix(Box::new(Phoenix { value: Some(14) }))],
                true,
            ),
        ];

        single_trick_tests
            .iter()
            .for_each(|(last, player, expected)| {
                let result = compare_tricks(last, player);
                assert_eq!(result.is_ok(), *expected);
            });
    }

    #[test]
    fn test_compare_pair_tricks() {
        let pair_trick_tests = vec![
            (
                vec![Cards::Two(Color::Black), Cards::Two(Color::Blue)],
                vec![Cards::Three(Color::Black), Cards::Three(Color::Blue)],
                true,
            ),
            (
                vec![Cards::Two(Color::Black), Cards::Two(Color::Blue)],
                vec![Cards::Two(Color::Green), Cards::Two(Color::Red)],
                false,
            ),
            (
                vec![Cards::Two(Color::Black), Cards::Two(Color::Blue)],
                vec![
                    Cards::Phoenix(Box::new(Phoenix { value: Some(2) })),
                    Cards::Two(Color::Black),
                ],
                false,
            ),
            (
                vec![Cards::Two(Color::Black), Cards::Two(Color::Blue)],
                vec![
                    Cards::Phoenix(Box::new(Phoenix { value: Some(3) })),
                    Cards::Three(Color::Black),
                ],
                true,
            ),
            (
                vec![Cards::Ace(Color::Black), Cards::Ace(Color::Blue)],
                vec![Cards::Two(Color::Green), Cards::Two(Color::Red)],
                false,
            ),
            (
                vec![Cards::Two(Color::Green), Cards::Two(Color::Red)],
                vec![Cards::Ace(Color::Black), Cards::Ace(Color::Blue)],
                true,
            ),
        ];

        pair_trick_tests
            .iter()
            .for_each(|(last, player, expected)| {
                let result = compare_tricks(last, player);
                assert_eq!(result.is_ok(), *expected);
            });
    }

    #[test]
    fn test_compare_trio_tricks() {
        let trio_trick_tests = vec![
            (
                vec![
                    Cards::Two(Color::Black),
                    Cards::Two(Color::Blue),
                    Cards::Two(Color::Red),
                ],
                vec![
                    Cards::Three(Color::Black),
                    Cards::Three(Color::Blue),
                    Cards::Three(Color::Red),
                ],
                true,
            ),
            (
                vec![
                    Cards::Two(Color::Black),
                    Cards::Two(Color::Blue),
                    Cards::Two(Color::Red),
                ],
                vec![
                    Cards::Two(Color::Green),
                    Cards::Two(Color::Red),
                    Cards::Two(Color::Black),
                ],
                false,
            ),
            (
                vec![
                    Cards::Two(Color::Black),
                    Cards::Two(Color::Blue),
                    Cards::Two(Color::Red),
                ],
                vec![
                    Cards::Phoenix(Box::new(Phoenix { value: Some(2) })),
                    Cards::Two(Color::Black),
                    Cards::Two(Color::Blue),
                ],
                false,
            ),
            (
                vec![
                    Cards::Two(Color::Black),
                    Cards::Two(Color::Blue),
                    Cards::Two(Color::Red),
                ],
                vec![
                    Cards::Phoenix(Box::new(Phoenix { value: Some(3) })),
                    Cards::Three(Color::Black),
                    Cards::Three(Color::Blue),
                ],
                true,
            ),
            (
                vec![
                    Cards::Ace(Color::Black),
                    Cards::Ace(Color::Blue),
                    Cards::Ace(Color::Red),
                ],
                vec![
                    Cards::Two(Color::Green),
                    Cards::Two(Color::Red),
                    Cards::Two(Color::Black),
                ],
                false,
            ),
            (
                vec![
                    Cards::Two(Color::Green),
                    Cards::Two(Color::Red),
                    Cards::Two(Color::Black),
                ],
                vec![
                    Cards::Ace(Color::Black),
                    Cards::Ace(Color::Blue),
                    Cards::Ace(Color::Red),
                ],
                true,
            ),
        ];

        trio_trick_tests
            .iter()
            .for_each(|(last, player, expected)| {
                let result = compare_tricks(last, player);
                assert_eq!(result.is_ok(), *expected);
            });
    }

    #[test]
    fn test_compare_full_house_tricks() {
        let full_house_trick_tests = vec![
            (
                vec![
                    Cards::Two(Color::Black),
                    Cards::Two(Color::Blue),
                    Cards::Two(Color::Red),
                    Cards::Three(Color::Black),
                    Cards::Three(Color::Blue),
                ],
                vec![
                    Cards::Four(Color::Black),
                    Cards::Four(Color::Blue),
                    Cards::Four(Color::Red),
                    Cards::Two(Color::Black),
                    Cards::Two(Color::Blue),
                ],
                true,
            ),
            (
                vec![
                    Cards::Two(Color::Black),
                    Cards::Two(Color::Blue),
                    Cards::Three(Color::Black),
                    Cards::Three(Color::Blue),
                    Cards::Three(Color::Red),
                ],
                vec![
                    Cards::Two(Color::Green),
                    Cards::Two(Color::Red),
                    Cards::Two(Color::Black),
                    Cards::Three(Color::Black),
                    Cards::Three(Color::Blue),
                ],
                false,
            ),
            (
                vec![
                    Cards::Two(Color::Black),
                    Cards::Two(Color::Blue),
                    Cards::Two(Color::Red),
                    Cards::Three(Color::Black),
                    Cards::Three(Color::Blue),
                ],
                vec![
                    Cards::Phoenix(Box::new(Phoenix { value: Some(2) })),
                    Cards::Two(Color::Black),
                    Cards::Two(Color::Blue),
                    Cards::Three(Color::Black),
                    Cards::Three(Color::Blue),
                ],
                false,
            ),
            (
                vec![
                    Cards::Two(Color::Black),
                    Cards::Two(Color::Blue),
                    Cards::Two(Color::Red),
                    Cards::Three(Color::Black),
                    Cards::Three(Color::Blue),
                ],
                vec![
                    Cards::Phoenix(Box::new(Phoenix { value: Some(3) })),
                    Cards::Three(Color::Black),
                    Cards::Three(Color::Blue),
                    Cards::Four(Color::Black),
                    Cards::Four(Color::Blue),
                ],
                true,
            ),
            (
                vec![
                    Cards::Ace(Color::Black),
                    Cards::Ace(Color::Blue),
                    Cards::Ace(Color::Red),
                    Cards::Two(Color::Black),
                    Cards::Two(Color::Blue),
                ],
                vec![
                    Cards::Two(Color::Green),
                    Cards::Two(Color::Red),
                    Cards::Two(Color::Black),
                    Cards::Three(Color::Black),
                    Cards::Three(Color::Blue),
                ],
                false,
            ),
            (
                vec![
                    Cards::Two(Color::Green),
                    Cards::Two(Color::Red),
                    Cards::Two(Color::Black),
                    Cards::Three(Color::Black),
                    Cards::Three(Color::Blue),
                ],
                vec![
                    Cards::Ace(Color::Black),
                    Cards::Ace(Color::Blue),
                    Cards::Ace(Color::Red),
                    Cards::Two(Color::Black),
                    Cards::Two(Color::Blue),
                ],
                true,
            ),
            (
                vec![
                    Cards::Ace(Color::Green),
                    Cards::Ace(Color::Red),
                    Cards::Three(Color::Black),
                    Cards::Three(Color::Red),
                    Cards::Three(Color::Blue),
                ],
                vec![
                    Cards::Four(Color::Black),
                    Cards::Four(Color::Blue),
                    Cards::Four(Color::Red),
                    Cards::Two(Color::Black),
                    Cards::Two(Color::Blue),
                ],
                true,
            ),
        ];

        full_house_trick_tests
            .iter()
            .for_each(|(last, player, expected)| {
                let result = compare_tricks(last, player);
                assert_eq!(result.is_ok(), *expected);
            });
    }

    #[test]
    fn test_compare_straight_tricks() {
        let straight_trick_tests = vec![
            (
                vec![
                    Cards::Two(Color::Black),
                    Cards::Three(Color::Blue),
                    Cards::Four(Color::Red),
                    Cards::Five(Color::Black),
                    Cards::Six(Color::Blue),
                ],
                vec![
                    Cards::Three(Color::Black),
                    Cards::Four(Color::Blue),
                    Cards::Five(Color::Red),
                    Cards::Six(Color::Black),
                    Cards::Seven(Color::Blue),
                ],
                true,
            ),
            (
                vec![
                    Cards::Two(Color::Black),
                    Cards::Three(Color::Blue),
                    Cards::Four(Color::Red),
                    Cards::Five(Color::Black),
                    Cards::Six(Color::Blue),
                ],
                vec![
                    Cards::Mahjong(Box::new(Mahjong { wish: None })),
                    Cards::Two(Color::Black),
                    Cards::Three(Color::Blue),
                    Cards::Four(Color::Red),
                    Cards::Five(Color::Black),
                ],
                false,
            ),
            (
                vec![
                    Cards::Two(Color::Black),
                    Cards::Three(Color::Blue),
                    Cards::Four(Color::Red),
                    Cards::Five(Color::Black),
                    Cards::Six(Color::Blue),
                ],
                vec![
                    Cards::Phoenix(Box::new(Phoenix { value: Some(2) })),
                    Cards::Three(Color::Black),
                    Cards::Four(Color::Blue),
                    Cards::Five(Color::Red),
                    Cards::Six(Color::Black),
                ],
                false,
            ),
            (
                vec![
                    Cards::Two(Color::Black),
                    Cards::Three(Color::Blue),
                    Cards::Four(Color::Red),
                    Cards::Five(Color::Black),
                    Cards::Six(Color::Blue),
                ],
                vec![
                    Cards::Phoenix(Box::new(Phoenix { value: Some(3) })),
                    Cards::Four(Color::Black),
                    Cards::Five(Color::Blue),
                    Cards::Six(Color::Red),
                    Cards::Seven(Color::Black),
                ],
                true,
            ),
            (
                vec![
                    Cards::Two(Color::Black),
                    Cards::Three(Color::Blue),
                    Cards::Four(Color::Red),
                    Cards::Five(Color::Black),
                    Cards::Six(Color::Blue),
                ],
                vec![
                    Cards::Phoenix(Box::new(Phoenix { value: Some(4) })),
                    Cards::Five(Color::Black),
                    Cards::Six(Color::Blue),
                    Cards::Seven(Color::Red),
                    Cards::Eight(Color::Black),
                ],
                true,
            ),
            (
                vec![
                    Cards::Mahjong(Box::new(Mahjong { wish: None })),
                    Cards::Two(Color::Black),
                    Cards::Three(Color::Blue),
                    Cards::Four(Color::Red),
                    Cards::Five(Color::Black),
                ],
                vec![
                    Cards::Phoenix(Box::new(Phoenix { value: Some(3) })),
                    Cards::Four(Color::Black),
                    Cards::Five(Color::Blue),
                    Cards::Six(Color::Red),
                    Cards::Seven(Color::Black),
                ],
                true,
            ),
            (
                vec![
                    Cards::Mahjong(Box::new(Mahjong { wish: None })),
                    Cards::Two(Color::Black),
                    Cards::Three(Color::Blue),
                    Cards::Four(Color::Red),
                    Cards::Five(Color::Black),
                    Cards::Six(Color::Blue),
                ],
                vec![
                    Cards::Phoenix(Box::new(Phoenix { value: Some(3) })),
                    Cards::Four(Color::Black),
                    Cards::Five(Color::Blue),
                    Cards::Six(Color::Red),
                    Cards::Seven(Color::Black),
                    Cards::Eight(Color::Blue),
                ],
                true,
            ),
        ];

        straight_trick_tests
            .iter()
            .for_each(|(last, player, expected)| {
                let result = compare_tricks(last, player);
                assert_eq!(result.is_ok(), *expected);
            });
    }

    #[test]
    fn test_bombs() {
        let bomb_trick_tests = vec![
            (
                vec![
                    Cards::Two(Color::Black),
                    Cards::Two(Color::Blue),
                    Cards::Two(Color::Red),
                    Cards::Two(Color::Green),
                ],
                vec![
                    Cards::Three(Color::Black),
                    Cards::Three(Color::Blue),
                    Cards::Three(Color::Red),
                    Cards::Three(Color::Green),
                ],
                true,
            ),
            (
                vec![
                    Cards::Two(Color::Black),
                    Cards::Three(Color::Red),
                    Cards::Four(Color::Black),
                    Cards::Five(Color::Blue),
                    Cards::Six(Color::Green),
                ],
                vec![
                    Cards::Three(Color::Black),
                    Cards::Three(Color::Blue),
                    Cards::Three(Color::Red),
                    Cards::Three(Color::Green),
                ],
                true,
            ),
            (
                vec![Cards::King(Color::Black), Cards::King(Color::Blue)],
                vec![
                    Cards::Ace(Color::Black),
                    Cards::Ace(Color::Blue),
                    Cards::Ace(Color::Red),
                    Cards::Ace(Color::Green),
                ],
                true,
            ),
            (
                vec![
                    Cards::Two(Color::Black),
                    Cards::Two(Color::Blue),
                    Cards::Two(Color::Red),
                    Cards::Two(Color::Green),
                ],
                vec![
                    Cards::Two(Color::Black),
                    Cards::Three(Color::Black),
                    Cards::Four(Color::Black),
                    Cards::Five(Color::Black),
                    Cards::Six(Color::Black),
                ],
                true,
            ),
            (
                vec![
                    Cards::Two(Color::Black),
                    Cards::Three(Color::Black),
                    Cards::Four(Color::Black),
                    Cards::Five(Color::Black),
                    Cards::Six(Color::Black),
                ],
                vec![
                    Cards::Two(Color::Black),
                    Cards::Two(Color::Blue),
                    Cards::Two(Color::Red),
                    Cards::Two(Color::Green),
                ],
                false,
            ),
        ];

        bomb_trick_tests
            .iter()
            .for_each(|(last, player, expected)| {
                let result = compare_tricks(last, player);
                assert_eq!(result.is_ok(), *expected);
            });
    }

    #[test]
    fn test_init_round() {
        let mut game = dummy_game();
        game.deal_cards();
        game.start().unwrap();

        let first_player = game.round.as_ref().unwrap().current_player;
        let first_player_hand = game
            .players
            .get(&first_player)
            .unwrap()
            .hand
            .clone()
            .unwrap();

        let first_player_card = first_player_hand.cards.first().unwrap().clone();

        let turn = Turn {
            player: first_player,
            action: Action::Play,
            cards: Some(vec![first_player_card]),
        };

        let result = game.play_turn(turn);
        assert!(result.is_ok());

        let next_player = game.round.as_ref().unwrap().current_player;

        assert_ne!(first_player, next_player);
        assert_eq!(
            game.round.as_ref().unwrap().previous_action,
            Some(Action::Play)
        );
        assert_eq!(
            game.round.as_ref().unwrap().last_played_player,
            first_player
        );
        assert_eq!(
            game.round
                .as_ref()
                .unwrap()
                .prev_next_player
                .get(&first_player)
                .unwrap()
                .socket_id,
            next_player
        );

        assert_eq!(
            game.round.as_ref().unwrap().current_trick_type,
            Some(TrickType::Single)
        );
    }

    #[test]
    fn test_invalid_init_round() {
        let mut game = dummy_game();
        game.deal_cards();
        game.start().unwrap();

        let second_player = game
            .round
            .as_ref()
            .unwrap()
            .prev_next_player
            .get(&game.round.as_ref().unwrap().current_player)
            .unwrap()
            .clone();

        let second_player_hand = game
            .players
            .get(&second_player.socket_id)
            .unwrap()
            .hand
            .clone()
            .unwrap();

        let second_player_card = second_player_hand.cards.first().unwrap().clone();

        let turn = Turn {
            player: second_player.socket_id,
            action: Action::Play,
            cards: Some(vec![second_player_card]),
        };

        assert_eq!(game.play_turn(turn).is_err(), true);
    }

    #[test]
    fn test_play_turns() {
        let mut game = dummy_game();

        game.deal_cards();
        game.start().unwrap();

        let all_cards = game.players.values().fold(vec![], |mut acc, player| {
            let cards = player.hand.as_ref().unwrap().cards.clone();
            acc.extend(cards);
            acc
        });

        for player in game.players.values_mut() {
            let new_hand = Hand {
                cards: all_cards.clone(),
            };
            player.hand = Some(new_hand);
        }

        let p1 = game.round.as_ref().unwrap().current_player;

        let first_turn = Turn {
            player: p1,
            action: Action::Play,
            cards: Some(vec![Cards::Two(Color::Black)]),
        };

        assert_eq!(game.play_turn(first_turn).is_ok(), true);

        let p2 = game.round.as_ref().unwrap().current_player;

        let second_turn = Turn {
            player: p2,
            action: Action::Play,
            cards: Some(vec![Cards::Ten(Color::Black)]),
        };

        let result = game.play_turn(second_turn);

        assert_eq!(result.is_ok(), true);
        assert_eq!(result.unwrap(), false);

        let p3 = game.round.as_ref().unwrap().current_player;

        let third_turn = Turn {
            player: p3,
            action: Action::Pass,
            cards: None,
        };

        let result = game.play_turn(third_turn);

        assert_eq!(result.is_ok(), true);
        assert_eq!(result.unwrap(), false);

        let p4 = game.round.as_ref().unwrap().current_player;
        let fourth_turn = Turn {
            player: p4,
            action: Action::Pass,
            cards: None,
        };

        let result = game.play_turn(fourth_turn);

        assert_eq!(result.is_ok(), true);
        assert_eq!(result.unwrap(), false);

        let p1 = game.round.as_ref().unwrap().current_player;
        let fifth_turn = Turn {
            player: p1,
            action: Action::Pass,
            cards: None,
        };

        let result = game.play_turn(fifth_turn);
        assert_eq!(result.is_ok(), true);
        assert_eq!(result.unwrap(), true);

        assert_eq!(game.cleanup_trick().is_ok(), true);

        //turn is over, next player should be the winner of the last trick
        let next_player = game.round.as_ref().unwrap().current_player;

        assert_eq!(next_player, p2);

        let p2_points = game.players.get(&p2).unwrap().trick_points;
        assert_eq!(p2_points, 10);

        let sixth_turn = Turn {
            player: p2,
            action: Action::Play,
            cards: Some(vec![Cards::Three(Color::Black), Cards::Three(Color::Blue)]),
        };

        let result = game.play_turn(sixth_turn);

        assert_eq!(result.is_ok(), true);
        assert_eq!(result.unwrap(), false);

        let seventh_turn = Turn {
            player: p3,
            action: Action::Play,
            cards: Some(vec![Cards::Four(Color::Black), Cards::Four(Color::Blue)]),
        };

        let result = game.play_turn(seventh_turn);

        assert_eq!(result.is_ok(), true);
        assert_eq!(result.unwrap(), false);

        let eighth_turn = Turn {
            player: p4,
            action: Action::Play,
            cards: Some(vec![Cards::Five(Color::Black), Cards::Five(Color::Blue)]),
        };

        let result = game.play_turn(eighth_turn);

        assert_eq!(result.is_ok(), true);
        assert_eq!(result.unwrap(), false);

        let ninth_turn = Turn {
            player: p1,
            action: Action::Play,
            cards: Some(vec![Cards::Six(Color::Black), Cards::Six(Color::Blue)]),
        };

        let result = game.play_turn(ninth_turn);

        assert_eq!(result.is_ok(), true);
        assert_eq!(result.unwrap(), false);

        let tenth_turn = Turn {
            player: p2,
            action: Action::Pass,
            cards: None,
        };

        let result = game.play_turn(tenth_turn);
        assert_eq!(result.is_ok(), true);
        assert_eq!(result.unwrap(), false);

        let eleventh_turn = Turn {
            player: p3,
            action: Action::Pass,
            cards: None,
        };

        let result = game.play_turn(eleventh_turn);
        assert_eq!(result.is_ok(), true);
        assert_eq!(result.unwrap(), false);

        let twelfth_turn = Turn {
            player: p4,
            action: Action::Pass,
            cards: None,
        };

        let result = game.play_turn(twelfth_turn);
        assert_eq!(result.is_ok(), true);
        assert_eq!(result.unwrap(), true);

        assert_eq!(game.cleanup_trick().is_ok(), true);

        assert_eq!(game.round.as_ref().unwrap().current_player, p1);

        assert_eq!(game.players.get(&p1).unwrap().trick_points, 10);
        assert_eq!(game.players.get(&p2).unwrap().trick_points, 10);

        let t_13 = Turn {
            player: p1,
            action: Action::Play,
            cards: Some(vec![
                Cards::Seven(Color::Black),
                Cards::Seven(Color::Blue),
                Cards::Seven(Color::Red),
            ]),
        };

        let result = game.play_turn(t_13);
        assert_eq!(result.is_ok(), true);
        assert_eq!(result.unwrap(), false);

        let t_14 = Turn {
            player: p2,
            action: Action::Play,
            cards: Some(vec![
                Cards::Eight(Color::Black),
                Cards::Eight(Color::Blue),
                Cards::Eight(Color::Red),
            ]),
        };

        let result = game.play_turn(t_14);
        assert_eq!(result.is_ok(), true);
        assert_eq!(result.unwrap(), false);

        let t_15 = Turn {
            player: p3,
            action: Action::Play,
            cards: Some(vec![
                Cards::Nine(Color::Black),
                Cards::Nine(Color::Blue),
                Cards::Phoenix(Box::new(Phoenix { value: Some(9) })),
            ]),
        };

        let result = game.play_turn(t_15);
        assert_eq!(result.is_ok(), true);
        assert_eq!(result.unwrap(), false);

        let t_16 = Turn {
            player: p4,
            action: Action::Pass,
            cards: None,
        };

        let result = game.play_turn(t_16);
        assert_eq!(result.is_ok(), true);
        assert_eq!(result.unwrap(), false);

        let t_17 = Turn {
            player: p1,
            action: Action::Pass,
            cards: None,
        };

        let result = game.play_turn(t_17);
        assert_eq!(result.is_ok(), true);
        assert_eq!(result.unwrap(), false);

        let t_18 = Turn {
            player: p2,
            action: Action::Play,
            cards: Some(vec![
                Cards::Ten(Color::Green),
                Cards::Ten(Color::Blue),
                Cards::Ten(Color::Red),
            ]),
        };

        let result = game.play_turn(t_18);
        assert_eq!(result.is_ok(), true);
        assert_eq!(result.unwrap(), false);

        let t_19 = Turn {
            player: p3,
            action: Action::Pass,
            cards: None,
        };

        let result = game.play_turn(t_19);
        assert_eq!(result.is_ok(), true);
        assert_eq!(result.unwrap(), false);

        let t_20 = Turn {
            player: p4,
            action: Action::Play,
            cards: Some(vec![
                Cards::Jack(Color::Black),
                Cards::Jack(Color::Blue),
                Cards::Jack(Color::Red),
            ]),
        };

        let result = game.play_turn(t_20);
        assert_eq!(result.is_ok(), true);
        assert_eq!(result.unwrap(), false);

        let t_21 = Turn {
            player: p1,
            action: Action::Pass,
            cards: None,
        };

        let result = game.play_turn(t_21);
        assert_eq!(result.is_ok(), true);
        assert_eq!(result.unwrap(), false);

        let t_22 = Turn {
            player: p2,
            action: Action::Pass,
            cards: None,
        };

        let result = game.play_turn(t_22);
        assert_eq!(result.is_ok(), true);
        assert_eq!(result.unwrap(), false);

        let t_23 = Turn {
            player: p3,
            action: Action::Pass,
            cards: None,
        };

        let result = game.play_turn(t_23);
        assert_eq!(result.is_ok(), true);
        assert_eq!(result.unwrap(), true);

        assert_eq!(game.cleanup_trick().is_ok(), true);

        assert_eq!(game.round.unwrap().current_player, p4);

        assert_eq!(game.players.get(&p1).unwrap().trick_points, 10);
        assert_eq!(game.players.get(&p2).unwrap().trick_points, 10);
        assert_eq!(game.players.get(&p3).unwrap().trick_points, 0);
        assert_eq!(game.players.get(&p4).unwrap().trick_points, 5);
    }
}
