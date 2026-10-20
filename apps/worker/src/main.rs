//! Sitolo worker binary.
//!
//! Runs the durable outbox relay. The processing engine is transport-neutral;
//! a concrete publisher and its configuration must be supplied by an approved
//! relay contract before this executable may start.
#![forbid(unsafe_code)]

fn main() {
    eprintln!(
        "sitolo-worker: refusing to start without an approved relay destination adapter and configuration"
    );
    std::process::exit(78);
}
