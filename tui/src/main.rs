// todo(startup): Single application instance
//   - Take OS advisory lock on tuido/lists.lock, keep it for the process lifetime
//   - If locked: print message and exit before any file operation
//   - If free: copy lists.json -> startup_lists.json, then load_lists()

// todo(tui): Save the whole ListManager after every list mutation

// todo(tui): Localized error messages (after finish feat/json-list-storage)
//   - Trigger: a second language, or the same wording repeated in several places
//   - Catalog: tui/locales/*.json, embedded with include_str!, keys -> template with placeholders
//   - Mapping: StorageError variant -> key stays in one function in this layer
//   - Constraint: core holds no user-facing text; error variants keep data, not formatted strings

fn main() {
    println!("Hello, world!");
}
