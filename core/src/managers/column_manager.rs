use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::errors::column_error::ColumnError;
use crate::models::column::Column;

/// Stores and manages the columns belonging to a single list.
#[derive(Serialize, Deserialize)]
pub(crate) struct ColumnManager {
    columns: HashMap<u8, Column>,
    next_id: u16,
}

impl ColumnManager {
    /// Creates an empty column manager.
    pub(crate) fn new() -> Self {
        Self {
            columns: HashMap::new(),
            next_id: 0,
        }
    }

    /// Adds a column and returns its generated ID.
    ///
    /// Leading and trailing whitespace is removed from `name`.
    ///
    /// Returns [`ColumnError::ColumnIdLimitReached`] when no more column
    /// IDs are available, [`ColumnError::ColumnPlaceLimitReached`] when no
    /// more column places are available, or [`ColumnError::EmptyName`] when
    /// the trimmed name is empty.
    pub(crate) fn add(&mut self, name: &str) -> Result<u8, ColumnError> {
        if self.next_id > u16::from(u8::MAX) {
            return Err(ColumnError::ColumnIdLimitReached);
        }

        if self.columns.len() > u8::MAX as usize {
            return Err(ColumnError::ColumnPlaceLimitReached);
        }

        let column_name = name.trim().to_string();
        if column_name.is_empty() {
            return Err(ColumnError::EmptyName);
        }

        let column_id = self.next_id as u8;
        let column = Column::new(column_id, column_name, self.columns.len() as u8);
        self.columns.insert(column_id, column);
        self.next_id += 1;
        Ok(column_id)
    }

    /// Removes the column with the given ID.
    ///
    /// Returns the removed column, or `None` if no column has that ID.
    pub(crate) fn remove(&mut self, id: u8) -> Option<Column> {
        let removed_column = self.columns.remove(&id)?;

        let removed_column_place = removed_column.get_place();

        for column in self.columns.values_mut() {
            if column.get_place() > removed_column_place {
                column.move_to(column.get_place() - 1);
            }
        }

        Some(removed_column)
    }

    /// Finds a column by its ID.
    ///
    /// Returns `None` if no column has the given ID.
    pub(crate) fn get_by_id(&self, id: u8) -> Option<&Column> {
        self.columns.get(&id)
    }

    pub(crate) fn get_by_id_mut(&mut self, id: u8) -> Option<&mut Column> {
        self.columns.get_mut(&id)
    }

    /// Finds all columns whose names exactly match `name`.
    pub(crate) fn get_by_name(&self, name: &str) -> Vec<&Column> {
        let mut matching_columns = Vec::new();

        for column in self.columns.values() {
            if column.get_name() == name {
                matching_columns.push(column);
            }
        }

        matching_columns
    }

    /// Renames the column with the given ID.
    ///
    /// Leading and trailing whitespace is removed from `new_name`.
    ///
    /// Returns [`ColumnError::EmptyName`] if the trimmed name is empty, or
    /// [`ColumnError::ColumnNotFound`] if no column has the given ID.
    pub(crate) fn rename(&mut self, id: u8, new_name: &str) -> Result<(), ColumnError> {
        let name = new_name.trim();
        if name.is_empty() {
            return Err(ColumnError::EmptyName);
        }

        match self.columns.get_mut(&id) {
            Some(column) => {
                column.rename(name);
                Ok(())
            }
            None => Err(ColumnError::ColumnNotFound),
        }
    }

    /// Moves a column to a new position and returns its final position.
    ///
    /// Columns between the old and new positions are shifted to preserve
    /// consecutive positions.
    ///
    /// Returns [`ColumnError::ColumnNotFound`] if no column has the given ID,
    /// or [`ColumnError::InvalidColumnPlace`] if `new_place` is outside the
    /// current range of column positions.
    pub(crate) fn move_to(&mut self, id: u8, new_place: u8) -> Result<u8, ColumnError> {
        let old_place = match self.get_by_id(id) {
            Some(column) => column.get_place(),
            None => return Err(ColumnError::ColumnNotFound),
        };

        if old_place == new_place {
            return Ok(old_place);
        }

        if usize::from(new_place) >= self.columns.len() {
            return Err(ColumnError::InvalidColumnPlace);
        }

        let move_left = new_place < old_place;
        let _move_right = new_place > old_place;

        for column in self.columns.values_mut() {
            if move_left {
                if column.get_place() >= new_place && column.get_place() < old_place {
                    column.move_to(column.get_place() + 1);
                }
            } else {
                if column.get_place() <= new_place && column.get_place() > old_place {
                    column.move_to(column.get_place() - 1);
                }
            }
        }

        let column = self.columns.get_mut(&id).unwrap();
        column.move_to(new_place);

        Ok(new_place)
    }
}

impl Default for ColumnManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add() {
        let mut manager = ColumnManager::new();

        let result = manager.add("A");

        assert_eq!(result, Ok(0))
    }

    #[test]
    fn add_empty_name() {
        let mut manager = ColumnManager::new();

        let result = manager.add("   ");

        assert_eq!(result, Err(ColumnError::EmptyName));
    }

    #[test]
    fn get_by_id() {
        let mut manager = ColumnManager::new();

        let result = manager.add("A");

        assert!(manager.get_by_id(result.unwrap()).is_some());
    }

    #[test]
    fn get_by_id_missing() {
        let manager = ColumnManager::new();

        assert!(manager.get_by_id(0).is_none());
    }

    #[test]
    fn get_by_name() {
        let mut manager = ColumnManager::new();
        manager.add("A").unwrap();

        let result = manager.get_by_name("A");

        assert!(!result.is_empty())
    }

    #[test]
    fn get_by_name_missing() {
        let manager = ColumnManager::new();

        assert!(manager.get_by_name("A").is_empty());
    }

    #[test]
    fn remove() {
        let mut manager = ColumnManager::new();
        let id = manager.add("A").unwrap();
        let to_be_moved = manager.add("B").unwrap();

        let result = manager.remove(id);

        assert!(result.is_some());
        assert!(manager.get_by_id(id).is_none());
        assert_eq!(manager.get_by_id(to_be_moved).unwrap().get_place(), 0);
    }

    #[test]
    fn remove_missing() {
        let mut manager = ColumnManager::new();

        let result = manager.remove(0);

        assert!(result.is_none());
    }

    #[test]
    fn rename() {
        let mut manager = ColumnManager::new();
        let id = manager.add("A").unwrap();
        manager.rename(id, "B").unwrap();

        let renamed_column = manager.get_by_id(id).unwrap();

        assert_eq!(renamed_column.get_name(), "B");
    }

    #[test]
    fn rename_missing() {
        let mut manager = ColumnManager::new();
        let id = 0;

        let result = manager.rename(id, "B");

        assert_eq!(manager.columns.len(), 0);
        assert_eq!(result, Err(ColumnError::ColumnNotFound));
    }

    #[test]
    fn rename_empty_name() {
        let mut manager = ColumnManager::new();
        let id = manager.add("A").unwrap();

        let result = manager.rename(id, "  ");

        assert_eq!(result, Err(ColumnError::EmptyName));
    }

    #[test]
    fn move_to_left() {
        let mut manager = ColumnManager::new();
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
        let mut manager = ColumnManager::new();
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
        let mut manager = ColumnManager::new();
        manager.add("A").unwrap();

        let result = manager.move_to(0, 0);

        assert_eq!(result, Ok(0));
        assert_eq!(manager.get_by_id(0).unwrap().get_place(), 0);
    }

    #[test]
    fn move_to_invalid_column() {
        let mut manager = ColumnManager::new();
        manager.add("A").unwrap();
        manager.add("B").unwrap();
        manager.add("C").unwrap();

        let result = manager.move_to(10, 0);

        assert_eq!(result, Err(ColumnError::ColumnNotFound));
    }

    #[test]
    fn move_to_invalid_place() {
        let mut manager = ColumnManager::new();
        manager.add("A").unwrap();
        manager.add("B").unwrap();
        manager.add("C").unwrap();

        let result = manager.move_to(0, 10);

        assert_eq!(result, Err(ColumnError::InvalidColumnPlace));
    }
}
