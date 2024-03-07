use std::collections::HashMap;

use super::core::Cards;

#[derive(Debug, Clone)]
pub struct Exchange {
    pub player_id: String,
    pub player_card: HashMap<String, Cards>,
}
