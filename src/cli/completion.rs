use usage::{
    Args, Run,
    complete::Shell,
    install::{Env, OnForeign},
};

use super::Greet;

const ZSH_COMPLETION_INIT: &str =
    "(( $+functions[compdef] )) || { autoload -Uz compinit && compinit; }\n";

fn completion_script(shell: Shell) -> String {
    let mut script = Greet::completion_script(shell);
    if shell == Shell::Zsh {
        // `#compdef` must remain first so compinit can also autoload this file from fpath.
        let insertion_point = script
            .find('\n')
            .map(|index| index + 1)
            .expect("the generated Zsh completion has a #compdef line");
        script.insert_str(insertion_point, ZSH_COMPLETION_INIT);
    }
    script
}

fn quote_zsh(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

/// Generate completion script for specified shell
#[derive(Args)]
pub(crate) struct Completion {
    /// Which shell to generate for
    #[usage(long, choices("bash", "zsh", "fish"))]
    shell: String,

    /// Install instead of printing the completion script
    #[usage(long)]
    install: bool,
}

impl Run for Completion {
    type Output = ();

    fn run(self) {
        let shell = match self.shell.as_str() {
            "bash" => Shell::Bash,
            "zsh" => Shell::Zsh,
            _ => Shell::Fish,
        };
        let script = completion_script(shell);

        if !self.install {
            print!("{script}");
            return;
        }

        let env = Env::from_process();
        let plan = Greet::completion_install_plan(shell, &env).unwrap();
        let done = usage::install::write(&plan, &script, OnForeign::Refuse).unwrap();

        println!("installed to {}", done.plan.path.display());

        if shell == Shell::Zsh {
            println!(
                "add this to your shell's startup file, once:\nsource {}",
                quote_zsh(&done.plan.path.display().to_string())
            );
        } else if let Some(line) = done.plan.loading.instruction() {
            println!("add this to your shell's startup file, once:\n{line}");
        }
    }
}
