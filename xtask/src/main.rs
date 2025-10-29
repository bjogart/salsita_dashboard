use xshell::Shell;

mod deploy;
mod test;

mod flags {
    xflags::xflags! {
        cmd xtask {
            // Run the repository's test suite.
            cmd test {}
            // Build `index.html` from repository files and print it to standard output
            cmd deploy {
                optional --dry-run
            }
        }
    }
}

fn main() -> anyhow::Result<()> {
    flags::Xtask::from_env()?.subcommand.run(&Shell::new()?)
}

impl flags::XtaskCmd {
    fn run(self, sh: &Shell) -> anyhow::Result<()> {
        match self {
            Self::Test(test) => test.run(sh),
            Self::Deploy(deploy) => deploy.run(sh),
        }
    }
}
