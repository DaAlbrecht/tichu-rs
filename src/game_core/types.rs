use anyhow::anyhow;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use socketioxide::socket::Sid;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Round {
    pub prev_next_player: HashMap<Sid, Sid>,
    pub current_player: Sid,
    pub round_initiator: Sid,
    pub previous_action: Option<Action>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Action {
    Pass,
    Play,
}

pub fn generate_player_turn_sequence(players: Vec<Sid>) -> HashMap<Sid, Sid> {
    let mut turn_sequence = HashMap::new();
    let mut previous_player = players.last().unwrap();
    for current_player in players.iter() {
        turn_sequence.insert(*previous_player, *current_player);
        previous_player = current_player;
    }
    turn_sequence
}

impl Iterator for Round {
    type Item = Sid;

    fn next(&mut self) -> Option<Self::Item> {
        let next_player = self.prev_next_player.get(&self.current_player);
        if let Some(prev_action) = &self.previous_action {
            if prev_action == &Action::Pass && next_player == Some(&self.round_initiator) {
                return None;
            }
        }
        self.current_player = *next_player?;
        Some(self.current_player)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Phase {
    Exchanging,
    Playing,
}

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
    pub cards: Vec<Cards>,
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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum Color {
    Black,
    Blue,
    Red,
    Green,
}

//When comparing Cards, only the number is relevant
impl PartialOrd for Color {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Color {
    fn cmp(&self, _: &Self) -> std::cmp::Ordering {
        std::cmp::Ordering::Equal
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Team {
    One,
    Two,
    Spectator,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, PartialOrd)]
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
    pub fn get_card_number(&self) -> Option<u8> {
        match self {
            Cards::Mahjong(_) => Some(1),
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

impl TryFrom<&[Cards]> for TrickType {
    type Error = anyhow::Error;

    fn try_from(cards: &[Cards]) -> anyhow::Result<Self> {
        fn all_equal(cards: &[Cards]) -> bool {
            let mut card_types = cards
                .iter()
                .filter_map(|c| c.get_card_number())
                .collect::<Vec<u8>>();
            card_types.sort();
            card_types.dedup();
            card_types.len() == 1
        }

        fn is_sequence(cards: &[Cards]) -> bool {
            let mut card_digits = cards
                .iter()
                .filter_map(|c| c.get_card_number())
                .collect::<Vec<u8>>();
            card_digits.sort();
            card_digits.windows(2).all(|w| w[0] + 1 == w[1])
        }

        fn is_sequence_of_pairs(cards: &[Cards]) -> bool {
            let mut card_digits = cards
                .iter()
                .filter_map(|c| c.get_card_number())
                .collect::<Vec<u8>>();
            card_digits.sort();
            card_digits
                .windows(4)
                .all(|w| w[0] == w[1] && w[2] == w[3] && w[0] + 1 == w[2])
        }

        fn is_full_house(cards: &[Cards]) -> bool {
            let card_values = cards
                .iter()
                .filter_map(|c| c.get_card_number())
                .collect::<Vec<u8>>();

            let mut unique_cards = card_values.clone();
            unique_cards.sort();
            unique_cards.dedup();

            if unique_cards.len() != 2 {
                return false;
            }

            let occurrences = unique_cards
                .iter()
                .map(|c| card_values.iter().filter(|&x| x == c).count())
                .collect::<Vec<_>>();

            matches!(occurrences.as_slice(), [2, 3] | [3, 2])
        }

        match cards.len() {
            1 => Ok(TrickType::Single),
            2 if all_equal(cards) => Ok(TrickType::Pair),
            3 if all_equal(cards) => Ok(TrickType::Triple),
            4 => {
                if cards
                    .iter()
                    .all(|c| std::mem::discriminant(c) == std::mem::discriminant(&cards[0]))
                {
                    Ok(TrickType::FourOfAKind)
                } else {
                    Err(anyhow!("invalid trick"))
                }
            }
            5 if is_full_house(cards) => Ok(TrickType::FullHouse),
            4..=14 if is_sequence_of_pairs(cards) => Ok(TrickType::SequenceOfPairs),
            5..=14 if is_sequence(cards) => {
                let colors = cards
                    .iter()
                    .filter_map(|c| c.get_color())
                    .collect::<Vec<_>>();

                //creating a straight flush with phoenix is not allowed
                if colors.len() != cards.len() {
                    return Ok(TrickType::Straight);
                }
                if colors
                    .iter()
                    .all(|c| std::mem::discriminant(c) == std::mem::discriminant(&colors[0]))
                {
                    return Ok(TrickType::StraightFlush);
                }
                Ok(TrickType::Straight)
            }
            _ => Err(anyhow!("invalid trick")),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Exchange {
    pub player: Sid,
    pub player_card: HashMap<String, Cards>,
}
