pub(crate) fn compact_json_payload(value: &serde_json::Value) -> String {
    match serde_json::to_string(value) {
        Ok(s) => {
            // Markdown table cells render best when the payload is short.
            if s.len() > 96 {
                let truncated: String = s.chars().take(95).collect();
                format!("{truncated}...")
            } else {
                s
            }
        }
        Err(_) => "<unserializable>".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_json_payload_truncates_on_utf8_char_boundaries() {
        let value = serde_json::json!({
            "message": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa界界界"
        });

        let compact = compact_json_payload(&value);

        assert!(compact.ends_with("..."));
        assert!(compact.is_char_boundary(compact.len()));
        assert!(compact.chars().count() <= 98);
    }
}
