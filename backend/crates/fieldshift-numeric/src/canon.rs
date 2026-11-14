// AI-assisted (OpenBot agent)
use serde_json::Value;
use sha2::{Digest, Sha256};
pub fn sha256_hex(bytes: &[u8]) -> String {
    let d = Sha256::digest(bytes);
    d.iter().map(|b| format!("{b:02x}")).collect()
}

/// `repr(float)` do Python (menor texto que volta ao mesmo float; notação científica fora de [1e-4, 1e16)).
pub fn py_float_repr(x: f64) -> String {
    if x == 0.0 {
        return if x.is_sign_negative() {
            "-0.0".into()
        } else {
            "0.0".into()
        };
    }
    let sci = format!("{x:e}"); // ex.: "1.2345e-5", "1e16"
    let (mant, exp) = sci.split_once('e').unwrap_or((&sci, "0"));
    let exp: i32 = exp.parse().unwrap_or(0);
    let neg = mant.starts_with('-');
    let digits: String = mant.chars().filter(char::is_ascii_digit).collect();
    let sign = if neg { "-" } else { "" };
    if (-4..16).contains(&exp) {
        if exp >= 0 {
            let e = exp as usize;
            let (int_part, frac_part) = if digits.len() > e + 1 {
                (digits[..=e].to_string(), digits[e + 1..].to_string())
            } else {
                (
                    format!("{digits}{}", "0".repeat(e + 1 - digits.len())),
                    "0".to_string(),
                )
            };
            format!("{sign}{int_part}.{frac_part}")
        } else {
            format!("{sign}0.{}{digits}", "0".repeat((-exp - 1) as usize))
        }
    } else {
        let head = &digits[..1];
        let tail = &digits[1..];
        let m = if tail.is_empty() {
            head.to_string()
        } else {
            format!("{head}.{tail}")
        };
        format!(
            "{sign}{m}e{}{:02}",
            if exp < 0 { "-" } else { "+" },
            exp.abs()
        )
    }
}

fn push_json_string(out: &mut String, s: &str, ascii: bool) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 || (ascii && (c as u32) > 0x7e) => {
                let mut buf = [0u16; 2];
                for u in c.encode_utf16(&mut buf) {
                    out.push_str(&format!("\\u{u:04x}"));
                }
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// `json.dumps(v, sort_keys=True, separators=(",", ":"), ensure_ascii=True, allow_nan=False)`.
pub fn to_canonical(v: &Value) -> String {
    let mut out = String::new();
    write_canonical(v, &mut out, true);
    out
}

fn write_canonical(v: &Value, out: &mut String, ascii: bool) {
    match v {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                out.push_str(&i.to_string());
            } else if let Some(u) = n.as_u64() {
                out.push_str(&u.to_string());
            } else if let Some(f) = n.as_f64() {
                out.push_str(&py_float_repr(f));
            }
        }
        Value::String(s) => push_json_string(out, s, ascii),
        Value::Array(a) => {
            out.push('[');
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_canonical(x, out, ascii);
            }
            out.push(']');
        }
        Value::Object(m) => {
            let mut keys: Vec<&String> = m.keys().collect();
            keys.sort();
            out.push('{');
            for (i, k) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                push_json_string(out, k, ascii);
                out.push(':');
                write_canonical(&m[*k], out, ascii);
            }
            out.push('}');
        }
    }
}

pub fn to_canonical_utf8(v: &Value) -> String {
    let mut out = String::new();
    write_canonical(v, &mut out, false);
    out
}
