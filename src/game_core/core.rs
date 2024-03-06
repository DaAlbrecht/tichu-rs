use std::collections::HashMap;

use rand::*;
use uuid::Uuid;

pub struct Game {
    pub game_id: Uuid,
    pub hands: HashMap<String, Vec<Hand>>,
}

#[derive(Debug, Clone)]
pub struct Hand {
    pub cards: Vec<Cards>,
}

#[derive(Debug, Clone)]
pub enum Cards {
    Dog,
    Mahjong,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Jack,
    Queen,
    King,
    Ace,
    Phoenix,
    Dragon,
}

fn generate_hands() -> Vec<Hand> {
    let mut deck: Vec<Cards> = Vec::with_capacity(56);
    for _ in 0..4 {
        deck.push(Cards::Two);
        deck.push(Cards::Three);
        deck.push(Cards::Four);
        deck.push(Cards::Five);
        deck.push(Cards::Six);
        deck.push(Cards::Seven);
        deck.push(Cards::Eight);
        deck.push(Cards::Nine);
        deck.push(Cards::Ten);
        deck.push(Cards::Jack);
        deck.push(Cards::Queen);
        deck.push(Cards::King);
        deck.push(Cards::Ace);
    }
    deck.push(Cards::Dog);
    deck.push(Cards::Mahjong);
    deck.push(Cards::Phoenix);
    deck.push(Cards::Dragon);

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
