use std::path::PathBuf;

use serde_json::json;

#[derive(Default)]
pub(crate) struct ValidationReport {
    pub(crate) entries: Vec<Entry>,
}

#[derive(Debug)]
pub(crate) struct Entry {
    pub(crate) path: PathBuf,
    pub(crate) result: EntryResult,
    pub(crate) message: String,
}

#[derive(Debug)]
pub(crate) enum EntryResult {
    Pass,
    Warn,
    Fail,
}

impl ValidationReport {
    pub(crate) fn add_pass(&mut self, path: PathBuf, message: String) {
        self.entries.push(Entry {
            path,
            result: EntryResult::Pass,
            message,
        });
    }
    pub(crate) fn add_warn(&mut self, path: PathBuf, message: String) {
        self.entries.push(Entry {
            path,
            result: EntryResult::Warn,
            message,
        });
    }
    pub(crate) fn add_error(&mut self, path: PathBuf, message: String) {
        self.entries.push(Entry {
            path,
            result: EntryResult::Fail,
            message,
        });
    }
    pub(crate) fn pass(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| matches!(e.result, EntryResult::Pass))
            .count()
    }
    pub(crate) fn warn(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| matches!(e.result, EntryResult::Warn))
            .count()
    }
    pub(crate) fn fail(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| matches!(e.result, EntryResult::Fail))
            .count()
    }
    pub(crate) fn to_json(&self) -> serde_json::Value {
        json!({
            "schema_version": 1,
            "scanned": self.entries.len(),
            "pass": self.pass(),
            "warn": self.warn(),
            "fail": self.fail(),
            "entries": self
                .entries
                .iter()
                .map(|e| {
                    json!({
                        "path": e.path.display().to_string(),
                        "result": match e.result {
                            EntryResult::Pass => "pass",
                            EntryResult::Warn => "warn",
                            EntryResult::Fail => "fail",
                        },
                        "message": e.message,
                    })
                })
                .collect::<Vec<_>>()
        })
    }
}
