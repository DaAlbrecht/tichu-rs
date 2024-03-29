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

use super::types;

pub type GameStore = Arc<Mutex<HashMap<String, Game>>>;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Game {
    pub game_id: String,
    pub players: HashMap<Sid, Player>,
    pub phase: Option<Phase>,
    pub score_t1: u16,
    pub score_t2: u16,
    pub round: Option<Round>,
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

        let player_turn_sequence = types::generate_player_turn_sequence(turns);

        let round = Round {
            prev_next_player: player_turn_sequence,
            current_player: Sid::new(),
            ..Default::default()
        };

        self.round = Some(round);

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

        self.round
            .as_mut()
            .context("failed getting player turn iterator")?
            .current_player = *player_with_mahjong;

        self.round.as_mut().unwrap().round_initiator = *player_with_mahjong;
        Ok(())
    }

    pub fn play_turn(&mut self, player: &Sid, trick: &[Cards]) -> anyhow::Result<()> {
        let current_player = self
            .round
            .as_ref()
            .context("failed getting player turn iterator")?
            .current_player;

        if current_player != *player {
            return Err(anyhow!("not your turn"));
        }

        let trick_type = TrickType::try_from(trick)
            .with_context(|| format!("failed converting trick {:?} to trick type", trick))?;

        if let Some(turn_trick_type) = &self.current_trick_type {
            if &trick_type <= turn_trick_type {
                return Err(anyhow!(
                    "type: {:?} does not match current trick type or is less powerfull than: {:?}",
                    trick_type,
                    turn_trick_type
                ));
            }
        }

        let player = self
            .players
            .get_mut(player)
            .with_context(|| format!("failed getting player with socket_id {}", player))?;

        if !player_owns_cards(player.hand.as_ref().unwrap(), trick) {
            return Err(anyhow!("player does not own all cards"));
        }

        compare_tricks(&self.current_trick, trick)?;

        player
            .hand
            .as_mut()
            .unwrap()
            .cards
            .retain(|c| !trick.contains(c));

        self.current_trick = trick.to_vec();
        todo!()
    }
}

pub fn compare_tricks(last_trick: &[Cards], players_trick: &[Cards]) -> anyhow::Result<()> {
    let players_trick_type = TrickType::try_from(players_trick).with_context(|| {
        format!(
            "failed converting players trick {:?} to trick type",
            players_trick
        )
    })?;

    //this should never fail, since the last trick is already a valid trick
    let last_trick_type = TrickType::try_from(last_trick)?;

    match last_trick_type {
        TrickType::Single => {
            if let TrickType::Single = players_trick_type {
                return match players_trick[0].clone() {
                    Cards::Dragon => Ok(()),
                    _ => {
                        match last_trick[0].clone() {
                            Cards::Phoenix(card) => {
                                //phoenix only counts as 0.5, but i don't want to support floats so if they
                                //have the same value, the phoenix in theory would be 0.5 lower
                                if card.value <= players_trick[0].get_card_number() {
                                    return Ok(());
                                }
                                Err(anyhow!(
                                    "trick {:?} is not greater than last trick {:?}",
                                    players_trick,
                                    last_trick
                                ))
                            }
                            _ => {
                                if last_trick < players_trick {
                                    return Ok(());
                                }
                                Err(anyhow!(
                                    "trick {:?} is not greater than last trick {:?}",
                                    players_trick,
                                    last_trick
                                ))
                            }
                        }
                    }
                };
            }
            Err(anyhow!(
                "Trick type {:?} does not match {:?}",
                players_trick_type,
                last_trick_type
            ))
        }
        TrickType::Pair => {
            if let TrickType::Pair = players_trick_type {
                if last_trick[0].get_card_number() < players_trick[0].get_card_number() {
                    return Ok(());
                }
                return Err(anyhow!(
                    "tick {:?} is not greater than last trick {:?}",
                    players_trick,
                    last_trick
                ));
            }

            Err(anyhow!(
                "Trick type {:?} does not match {:?}",
                players_trick_type,
                last_trick_type
            ))
        }
        TrickType::Triple => {
            if let TrickType::Triple = players_trick_type {
                if last_trick[0].get_card_number() < players_trick[0].get_card_number() {
                    return Ok(());
                }

                return Err(anyhow!(
                    "tick {:?} is not greater than last trick {:?}",
                    players_trick,
                    last_trick
                ));
            }
            Err(anyhow!(
                "Trick type {:?} does not match {:?}",
                players_trick_type,
                last_trick_type
            ))
        }
        TrickType::FullHouse => {
            if let TrickType::FullHouse = players_trick_type {
                let mut last_trick = last_trick.to_owned();
                let mut players_trick = players_trick.to_owned();

                last_trick.sort();
                players_trick.sort();

                let last_3_kind = last_trick
                    .iter()
                    .find(|c| {
                        last_trick
                            .iter()
                            .filter(|c2| c2.get_card_number() == c.get_card_number())
                            .count()
                            == 3
                    })
                    .context("failed finding 3 of a kind in last trick")?;

                let players_3_kind = players_trick
                    .iter()
                    .find(|c| {
                        players_trick
                            .iter()
                            .filter(|c2| c2.get_card_number() == c.get_card_number())
                            .count()
                            == 3
                    })
                    .context("failed finding 3 of a kind in players trick")?;

                if last_3_kind.get_card_number() < players_3_kind.get_card_number() {
                    return Ok(());
                }

                return Err(anyhow!(
                    "tick {:?} is not greater than last trick {:?}",
                    players_trick,
                    last_trick
                ));
            }
            Err(anyhow!(
                "Trick type {:?} does not match {:?}",
                players_trick_type,
                last_trick_type
            ))
        }
        TrickType::Straight => {
            if let TrickType::Straight = players_trick_type {
                if players_trick.len() != last_trick.len() {
                    return Err(anyhow!("invalid trick"));
                }
                let last_highest_number = last_trick.iter().map(|c| c.get_card_number()).max();

                let players_highest_number =
                    players_trick.iter().map(|c| c.get_card_number()).max();

                if last_highest_number < players_highest_number {
                    return Ok(());
                }

                return Err(anyhow!(
                    "tick {:?} is not greater than last trick {:?}",
                    players_trick,
                    last_trick
                ));
            }
            Err(anyhow!("invalid trick"))
        }
        TrickType::FourOfAKind => {
            if let TrickType::FourOfAKind = players_trick_type {
                if last_trick[0].get_card_number() < players_trick[0].get_card_number() {
                    return Ok(());
                }

                return Err(anyhow!(
                    "tick {:?} is not greater than last trick {:?}",
                    players_trick,
                    last_trick
                ));
            }
            Err(anyhow!("invalid trick"))
        }
        TrickType::StraightFlush => {
            if let TrickType::StraightFlush = players_trick_type {
                if players_trick.len() != last_trick.len() {
                    return Err(anyhow!("invalid trick"));
                }
                let mut last_trick = last_trick.to_owned();
                let mut players_trick = players_trick.to_owned();
                last_trick.sort();
                players_trick.sort();
                if last_trick[0].get_card_number() < players_trick[0].get_card_number() {
                    return Ok(());
                }
                return Err(anyhow!(
                    "tick {:?} is not greater than last trick {:?}",
                    players_trick,
                    last_trick
                ));
            }
            Err(anyhow!("invalid trick"))
        }
        TrickType::SequenceOfPairs => {
            if let TrickType::SequenceOfPairs = players_trick_type {
                if players_trick.len() != last_trick.len() {
                    return Err(anyhow!("trick length does not match"));
                }
                let mut last_trick = last_trick.to_owned();
                let mut players_trick = players_trick.to_owned();
                last_trick.sort();
                players_trick.sort();
                if last_trick[0].get_card_number() < players_trick[0].get_card_number() {
                    return Ok(());
                }
                return Err(anyhow!(
                    "tick {:?} is not greater than last trick {:?}",
                    players_trick,
                    last_trick
                ));
            }
            Err(anyhow!("invalid trick"))
        }
    }
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

fn player_owns_cards(hand: &Hand, selected_cards: &[Cards]) -> bool {
    selected_cards.iter().all(|card| hand.cards.contains(card))
}
