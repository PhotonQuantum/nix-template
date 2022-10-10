use xshell::{cmd, Shell};

use crate::Result;

pub fn commit_of_git(url: String, rev: String) -> Result<String> {
    eprintln!("Fetching commit of {url}#{rev}");

    let sh = Shell::new()?;
    let temp_dir = sh.create_temp_dir()?;
    let temp_path = temp_dir.path();
    sh.change_dir(temp_path);

    cmd!(sh, "git init").ignore_stdout().ignore_stderr().quiet().run()?;
    cmd!(sh, "git remote add origin {url}").ignore_stderr().quiet().run()?;
    let remotes = cmd!(sh, "git ls-remote origin {rev}").quiet().read()?;
    remotes.lines().next()
        .and_then(|line| line.split("\t").next())
        .map(|commit| commit.to_string())
        .ok_or_else(|| format!("Could not find commit for rev {rev} in {url}").into())
}

pub fn commit_of_github(owner: String, repo: String, rev: String) -> Result<String> {
    commit_of_git(format!("https://github.com/{owner}/{repo}.git"), rev)
}

pub fn hash_from_git(url: String, rev: String) -> Result<String> {
    eprintln!("Calculating nix hash for {url}#{rev}");

    let sh = Shell::new()?;
    let temp_dir = sh.create_temp_dir()?;
    let temp_path = temp_dir.path();
    sh.change_dir(temp_path);

    cmd!(sh, "git init").ignore_stdout().ignore_stderr().quiet().run()?;
    cmd!(sh, "git remote add origin {url}").ignore_stderr().quiet().run()?;
    cmd!(sh, "git fetch --depth 1 origin {rev}").ignore_stderr().quiet().run()?;
    cmd!(sh, "git checkout FETCH_HEAD").ignore_stderr().quiet().run()?;

    sh.remove_path(".git")?;
    Ok(cmd!(sh, "nix hash path --type sha256 --base64 {temp_path}")
        .quiet()
        .read()?)
}

pub fn hash_from_github(owner: String, repo: String, rev: String) -> Result<String> {
    hash_from_git(format!("https://github.com/{owner}/{repo}.git"), rev)
}
