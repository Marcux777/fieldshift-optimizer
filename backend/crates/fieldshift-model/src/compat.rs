// AI-assisted (OpenBot agent)
//! Re-exports das primitivas canônicas de `fieldshift-numeric`.

pub use fieldshift_numeric::canon::{py_float_repr, sha256_hex, to_canonical};
pub use fieldshift_numeric::num::{fsum, median, np_round, np_sum, py_round};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fsum_is_exact() {
        assert_eq!(fsum(&[0.1; 10]), 1.0);
        assert_eq!(fsum(&[1e100, 1.0, -1e100]), 1.0);
        assert_eq!(fsum(&[]), 0.0);
    }

    #[test]
    fn py_round_matches_python_semantics() {
        assert_eq!(py_round(2.675, 2), 2.67);
        assert_eq!(py_round(0.125, 2), 0.12);
        assert_eq!(py_round(0.375, 2), 0.38);
        assert_eq!(py_round(1.0 / 3.0, 9), 0.333333333);
    }

    #[test]
    fn float_repr_like_python() {
        assert_eq!(py_float_repr(1.0), "1.0");
        assert_eq!(py_float_repr(0.1), "0.1");
        assert_eq!(py_float_repr(1e-5), "1e-05");
        assert_eq!(py_float_repr(1e16), "1e+16");
        assert_eq!(
            py_float_repr(123456789012345680.0),
            "1.2345678901234568e+17"
        );
        assert_eq!(py_float_repr(0.0001), "0.0001");
        assert_eq!(py_float_repr(1234.5), "1234.5");
        assert_eq!(py_float_repr(-2.5e-7), "-2.5e-07");
        assert_eq!(py_float_repr(100.0), "100.0");
    }

    #[test]
    fn canonical_escapes_like_python() {
        let v = serde_json::json!({"b": 1, "a": ["não", 0.5]});
        assert_eq!(to_canonical(&v), "{\"a\":[\"n\\u00e3o\",0.5],\"b\":1}");
    }
}
