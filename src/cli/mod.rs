mod completion;
mod hello;
mod upgrade;

use completion::Completion;
use hello::Hello;
use upgrade::Upgrade;
use usage::{Cli, Subcommands};

/// Greets people, politely
#[derive(Cli)]
// `bin` has to be a string literal, so keep it in sync with the package name in
// Cargo.toml when you rename this CLI.
#[usage(bin = "greet", version = env!("CARGO_PKG_VERSION"), completion)]
pub(crate) struct Greet {
    #[usage(subcommand)]
    pub(crate) command: Commands,
}

#[derive(Subcommands)]
#[usage(run)] // Generate the match from each variant to its `Run` implementation.
pub(crate) enum Commands {
    Hello(Hello),
    Completion(Completion),
    Upgrade(Upgrade),
}
