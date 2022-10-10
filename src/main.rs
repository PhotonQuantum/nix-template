use std::error::Error;
use std::io::Read;
use std::path::PathBuf;
use std::{fs, io};

use clap::Parser;
use minijinja::{context, Environment};

use utils::{commit_of_git, commit_of_github, hash_from_git, hash_from_github};

mod utils;

type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

const AVAILABLE_FUNCTIONS: &str = r#"Available functions

`commit_of_git(url, rev)` - returns the commit hash of given git url and rev.
`commit_of_github(owner, repo, rev)` - returns the commit hash of given repo and rev.
`hash_from_git(url, rev)` - returns the sha256 hash of given git url and rev.
`hash_from_github(owner, repo, rev)` - returns the sha256 hash of given repo and rev."#;

/// Utility to instantiate a nix file template.
///
/// This utility accepts a valid `minijinja` template and outputs a file into stdout.
#[derive(Parser)]
#[command(author, version, about, after_long_help = AVAILABLE_FUNCTIONS)]
struct Args {
    /// The template to instantiate. If not specified, stdin is used.
    filename: Option<PathBuf>,
    #[arg(short, long)]
    variable: Vec<String>,
}

macro_rules! f {
    ($func: ident($($arg: ident),*)) => {
        |$($arg),*| $func ( $($arg),* ).map_err(|e|
            minijinja::Error::new(minijinja::ErrorKind::InvalidOperation, e.to_string())
        )
    };
}

fn main() -> Result<()> {
    let args = Args::parse();
    let template = if let Some(filename) = args.filename {
        fs::read_to_string(filename)?
    } else {
        // TODO #[unstable(feature = "io_read_to_string", issue = "80218")]
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf)?;
        buf
    };

    let mut env = Environment::new();
    env.add_function("commit_of_git", f!(commit_of_git(url, rev)));
    env.add_function("hash_from_git", f!(hash_from_git(url, rev)));
    env.add_function("commit_of_github", f!(commit_of_github(owner, repo, rev)));
    env.add_function("hash_from_github", f!(hash_from_github(owner, repo, rev)));

    let result = env.render_str(&template, context!())?;
    println!("{}", result);

    Ok(())
}
