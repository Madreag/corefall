use std::{
    fs,
    io::Write,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

pub(crate) fn write_tmp(name: &str, contents: &str) -> PathBuf {
    let seq = TMP_COUNTER.fetch_add(1, Ordering::SeqCst);
    let pid = std::process::id();
    let dir = std::env::temp_dir().join(format!("cf-mod-test-{pid}-{seq}"));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    let mut f = fs::File::create(&path).unwrap();
    f.write_all(contents.as_bytes()).unwrap();
    path
}

pub(crate) fn next_seq() -> u64 {
    TMP_COUNTER.fetch_add(1, Ordering::SeqCst)
}
