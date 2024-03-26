#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use socketioxide::socket::Sid;

    use crate::game_core::core::{
        compare_tricks, generate_hands, Cards, Color, Exchange, Game, Mahjong, Phoenix, Player,
        Team, TrickType,
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
            let team_current = game.players.get(&curr).unwrap().team.clone();
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
            println!("{:?}", cards);
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
            println!("{:?}", trick);
            println!("{:?}", cards);
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
                println!("last {:?} player {:?}", last, player);
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
                println!("last {:?} player {:?}", last, player);
                let result = compare_tricks(last, player);
                assert_eq!(result.is_ok(), *expected);
            });
    }
}
