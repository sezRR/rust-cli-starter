use usage::{Args, Run};

/// Greet someone
#[derive(Args)]
pub(crate) struct Hello {
    /// Who to greet
    // `env` has to be a string literal; rename it along with the package name.
    #[usage(env = "GREET_NAME", default = "world")]
    name: String,
}

impl Run for Hello {
    type Output = ();
    fn run(self) {
        println!("hello, {}", self.name);
    }
}
