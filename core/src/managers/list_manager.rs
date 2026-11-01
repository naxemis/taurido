use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::errors::list_error::ListError;
use crate::models::list::List;

/// Stores and manages the lists in the application.
#[derive(Serialize, Deserialize)]
pub struct ListManager {
    lists: HashMap<u16, List>,
    next_id: u32,
}

impl ListManager {
    /// Creates an empty list manager.
    pub fn new() -> Self {
        Self {
            lists: HashMap::new(),
            next_id: 0,
        }
    }

    /// Adds a list and returns its generated ID.
    ///
    /// Leading and trailing whitespace is removed from `name`.
    ///
    /// Returns [`ListError::ListIdLimitReached`] when no more list IDs are available.
    pub fn add(&mut self, name: &str) -> Result<u16, ListError> {
        if self.next_id > u32::from(u16::MAX) {
            return Err(ListError::ListIdLimitReached);
        }

        let list_name = name.trim().to_string();
        if list_name.is_empty() {
            return Err(ListError::EmptyName);
        }

        let list_id = self.next_id as u16;
        let list = List::new(list_id, list_name);
        self.lists.insert(list_id, list);
        self.next_id += 1;
        Ok(list_id)
    }

    /// Removes the list with the given ID.
    ///
    /// Returns the removed list, or `None` if no list has that ID.
    pub fn remove(&mut self, id: u16) -> Option<List> {
        self.lists.remove(&id)
    }

    /// Finds a list by its ID.
    ///
    /// Returns `None` if no list has the given ID.
    pub fn get_by_id(&self, id: u16) -> Option<&List> {
        self.lists.get(&id)
    }

    /// Finds all lists whose names exactly match `name`.
    pub fn get_by_name(&self, name: &str) -> Vec<&List> {
        let mut matching_lists = Vec::new();

        for list in self.lists.values() {
            if list.get_name() == name {
                matching_lists.push(list);
            }
        }

        matching_lists
    }

    /// Renames the list with the given ID.
    ///
    /// Leading and trailing whitespace is removed from `new_name`.
    ///
    /// Returns [`ListError::EmptyName`] if the trimmed name is empty, or
    /// [`ListError::ListNotFound`] if no list has the given ID.
    pub fn rename(&mut self, id: u16, new_name: &str) -> Result<(), ListError> {
        let name = new_name.trim();
        if name.is_empty() {
            return Err(ListError::EmptyName);
        }

        match self.lists.get_mut(&id) {
            Some(list) => {
                list.rename(name);
                Ok(())
            }
            None => Err(ListError::ListNotFound),
        }
    }
}

impl Default for ListManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add() {
        let mut manager = ListManager::new();

        let result = manager.add("A");

        assert_eq!(result, Ok(0));
    }

    #[test]
    fn add_empty_name() {
        let mut manager = ListManager::new();

        let result = manager.add("");

        assert_eq!(result, Err(ListError::EmptyName));
    }

    #[test]
    fn get_by_id() {
        let mut manager = ListManager::new();

        let result = manager.add("A");

        assert!(manager.get_by_id(result.unwrap()).is_some());
    }

    #[test]
    fn get_by_id_missing() {
        let manager = ListManager::new();

        assert!(manager.get_by_id(0).is_none());
    }

    #[test]
    fn get_by_name() {
        let mut manager = ListManager::new();
        manager.add("A").unwrap();

        let result = manager.get_by_name("A");

        assert!(!result.is_empty());
    }

    #[test]
    fn get_by_name_missing() {
        let manager = ListManager::new();

        assert!(manager.get_by_name("A").is_empty());
    }

    #[test]
    fn remove() {
        let mut manager = ListManager::new();
        let id = manager.add("A").unwrap();

        let result = manager.remove(id);

        assert!(result.is_some());
        assert!(manager.get_by_id(id).is_none());
    }

    #[test]
    fn remove_missing() {
        let mut manager = ListManager::new();

        let result = manager.remove(0);

        assert!(result.is_none());
    }

    #[test]
    fn rename() {
        let mut manager = ListManager::new();
        let id = manager.add("A").unwrap();
        manager.rename(id, "B").unwrap();

        let result = manager.get_by_id(id).unwrap();

        assert_eq!(result.get_name(), "B");
    }

    #[test]
    fn rename_missing() {
        let mut manager = ListManager::new();
        let id = 0;

        let result = manager.rename(id, "B");

        assert_eq!(manager.lists.len(), 0);
        assert_eq!(result, Err(ListError::ListNotFound));
    }

    #[test]
    fn rename_empty_name() {
        let mut manager = ListManager::new();
        let id = manager.add("A").unwrap();

        let result = manager.rename(id, "  ");

        assert_eq!(result, Err(ListError::EmptyName));
    }
}
