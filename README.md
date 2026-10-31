# Tuido
Simple terminal to-do app built with Ratatui. Organize tasks into lists with custom columns, sort them by card properties, save locally as JSON and synchronize with git using commands.

## Project Status
Project is under development. Code from old repository has been rebuilded.

## Features
### Minimal Viable Product:
- [x] Creating, deleting, and renaming lists
- [x] Creating, removing, and renaming columns (e.g. backlog, in progress, etc.)
- [x] Creating, deleting, and renaming cards in lists
- [x] Moving cards between columns
- [ ] Saving lists in separate JSON files after every change
- [ ] TUI using Ratatui
### Full Release:
- [ ] Importing and exporting lists to and from the user's desktop
- [ ] Adding, deleting, and renaming card properties (e.g. deadline date, priority, etc.)
- [ ] Sorting cards by their properties
- [ ] Manual Git synchronization (commit, push, and pull)

## Requirements
- Rust 1.85 or newer
- Cargo 1.99 or newer

## Building
Clone the repository:
```bash
git clone https://github.com/naxemis/tuido.git
cd tuido
```

Build the workspace:
```bash
cargo build
```

## Running
Run the application with:
```bash
cargo run -p tui
```

## Testing
Run all tests in this workspace:
```bash
cargo test --workspace --all-targets --all-features
```

Run tests with additional output:
```bash
cargo test --workspace -- --nocapture
```

## Development
Before opening a pull request, make sure that formatting, tests and Clippy checks pass.

To test it locally, run:
```bash
bash ./.github/scripts/check.sh
```

To automatically format the code and apply available Clippy fixes, run:
```bash
bash ./.github/scripts/fix.sh
```

## Contributing
1. Create a feature branch.
2. Implement the change and add or update tests.
3. Run the `check.sh` script to verify formatting, tests, and Clippy checks.
4. Update the documentation if the public behavior changes.
5. Open a pull request describing the change and how it was tested.