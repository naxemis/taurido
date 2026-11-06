use serde::{Deserialize, Serialize};

use crate::errors::card_error::CardError;
use crate::errors::card_move_error::CardMoveError;
use crate::errors::column_error::ColumnError;
use crate::managers::column_manager::ColumnManager;
use crate::models::card::Card;
use crate::models::column::Column;

/// A to-do list identified by a numeric ID and a name.
#[derive(Serialize, Deserialize)]
pub struct List {
    id: u16,
    name: String,
    column_manager: ColumnManager,
}

impl List {
    /// Creates a list with the given ID and name.
    pub(crate) fn new(id: u16, name: String) -> Self {
        Self {
            id,
            name,
            column_manager: ColumnManager::new(),
        }
    }

    /// Returns the list's ID.
    pub fn get_id(&self) -> u16 {
        self.id
    }

    /// Returns the list's name.
    pub fn get_name(&self) -> &str {
        self.name.as_str()
    }

    /// Replaces the list's name with `new_name`.
    pub(crate) fn rename(&mut self, new_name: &str) {
        self.name = new_name.to_owned();
    }

    /// Adds a column to the list and returns its generated ID.
    ///
    /// Leading and trailing whitespace is removed from `name`.
    ///
    /// Returns [`ColumnError::ColumnIdLimitReached`] when no more column IDs
    /// are available, [`ColumnError::ColumnPlaceLimitReached`] when no more
    /// column places are available, or [`ColumnError::EmptyName`] when the
    /// trimmed name is empty.
    pub fn add_column(&mut self, name: &str) -> Result<u8, ColumnError> {
        self.column_manager.add(name)
    }

    /// Removes the column with the given ID.
    ///
    /// Returns the removed column, or [`ColumnError::ColumnNotFound`] if no
    /// column has that ID.
    pub fn remove_column(&mut self, id: u8) -> Result<Column, ColumnError> {
        self.column_manager
            .remove(id)
            .ok_or(ColumnError::ColumnNotFound)
    }

    /// Finds a column by its ID.
    ///
    /// Returns `None` if no column has that ID.
    pub fn get_column_by_id(&self, id: u8) -> Option<&Column> {
        self.column_manager.get_by_id(id)
    }

    pub(crate) fn get_column_by_id_mut(&mut self, id: u8) -> Option<&mut Column> {
        self.column_manager.get_by_id_mut(id)
    }

    /// Finds all columns whose names exactly match `name`.
    pub fn get_column_by_name(&self, name: &str) -> Vec<&Column> {
        self.column_manager.get_by_name(name)
    }

    /// Renames the column with the given ID.
    ///
    /// Leading and trailing whitespace is removed from `new_name`.
    ///
    /// Returns [`ColumnError::EmptyName`] if the trimmed name is empty, or
    /// [`ColumnError::ColumnNotFound`] if no column has that ID.
    pub fn rename_column(&mut self, id: u8, new_name: &str) -> Result<(), ColumnError> {
        self.column_manager.rename(id, new_name)
    }

    /// Moves a column to a new position and returns its final position.
    ///
    /// Columns between the old and new positions are shifted to preserve
    /// consecutive positions.
    ///
    /// Returns [`ColumnError::ColumnNotFound`] if no column has the given ID,
    /// or [`ColumnError::InvalidColumnPlace`] if `new_place` is outside the
    /// current range of column positions.
    pub fn move_column_to(&mut self, id: u8, new_place: u8) -> Result<u8, ColumnError> {
        self.column_manager.move_to(id, new_place)
    }

    /// Adds a new card to the end of the given column and returns its ID.
    ///
    /// Returns [`CardError::CardNotFound`] if no column has `column_id`.
    pub fn add_card(&mut self, column_id: u8, name: &str) -> Result<u16, CardError> {
        self.get_column_by_id_mut(column_id)
            .ok_or(CardError::CardNotFound)?
            .add_card(name)
    }

    /// Removes a card from the given column.
    ///
    /// Cards after the removed card are shifted one place to the left.
    /// Returns `None` if the column or card does not exist.
    pub fn remove_card(&mut self, column_id: u8, card_id: u16) -> Option<Card> {
        self.get_column_by_id_mut(column_id)?.remove_card(card_id)
    }

    /// Finds a card by its ID in the given column.
    pub fn get_card_by_id(&self, column_id: u8, card_id: u16) -> Option<&Card> {
        self.get_column_by_id(column_id)?.get_card_by_id(card_id)
    }

    /// Finds all cards whose names exactly match `name` in the given column.
    pub fn get_card_by_name(&self, column_id: u8, name: &str) -> Vec<&Card> {
        self.get_column_by_id(column_id)
            .map_or_else(Vec::new, |column| column.get_card_by_name(name))
    }

    /// Renames a card in the given column.
    ///
    /// Leading and trailing whitespace is removed from `new_name`.
    ///
    /// Returns [`CardError::CardNotFound`] if the column or card does not
    /// exist, or [`CardError::EmptyName`] if the trimmed name is empty.
    pub fn rename_card(
        &mut self,
        column_id: u8,
        card_id: u16,
        new_name: &str,
    ) -> Result<(), CardError> {
        self.get_column_by_id_mut(column_id)
            .ok_or(CardError::CardNotFound)?
            .rename_card(card_id, new_name)
    }

    /// Replaces a card's description in the given column.
    ///
    /// Returns [`CardError::CardNotFound`] if the column or card does not
    /// exist.
    pub fn change_card_description(
        &mut self,
        column_id: u8,
        card_id: u16,
        new_description: &str,
    ) -> Result<(), CardError> {
        self.get_column_by_id_mut(column_id)
            .ok_or(CardError::CardNotFound)?
            .change_card_description(card_id, new_description)
    }

    /// Moves a card to a new position within its column.
    ///
    /// Cards between the old and new positions are shifted to preserve
    /// consecutive positions.
    ///
    /// Returns the card's final position, or [`CardError`] if the column,
    /// card, or target position is invalid.
    pub fn move_card_in_column(
        &mut self,
        column_id: u8,
        card_id: u16,
        new_place: u16,
    ) -> Result<u16, CardError> {
        self.get_column_by_id_mut(column_id)
            .ok_or(CardError::CardNotFound)?
            .move_card_to(card_id, new_place)
    }

    /// Moves a card from one column to another.
    ///
    /// The card keeps its ID, name, and description. Its position is updated
    /// to `target_place` in the target column.
    ///
    /// Returns the ID of the target column when the card is moved successfully.
    ///
    /// Returns [`CardMoveError::SourceColumnNotFound`] if the source column
    /// does not exist, [`CardMoveError::TargetColumnNotFound`] if the target
    /// column does not exist, [`CardMoveError::CardNotFound`] if the card does
    /// not exist in the source column, [`CardMoveError::InvalidCardPlace`] if
    /// the target position is invalid, or
    /// [`CardMoveError::CardAlreadyInserted`] if the target column already
    /// contains a card with the same ID.
    pub fn move_card_to_column(
        &mut self,
        source_column_id: u8,
        card_id: u16,
        target_column_id: u8,
        target_place: u16,
    ) -> Result<u8, CardMoveError> {
        if self.get_column_by_id(source_column_id).is_none() {
            return Err(CardMoveError::SourceColumnNotFound);
        }

        if self.get_column_by_id(target_column_id).is_none() {
            return Err(CardMoveError::TargetColumnNotFound);
        }

        if self
            .get_column_by_id(source_column_id)
            .unwrap()
            .get_card_by_id(card_id)
            .is_none()
        {
            return Err(CardMoveError::CardNotFound);
        }

        if source_column_id == target_column_id {
            return self
                .move_card_in_column(source_column_id, card_id, target_place)
                .map(|_| source_column_id)
                .map_err(|error| match error {
                    CardError::CardNotFound => CardMoveError::CardNotFound,
                    CardError::InvalidCardPlace => CardMoveError::InvalidCardPlace,
                    _ => unreachable!(),
                });
        }

        let target_column = self.get_column_by_id(target_column_id).unwrap();
        target_column.can_insert_card(card_id, target_place)?;

        let removed_card = {
            let source_column = self.get_column_by_id_mut(source_column_id).unwrap();
            source_column.remove_card(card_id).unwrap()
        };

        let target_column = self.get_column_by_id_mut(target_column_id).unwrap();
        target_column.insert_card(removed_card, target_place)?;

        Ok(target_column_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rename() {
        let mut list = List::new(1, String::from("A"));
        list.rename("B");

        assert_eq!(list.get_name(), "B");
    }

    #[test]
    fn check_for_column() {
        let mut list = List::new(1, String::from("A"));

        assert!(!list.get_column_by_id(0).is_some());

        let column_id = list.add_column("B").unwrap();

        assert!(list.get_column_by_id(column_id).is_some());
    }

    #[test]
    fn move_card_to_column() {
        let mut list = List::new(1, String::from("List"));

        let source_column_id = list.add_column("Source").unwrap();
        let target_column_id = list.add_column("Target").unwrap();

        let card_id = list
            .get_column_by_id_mut(source_column_id)
            .unwrap()
            .add_card("Card")
            .unwrap();

        let result = list.move_card_to_column(source_column_id, card_id, target_column_id, 0);

        assert_eq!(result, Ok(target_column_id));

        assert!(
            list.get_column_by_id(source_column_id)
                .unwrap()
                .get_card_by_id(card_id)
                .is_none()
        );

        let moved_card = list
            .get_column_by_id(target_column_id)
            .unwrap()
            .get_card_by_id(card_id)
            .unwrap();

        assert_eq!(moved_card.get_id(), card_id);
        assert_eq!(moved_card.get_name(), "Card");
        assert_eq!(moved_card.get_place(), 0);
    }

    #[test]
    fn card_operations() {
        let mut list = List::new(1, String::from("List"));
        let column_id = list.add_column("Column").unwrap();

        let card_id = list.add_card(column_id, " Card ").unwrap();
        assert_eq!(
            list.get_card_by_id(column_id, card_id).unwrap().get_name(),
            "Card"
        );
        assert_eq!(list.get_card_by_name(column_id, "Card").len(), 1);

        list.rename_card(column_id, card_id, " Renamed ").unwrap();
        list.change_card_description(column_id, card_id, "Description")
            .unwrap();
        let card = list.get_card_by_id(column_id, card_id).unwrap();
        assert_eq!(card.get_name(), "Renamed");
        assert_eq!(card.get_description(), "Description");

        let second_card_id = list.add_card(column_id, "Second").unwrap();
        assert_eq!(
            list.move_card_in_column(column_id, second_card_id, 0),
            Ok(0)
        );
        assert_eq!(
            list.get_card_by_id(column_id, card_id).unwrap().get_place(),
            1
        );
        assert!(list.remove_card(column_id, card_id).is_some());
        assert!(list.get_card_by_id(column_id, card_id).is_none());
    }

    #[test]
    fn card_operations_preserve_card_errors() {
        let mut list = List::new(1, String::from("List"));
        let column_id = list.add_column("Column").unwrap();

        assert_eq!(list.add_card(column_id, "   "), Err(CardError::EmptyName));
        assert_eq!(
            list.rename_card(column_id, 0, "Name"),
            Err(CardError::CardNotFound)
        );
        assert_eq!(
            list.change_card_description(column_id, 0, "Description"),
            Err(CardError::CardNotFound)
        );
        assert_eq!(
            list.move_card_in_column(column_id, 0, 0),
            Err(CardError::CardNotFound)
        );
    }

    #[test]
    fn move_card_to_column_within_column_returns_invalid_place() {
        let mut list = List::new(1, String::from("List"));
        let column_id = list.add_column("Column").unwrap();
        let card_id = list.add_card(column_id, "Card").unwrap();

        assert_eq!(
            list.move_card_to_column(column_id, card_id, column_id, 1),
            Err(CardMoveError::InvalidCardPlace)
        );
    }
}
