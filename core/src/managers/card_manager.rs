use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::errors::card_error::CardError;
use crate::errors::card_move_error::CardMoveError;
use crate::models::card::Card;

#[derive(Serialize, Deserialize)]
pub(crate) struct CardManager {
    cards: HashMap<u16, Card>,
    next_id: u32,
}

impl CardManager {
    pub(crate) fn new() -> Self {
        Self {
            cards: HashMap::new(),
            next_id: 0,
        }
    }

    pub(crate) fn add(&mut self, name: &str) -> Result<u16, CardError> {
        if self.next_id > u32::from(u16::MAX) {
            return Err(CardError::CardIdLimitReached);
        }

        if self.cards.len() > u16::MAX as usize {
            return Err(CardError::CardPlaceLimitReached);
        }

        let card_name = name.trim().to_string();
        if card_name.is_empty() {
            return Err(CardError::EmptyName);
        }

        let card_id = self.next_id as u16;
        let card = Card::new(card_id, card_name, self.cards.len() as u16);
        self.cards.insert(card_id, card);
        self.next_id += 1;
        Ok(card_id)
    }

    pub(crate) fn can_insert_card(
        &self,
        card_id: u16,
        target_place: u16,
    ) -> Result<(), CardMoveError> {
        if usize::from(target_place) > self.cards.len() {
            return Err(CardMoveError::InvalidCardPlace);
        }

        if self.cards.contains_key(&card_id) {
            return Err(CardMoveError::CardAlreadyInserted);
        }

        Ok(())
    }

    pub(crate) fn insert(
        &mut self,
        mut card: Card,
        target_place: u16,
    ) -> Result<u16, CardMoveError> {
        self.can_insert_card(card.get_id(), target_place)?;

        for card in self.cards.values_mut() {
            if card.get_place() >= target_place {
                let new_place = card.get_place() + 1;
                card.move_to(new_place);
            }
        }

        let card_id = card.get_id();
        card.move_to(target_place);
        self.cards.insert(card_id, card);

        Ok(card_id)
    }

    pub(crate) fn remove(&mut self, id: u16) -> Option<Card> {
        let removed_card = self.cards.remove(&id)?;

        let removed_card_place = removed_card.get_place();

        for card in self.cards.values_mut() {
            if card.get_place() > removed_card_place {
                card.move_to(card.get_place() - 1);
            }
        }

        Some(removed_card)
    }

    pub(crate) fn get_by_id(&self, id: u16) -> Option<&Card> {
        self.cards.get(&id)
    }

    pub(crate) fn get_by_name(&self, name: &str) -> Vec<&Card> {
        let mut matching_cards = Vec::new();

        for card in self.cards.values() {
            if card.get_name() == name {
                matching_cards.push(card);
            }
        }

        matching_cards
    }

    pub(crate) fn rename(&mut self, id: u16, new_name: &str) -> Result<(), CardError> {
        let name = new_name.trim();
        if name.is_empty() {
            return Err(CardError::EmptyName);
        }

        match self.cards.get_mut(&id) {
            Some(card) => {
                card.rename(name);
                Ok(())
            }
            None => Err(CardError::CardNotFound),
        }
    }

    pub(crate) fn change_description(
        &mut self,
        id: u16,
        new_description: &str,
    ) -> Result<(), CardError> {
        match self.cards.get_mut(&id) {
            Some(card) => {
                card.change_description(new_description);
                Ok(())
            }
            None => Err(CardError::CardNotFound),
        }
    }

    fn is_valid_place_for_move(&self, new_place: u16) -> bool {
        if usize::from(new_place) >= self.cards.len() {
            return false;
        }
        true
    }

    pub(crate) fn move_to(&mut self, id: u16, new_place: u16) -> Result<u16, CardError> {
        let old_place = match self.get_by_id(id) {
            Some(card) => card.get_place(),
            None => return Err(CardError::CardNotFound),
        };

        if old_place == new_place {
            return Ok(old_place);
        }

        if !self.is_valid_place_for_move(new_place) {
            return Err(CardError::InvalidCardPlace);
        }

        let move_left = new_place < old_place;
        let _move_right = new_place > old_place;

        for card in self.cards.values_mut() {
            if move_left {
                if card.get_place() >= new_place && card.get_place() < old_place {
                    card.move_to(card.get_place() + 1);
                }
            } else {
                if card.get_place() <= new_place && card.get_place() > old_place {
                    card.move_to(card.get_place() - 1);
                }
            }
        }

        let card = self.cards.get_mut(&id).unwrap();
        card.move_to(new_place);

        Ok(new_place)
    }
}

impl Default for CardManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add() {
        let mut manager = CardManager::new();

        let result = manager.add("A");

        assert_eq!(result, Ok(0));
    }

    #[test]
    fn add_empty_name() {
        let mut manager = CardManager::new();

        let result = manager.add("   ");

        assert_eq!(result, Err(CardError::EmptyName));
    }

    #[test]
    fn insert() {
        let mut manager = CardManager::new();

        let existing_card = manager.add("A").unwrap();
        let card_to_insert = Card::new(1, String::from("A"), 0);

        let result = manager.insert(card_to_insert, 1);

        assert_eq!(result, Ok(1));
        assert_eq!(manager.cards.get(&existing_card).unwrap().get_place(), 0);
        assert_eq!(manager.cards.get(&result.unwrap()).unwrap().get_place(), 1);
    }

    #[test]
    fn insert_in_taken_place() {
        let mut manager = CardManager::new();

        let existing_card = manager.add("A").unwrap();
        let card_to_insert = Card::new(1, String::from("A"), 0);

        let result = manager.insert(card_to_insert, 0);

        assert_eq!(result, Ok(1));
        assert_eq!(manager.cards.get(&existing_card).unwrap().get_place(), 1);
        assert_eq!(manager.cards.get(&result.unwrap()).unwrap().get_place(), 0);
    }

    #[test]
    fn insert_invalid_place() {
        let mut manager = CardManager::new();

        let card_to_insert = Card::new(1, String::from("A"), 0);

        let result = manager.insert(card_to_insert, 2);

        assert_eq!(result, Err(CardMoveError::InvalidCardPlace))
    }

    #[test]
    fn insert_duplicate_id() {
        let mut manager = CardManager::new();

        manager.add("A").unwrap();
        let card_to_insert = Card::new(0, String::from("A"), 0);

        let result = manager.insert(card_to_insert, 0);

        assert_eq!(result, Err(CardMoveError::CardAlreadyInserted));
    }

    #[test]
    fn insert_check_description() {
        let mut manager = CardManager::new();

        manager.add("A").unwrap();
        let mut card_to_insert = Card::new(1, String::from("A"), 0);
        card_to_insert.change_description("A");

        let result = manager.insert(card_to_insert, 0);

        assert_eq!(result, Ok(1));
        assert_eq!(
            manager
                .cards
                .get(&result.unwrap())
                .unwrap()
                .get_description(),
            "A"
        );
    }

    #[test]
    fn insert_invalid_place_does_not_change_manager() {
        let mut manager = CardManager::new();
        let existing_id = manager.add("A").unwrap();

        let card_to_insert = Card::new(1, String::from("B"), 0);
        let result = manager.insert(card_to_insert, 2);

        assert_eq!(result, Err(CardMoveError::InvalidCardPlace));
        assert_eq!(manager.cards.len(), 1);

        let existing_card = manager.get_by_id(existing_id).unwrap();
        assert_eq!(existing_card.get_name(), "A");
        assert_eq!(existing_card.get_place(), 0);
        assert!(manager.get_by_id(1).is_none());
    }

    #[test]
    fn get_by_id() {
        let mut manager = CardManager::new();

        let result = manager.add("A");

        assert!(manager.get_by_id(result.unwrap()).is_some());
    }

    #[test]
    fn get_by_id_missing() {
        let manager = CardManager::new();

        assert!(manager.get_by_id(0).is_none());
    }

    #[test]
    fn get_by_name() {
        let mut manager = CardManager::new();
        manager.add("A").unwrap();

        let result = manager.get_by_name("A");

        assert!(!result.is_empty())
    }

    #[test]
    fn get_by_name_missing() {
        let manager = CardManager::new();

        assert!(manager.get_by_name("A").is_empty());
    }

    #[test]
    fn remove() {
        let mut manager = CardManager::new();
        let id = manager.add("A").unwrap();
        let to_be_moved = manager.add("B").unwrap();

        let result = manager.remove(id);

        assert!(result.is_some());
        assert!(manager.get_by_id(id).is_none());
        assert_eq!(manager.get_by_id(to_be_moved).unwrap().get_place(), 0);
    }

    #[test]
    fn remove_missing() {
        let mut manager = CardManager::new();

        let result = manager.remove(0);

        assert!(result.is_none());
    }

    #[test]
    fn rename() {
        let mut manager = CardManager::new();
        let id = manager.add("A").unwrap();
        manager.rename(id, "B").unwrap();

        let renamed_card = manager.get_by_id(id).unwrap();

        assert_eq!(renamed_card.get_name(), "B");
    }

    #[test]
    fn rename_missing() {
        let mut manager = CardManager::new();

        let result = manager.rename(0, "B");

        assert_eq!(manager.cards.len(), 0);
        assert_eq!(result, Err(CardError::CardNotFound));
    }

    #[test]
    fn rename_empty_name() {
        let mut manager = CardManager::new();
        let id = manager.add("A").unwrap();

        let result = manager.rename(id, "  ");

        assert_eq!(result, Err(CardError::EmptyName));
    }

    #[test]
    fn change_description() {
        let mut manager = CardManager::new();
        let id = manager.add("A").unwrap();

        manager.change_description(id, "Description").unwrap();

        assert_eq!(
            manager.get_by_id(id).unwrap().get_description(),
            "Description"
        );
    }

    #[test]
    fn change_description_missing() {
        let mut manager = CardManager::new();

        let result = manager.change_description(0, "Description");

        assert_eq!(result, Err(CardError::CardNotFound));
    }

    #[test]
    fn move_to_left() {
        let mut manager = CardManager::new();
        manager.add("A").unwrap();
        manager.add("B").unwrap();
        manager.add("C").unwrap();

        let result = manager.move_to(2, 0).unwrap();

        assert_eq!(manager.get_by_id(0).unwrap().get_place(), 1);
        assert_eq!(manager.get_by_id(1).unwrap().get_place(), 2);
        assert_eq!(result, 0);
    }

    #[test]
    fn move_to_right() {
        let mut manager = CardManager::new();
        manager.add("A").unwrap();
        manager.add("B").unwrap();
        manager.add("C").unwrap();

        let result = manager.move_to(0, 2).unwrap();

        assert_eq!(result, 2);
        assert_eq!(manager.get_by_id(1).unwrap().get_place(), 0);
        assert_eq!(manager.get_by_id(2).unwrap().get_place(), 1);
    }

    #[test]
    fn move_to_same_place() {
        let mut manager = CardManager::new();
        manager.add("A").unwrap();

        let result = manager.move_to(0, 0);

        assert_eq!(result, Ok(0));
        assert_eq!(manager.get_by_id(0).unwrap().get_place(), 0);
    }

    #[test]
    fn move_to_invalid_card() {
        let mut manager = CardManager::new();
        manager.add("A").unwrap();

        let result = manager.move_to(10, 0);

        assert_eq!(result, Err(CardError::CardNotFound));
    }

    #[test]
    fn move_to_invalid_place() {
        let mut manager = CardManager::new();
        manager.add("A").unwrap();
        manager.add("B").unwrap();
        manager.add("C").unwrap();

        let result = manager.move_to(0, 10);

        assert_eq!(result, Err(CardError::InvalidCardPlace));
    }
}
