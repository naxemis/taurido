use serde::{Deserialize, Serialize};

use crate::errors::card_error::CardError;
use crate::errors::card_move_error::CardMoveError;
use crate::managers::card_manager::CardManager;
use crate::models::card::Card;

/// A column in a to-do list identified by an ID, name, and position.
#[derive(Serialize, Deserialize)]
pub struct Column {
    id: u8,
    name: String,
    column_place: u8,
    card_manager: CardManager,
}

impl Column {
    /// Creates a column with the given ID, name, and position.
    pub(crate) fn new(id: u8, name: String, column_place: u8) -> Self {
        Self {
            id,
            name,
            column_place,
            card_manager: CardManager::new(),
        }
    }

    /// Returns the column's ID.
    pub fn get_id(&self) -> u8 {
        self.id
    }

    /// Returns the column's name.
    pub fn get_name(&self) -> &str {
        self.name.as_str()
    }

    /// Returns the column's position in the list.
    pub fn get_place(&self) -> u8 {
        self.column_place
    }

    /// Replaces the column's name with `new_name`.
    pub(crate) fn rename(&mut self, new_name: &str) {
        self.name = new_name.to_owned();
    }

    /// Changes the column's position to `new_place`.
    pub(crate) fn move_to(&mut self, new_place: u8) {
        self.column_place = new_place;
    }

    /// Adds a new card to the end of the column.
    ///
    /// Leading and trailing whitespace is removed from `name`.
    ///
    /// Returns the generated card ID, or a [`CardError`] if the name is empty
    /// or no more card IDs or positions are available.
    pub(crate) fn add_card(&mut self, name: &str) -> Result<u16, CardError> {
        self.card_manager.add(name)
    }

    pub(crate) fn can_insert_card(
        &self,
        card_id: u16,
        target_place: u16,
    ) -> Result<(), CardMoveError> {
        self.card_manager.can_insert_card(card_id, target_place)
    }

    /// Inserts an existing card at `target_place`.
    ///
    /// Cards at or after the target position are shifted one place to the
    /// right. The inserted card keeps its ID, name, and description.
    pub(crate) fn insert_card(
        &mut self,
        card: Card,
        target_place: u16,
    ) -> Result<u16, CardMoveError> {
        self.card_manager.insert(card, target_place)
    }

    /// Removes a card from the column.
    ///
    /// Cards after the removed card are shifted one place to the left.
    /// Returns the removed card, or `None` if no card has the given ID.
    pub(crate) fn remove_card(&mut self, id: u16) -> Option<Card> {
        self.card_manager.remove(id)
    }

    /// Finds a card by its ID.
    ///
    /// Returns `None` if no card has the given ID.
    pub fn get_card_by_id(&self, id: u16) -> Option<&Card> {
        self.card_manager.get_by_id(id)
    }

    /// Finds all cards whose names exactly match `name`.
    pub fn get_card_by_name(&self, name: &str) -> Vec<&Card> {
        self.card_manager.get_by_name(name)
    }

    /// Renames a card.
    ///
    /// Leading and trailing whitespace is removed from `name`.
    pub(crate) fn rename_card(&mut self, id: u16, name: &str) -> Result<(), CardError> {
        self.card_manager.rename(id, name)
    }

    /// Replaces the description of a card.
    pub(crate) fn change_card_description(
        &mut self,
        id: u16,
        new_description: &str,
    ) -> Result<(), CardError> {
        self.card_manager.change_description(id, new_description)
    }

    pub(crate) fn move_card_to(&mut self, id: u16, new_place: u16) -> Result<u16, CardError> {
        self.card_manager.move_to(id, new_place)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn rename() {
        let mut column = Column::new(0, String::from("A"), 0);
        column.rename("B");

        assert_eq!(column.get_name(), "B");
    }

    #[test]
    fn move_to() {
        let mut column = Column::new(0, String::from("A"), 0);
        column.move_to(1);

        assert_eq!(column.get_place(), 1);
    }
}
