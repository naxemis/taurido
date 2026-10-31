use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Card {
    id: u16,
    name: String,
    description: String,
    card_place: u16,
}

impl Card {
    /// Creates a card with the given ID, name, and position.
    pub fn new(id: u16, name: String, card_place: u16) -> Self {
        Self {
            id,
            name,
            description: String::new(),
            card_place,
        }
    }

    /// Returns the card's ID.
    pub fn get_id(&self) -> u16 {
        self.id
    }

    /// Returns the card's name.
    pub fn get_name(&self) -> &str {
        self.name.as_str()
    }

    /// Returns the card's description.
    pub fn get_description(&self) -> &str {
        self.description.as_str()
    }

    /// Returns the card's position within its column.
    pub fn get_place(&self) -> u16 {
        self.card_place
    }

    pub(crate) fn rename(&mut self, new_name: &str) {
        self.name = new_name.to_owned();
    }

    pub(crate) fn change_description(&mut self, new_description: &str) {
        self.description = new_description.to_owned();
    }

    pub(crate) fn move_to(&mut self, new_place: u16) {
        self.card_place = new_place;
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn rename() {
        let mut card = Card::new(0, String::from("A"), 0);
        card.rename("B");

        assert_eq!(card.get_name(), "B");
    }

    #[test]
    fn change_description() {
        let mut card = Card::new(0, String::from("A"), 0);
        card.change_description("D");

        assert_eq!(card.get_description(), "D");
    }

    #[test]
    fn move_to() {
        let mut card = Card::new(0, String::from("A"), 0);
        card.move_to(1);

        assert_eq!(card.get_place(), 1);
    }
}
