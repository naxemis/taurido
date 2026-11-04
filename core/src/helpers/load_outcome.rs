use std::fmt;
use std::path::PathBuf;

use crate::managers::list_manager::ListManager;

pub enum LoadOutcome {
    Loaded(ListManager),
    Recovered {
        manager: ListManager,
        source: PathBuf,
    },
    Missing,
}

impl fmt::Debug for LoadOutcome {
    /// Prints the variant and, for recovery, the source path. `ListManager` has no
    /// `Debug`, so the loaded data is elided.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Loaded(_) => f.write_str("Loaded(..)"),
            Self::Recovered { source, .. } => {
                write!(f, "Recovered {{ source: {} }}", source.display())
            }
            Self::Missing => f.write_str("Missing"),
        }
    }
}
