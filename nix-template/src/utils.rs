use eyre::eyre;
use xshell::{cmd, Shell};

use nix_template_macros::helper_func;

use crate::spinner::with_global_progress_bar;
use crate::Result;
use console::Emoji;

const EMOJI_FETCH: Emoji = Emoji("📥 ", "");
const EMOJI_HASH: Emoji = Emoji("🔑 ", "");

/// Returns the commit hash of given git url and rev.
#[helper_func(cached)]
fn commit_of_git(url: &str, rev: &str) -> Result<String> {
    with_global_progress_bar(|pb| {
        pb.set_message(format!("{EMOJI_FETCH}Fetching commit of {url}#{rev}"))
    });

    let sh = Shell::new()?;
    let temp_dir = sh.create_temp_dir()?;
    let temp_path = temp_dir.path();
    sh.change_dir(temp_path);

    cmd!(sh, "git init")
        .ignore_stdout()
        .ignore_stderr()
        .quiet()
        .run()?;
    cmd!(sh, "git remote add origin {url}")
        .ignore_stderr()
        .quiet()
        .run()?;
    let remotes = cmd!(sh, "git ls-remote origin {rev}").quiet().read()?;
    remotes
        .lines()
        .next()
        .and_then(|line| line.split('\t').next())
        .map(std::string::ToString::to_string)
        .ok_or_else(|| eyre!("Could not find commit for rev {rev} in {url}"))
}

/// Returns the commit hash of given repo and rev.
#[helper_func]
fn commit_of_github(owner: &str, repo: &str, rev: &str) -> Result<String> {
    Ok(commit_of_git(
        &format!("https://github.com/{owner}/{repo}.git"),
        rev,
    )?)
}

/// Returns the sha256 hash of given git url and rev.
#[helper_func(cached)]
fn hash_from_git(url: &str, rev: &str) -> Result<String> {
    with_global_progress_bar(|pb| {
        pb.set_message(format!("{EMOJI_HASH}Calculating nix hash for {url}#{rev}"))
    });

    let sh = Shell::new()?;
    let temp_dir = sh.create_temp_dir()?;
    let temp_path = temp_dir.path();
    sh.change_dir(temp_path);

    cmd!(sh, "git init")
        .ignore_stdout()
        .ignore_stderr()
        .quiet()
        .run()?;
    cmd!(sh, "git remote add origin {url}")
        .ignore_stderr()
        .quiet()
        .run()?;
    cmd!(sh, "git fetch --depth 1 origin {rev}")
        .ignore_stderr()
        .quiet()
        .run()?;
    cmd!(sh, "git checkout FETCH_HEAD")
        .ignore_stderr()
        .quiet()
        .run()?;

    sh.remove_path(".git")?;
    Ok(cmd!(sh, "nix hash path --type sha256 --base64 {temp_path}")
        .quiet()
        .read()?)
}

/// Returns the sha256 hash of given repo and rev.
#[helper_func]
fn hash_from_github(owner: &str, repo: &str, rev: &str) -> Result<String> {
    Ok(hash_from_git(
        &format!("https://github.com/{owner}/{repo}.git"),
        rev,
    )?)
}
