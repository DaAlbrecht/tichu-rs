use anyhow::Result;

use rand::*;
use tracing::info;
use uuid::Uuid;

use super::handler::Exchange;

pub struct Game {
    pub game_id: Uuid,
    pub players: Vec<Player>,
}

struct Player {
    pub id: String,
    pub hand: Hand,
}

#[derive(Debug, Clone)]
struct Hand {
    pub cards: Vec<Cards>,
}

#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone, PartialEq)]
struct Card {
    color: Color,
}

#[derive(Debug, Clone, PartialEq)]
enum Color {
    Black,
    Blue,
    Red,
    Green,
    Special,
}

fn generate_hands() -> Vec<Hand> {
    let mut deck: Vec<Cards> = Vec::with_capacity(56);
    for card_color in [Color::Black, Color::Blue, Color::Red, Color::Green] {
        deck.push(Cards::Two(Card {
            color: card_color.clone(),
        }));
        deck.push(Cards::Three(Card {
            color: card_color.clone(),
        }));
        deck.push(Cards::Four(Card {
            color: card_color.clone(),
        }));
        deck.push(Cards::Five(Card {
            color: card_color.clone(),
        }));
        deck.push(Cards::Six(Card {
            color: card_color.clone(),
        }));
        deck.push(Cards::Seven(Card {
            color: card_color.clone(),
        }));
        deck.push(Cards::Eight(Card {
            color: card_color.clone(),
        }));
        deck.push(Cards::Nine(Card {
            color: card_color.clone(),
        }));
        deck.push(Cards::Ten(Card {
            color: card_color.clone(),
        }));
        deck.push(Cards::Jack(Card {
            color: card_color.clone(),
        }));
        deck.push(Cards::Queen(Card {
            color: card_color.clone(),
        }));
        deck.push(Cards::King(Card {
            color: card_color.clone(),
        }));
        deck.push(Cards::Ace(Card {
            color: card_color.clone(),
        }));
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

fn declare_exchange(player: Player, exchange: Exchange) -> Result<()> {
    //TODO: error handling
    exchange.player_card.iter().for_each(|(id, card)| {
        if player.id == *id {
            info!("cant exchange with yourself");
        }
        if !player_owns_card(&player, card) {
            info!("Player does not own card");
        }
    });

    Ok(())
}

fn player_owns_card(player: &Player, card: &Cards) -> bool {
    player.hand.cards.contains(card)
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
