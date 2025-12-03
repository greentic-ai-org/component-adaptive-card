use serde_json::Value;

use crate::render::BindingContext;

/// Minimal expression engine interface so richer engines can be plugged in later.
pub trait ExpressionEngine: Send + Sync {
    /// Evaluate an expression string against the binding context.
    /// Returns None on invalid expressions or when no resolution is possible.
    fn eval(&self, expr: &str, ctx: &BindingContext) -> Option<Value>;
}

/// Default lightweight engine supporting:
/// - Path lookups (payload/session/state/params) via dotted notation.
/// - Equality on scalar values using `==`.
/// - Simple ternary `cond ? a : b`.
/// - Graceful failure: returns None for unknown expressions or missing paths.
#[derive(Default)]
pub struct SimpleExpressionEngine;

impl ExpressionEngine for SimpleExpressionEngine {
    fn eval(&self, expr: &str, ctx: &BindingContext) -> Option<Value> {
        let trimmed = expr.trim();
        // Ternary: cond ? a : b
        if let Some((cond_raw, rest)) = split_top_level(trimmed, '?') {
            let (then_raw, else_raw) = split_top_level(rest, ':')?;
            let cond_val = self.eval(cond_raw.trim(), ctx)?;
            let branch = if truthy(&cond_val) {
                self.eval(then_raw.trim(), ctx)?
            } else {
                self.eval(else_raw.trim(), ctx)?
            };
            return Some(branch);
        }

        // Equality
        if let Some((left, right)) = split_equality(trimmed) {
            let l = eval_atom(left.trim(), ctx)?;
            let r = eval_atom(right.trim(), ctx)?;
            return Some(Value::Bool(equals(&l, &r)));
        }

        eval_atom(trimmed, ctx)
    }
}

fn eval_atom(expr: &str, ctx: &BindingContext) -> Option<Value> {
    // Path forms: @{path} or ${path} or bare path
    if let Some(path) = expr.strip_prefix("@{").and_then(|s| s.strip_suffix('}')) {
        return ctx.lookup(path.trim());
    }
    if let Some(path) = expr.strip_prefix("${").and_then(|s| s.strip_suffix('}')) {
        return ctx.lookup(path.trim());
    }
    if expr.starts_with('@') || expr.starts_with('$') {
        return ctx.lookup(expr.trim_start_matches(&['@', '$'][..]));
    }

    // Literals
    if let Ok(n) = expr.parse::<f64>() {
        return serde_json::Number::from_f64(n).map(Value::Number);
    }
    if expr.eq_ignore_ascii_case("true") {
        return Some(Value::Bool(true));
    }
    if expr.eq_ignore_ascii_case("false") {
        return Some(Value::Bool(false));
    }
    if expr.eq_ignore_ascii_case("null") {
        return Some(Value::Null);
    }
    if expr.starts_with('"') && expr.ends_with('"') && expr.len() >= 2 {
        return Some(Value::String(expr.trim_matches('"').to_string()));
    }

    // Bare path
    ctx.lookup(expr.trim())
}

fn split_top_level(expr: &str, separator: char) -> Option<(&str, &str)> {
    let mut depth: i32 = 0;
    for (idx, ch) in expr.char_indices() {
        match ch {
            '(' | '{' => depth += 1,
            ')' | '}' => depth = depth.saturating_sub(1),
            _ => {}
        }
        if depth == 0 && ch == separator {
            return Some((&expr[..idx], &expr[idx + 1..]));
        }
    }
    None
}

fn split_equality(expr: &str) -> Option<(&str, &str)> {
    let mut depth: i32 = 0;
    let bytes = expr.as_bytes();
    let mut idx = 0;
    while idx + 1 < bytes.len() {
        match bytes[idx] as char {
            '(' | '{' => depth += 1,
            ')' | '}' => depth = depth.saturating_sub(1),
            '=' if depth == 0 && bytes[idx + 1] == b'=' => {
                return Some((&expr[..idx], &expr[idx + 2..]));
            }
            _ => {}
        }
        idx += 1;
    }
    None
}

pub(crate) fn truthy(v: &Value) -> bool {
    match v {
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(false),
        Value::String(s) => !s.is_empty(),
        Value::Null => false,
        Value::Array(arr) => !arr.is_empty(),
        Value::Object(obj) => !obj.is_empty(),
    }
}

pub(crate) fn equals(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.as_f64() == y.as_f64(),
        (Value::String(x), Value::String(y)) => x == y,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Null, Value::Null) => true,
        _ => a == b,
    }
}

pub(crate) fn stringify_value(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
        other => other.to_string(),
    }
}
