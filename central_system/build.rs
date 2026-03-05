use git_version::git_version;

fn main() {
    println!(
        "cargo:rustc-env=GIT_COMMIT_HASH={}",
        git_version!(args = ["--abbrev=40", "--always", "--dirty=-dirty"])
    );
}
