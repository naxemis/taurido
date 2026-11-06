# Taurido
Simple to-do app built with Tauri and Vue. Organize tasks into lists with custom columns, sort them by card properties, save locally as JSON.

## Project Status
Project is under development.

## Features
### Minimal Viable Product:
- [x] Creating, deleting, and renaming lists
- [x] Creating, removing, and renaming columns (e.g. backlog, in progress, etc.)
- [x] Creating, deleting, and renaming cards in lists
- [x] Moving cards between columns
- [x] Saving and loading lists from JSON file
- [ ] GUI using Tauri and Vue.js with TypeScript
### Full Release:
- [ ] Importing and exporting lists to and from the user's desktop
- [ ] Adding, deleting, and renaming card properties (e.g. deadline date, priority, etc.)
- [ ] Sorting cards by their properties

## Requirements
- Rust 1.85
- Cargo 1.99
- Node.js 26.13

## Building
Clone the repository:
```bash
git clone https://github.com/naxemis/taurido.git
cd taurido
```

Build the workspace:
```bash
cargo build
```

Build the desktop app:
```bash
cd gui
npm install
npm run tauri build
```

## Running
The GUI lives in `gui/`. From that directory, install the frontend dependencies once and start the app:

```bash
cd gui
npm install
npm run tauri dev
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