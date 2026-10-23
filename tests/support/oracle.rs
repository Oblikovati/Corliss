//! The reference readers' output (`tests/fixtures/models.json`, written by the reference
//! implementation's export tooling) and a JSON comparison that reports the first difference.

use std::sync::OnceLock;

use serde_json::Value;

/// Relative tolerance for floats: Rust and Python evaluate the same formulas, but library
/// functions such as atan2 may differ in the last bit.
const FLOAT_TOLERANCE: f64 = 1e-9;

fn oracle() -> &'static Value {
    static ORACLE: OnceLock<Value> = OnceLock::new();
    ORACLE.get_or_init(|| {
        let path = super::fixtures_dir().join("models.json");
        let text = std::fs::read_to_string(path).expect("models.json is readable");
        serde_json::from_str(&text).expect("models.json parses")
    })
}

/// Fixture labels (`spec/as_mate.iam`) whose extension is `ext` ("ipt", "iam", "idw").
pub fn labels(ext: &str) -> Vec<String> {
    let Value::Object(map) = oracle() else {
        return Vec::new();
    };
    map.keys()
        .filter(|k| k.ends_with(&format!(".{ext}")))
        .cloned()
        .collect()
}

/// The reference output of `method` for fixture `label`.
pub fn expected(label: &str, method: &str) -> &'static Value {
    &oracle()[label][method]
}

/// Compares `actual` with the reference; returns the path and values of the first difference.
pub fn diff(actual: &Value, expected: &Value) -> Option<String> {
    diff_at("$", actual, expected)
}

fn diff_at(path: &str, a: &Value, e: &Value) -> Option<String> {
    match (a, e) {
        (Value::Number(x), Value::Number(y)) => {
            let (x, y) = (
                x.as_f64().unwrap_or(f64::NAN),
                y.as_f64().unwrap_or(f64::NAN),
            );
            let close = x == y || (x - y).abs() <= FLOAT_TOLERANCE * x.abs().max(y.abs()).max(1.0);
            (!close).then(|| format!("{path}: {x} != {y}"))
        }
        (Value::Array(x), Value::Array(y)) => {
            if x.len() != y.len() {
                return Some(format!("{path}: {} items != {} items", x.len(), y.len()));
            }
            x.iter()
                .zip(y)
                .enumerate()
                .find_map(|(k, (p, q))| diff_at(&format!("{path}[{k}]"), p, q))
        }
        (Value::Object(x), Value::Object(y)) => {
            let missing = y
                .keys()
                .find(|k| !x.contains_key(*k))
                .map(|k| format!("{path}.{k}: missing"));
            let extra = x
                .keys()
                .find(|k| !y.contains_key(*k))
                .map(|k| format!("{path}.{k}: not in the reference"));
            missing.or(extra).or_else(|| {
                y.iter()
                    .find_map(|(k, q)| diff_at(&format!("{path}.{k}"), &x[k], q))
            })
        }
        _ => (a != e).then(|| format!("{path}: {a} != {e}")),
    }
}

/// Runs `actual(label)` for every fixture with extension `ext` and compares it with `method`'s
/// reference output; panics with every difference found.
pub fn check_all(ext: &str, method: &str, actual: impl Fn(&str) -> Value) {
    let mut failures = Vec::new();
    for label in labels(ext) {
        let want = expected(&label, method);
        if want.get("error").is_some() && want.as_object().is_some_and(|o| o.len() == 1) {
            continue;
        }
        if let Some(d) = diff(&actual(&label), want) {
            failures.push(format!("{label}: {d}"));
        }
    }
    let shown: Vec<&str> = failures.iter().take(25).map(String::as_str).collect();
    assert!(
        failures.is_empty(),
        "{method}: {} fixtures differ\n{}",
        failures.len(),
        shown.join("\n")
    );
}
