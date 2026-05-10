fn main() {
    let info = cf_control::runtime::git_worktree_info();
    if let Some(fingerprint) = info.fingerprint {
        println!("{fingerprint}");
    }
}
