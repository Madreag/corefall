use serde_json::Value;

#[derive(Debug, Clone, Copy)]
pub(crate) enum ExpectOp {
    Eq,
    Ge,
    Le,
}

#[derive(Debug)]
pub(crate) struct Expect<'a> {
    pub(crate) key: &'a str,
    pub(crate) op: ExpectOp,
    pub(crate) value: &'a str,
}

pub(crate) fn parse_expect(raw: &str) -> Option<Expect<'_>> {
    if let Some((k, v)) = raw.split_once(">=") {
        return Some(Expect {
            key: k.trim(),
            op: ExpectOp::Ge,
            value: v.trim(),
        });
    }
    if let Some((k, v)) = raw.split_once("<=") {
        return Some(Expect {
            key: k.trim(),
            op: ExpectOp::Le,
            value: v.trim(),
        });
    }
    raw.split_once('=').map(|(k, v)| Expect {
        key: k.trim(),
        op: ExpectOp::Eq,
        value: v.trim(),
    })
}

pub(crate) fn json_as_f64(value: &Value) -> Option<f64> {
    match value {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse().ok(),
        _ => None,
    }
}
