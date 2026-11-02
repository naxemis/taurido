// todo(startup): Single application instance
//   - Take OS advisory lock on tuido/lists.lock, keep it for the process lifetime
//   - If locked: print message and exit before any file operation
//   - If free: copy lists.json -> startup_lists.json, then load_lists()

// todo(tui): Save the whole ListManager after every list mutation

fn main() {
    println!("Hello, world!");
}
