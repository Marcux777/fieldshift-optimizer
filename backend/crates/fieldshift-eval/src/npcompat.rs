// AI-assisted (OpenBot agent)
//! Evaluation aliases for the canonical numeric crate; no alternate RNG or fallback.
pub use fieldshift_numeric::canon::{
    py_float_repr, sha256_hex, to_canonical_utf8 as canonical_json,
};
pub use fieldshift_numeric::num::{np_mean, np_median, np_quantile, np_round, np_sum};
pub use fieldshift_numeric::rng::{seed_sequence_state_u64, Pcg64};
pub fn py_round(x: f64, decimals: usize) -> f64 {
    fieldshift_numeric::num::py_round(x, decimals as i32)
}
pub fn f64_bytes(a: &[f64]) -> Vec<u8> {
    a.iter().flat_map(|x| x.to_le_bytes()).collect()
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn float_repr_matches_python_rules() {
        let cases = [
            (0.1, "0.1"),
            (1e-05, "1e-05"),
            (0.0001, "0.0001"),
            (1e16, "1e+16"),
            (1e15, "1000000000000000.0"),
            (123.0, "123.0"),
            (-2.5, "-2.5"),
            (1.5e-7, "1.5e-07"),
            (0.020000000000000004, "0.020000000000000004"),
            (1234567890123456.7, "1234567890123456.8"),
            (12345678901234567.0, "1.2345678901234568e+16"),
        ];
        for (x, want) in cases {
            assert_eq!(py_float_repr(x), want, "{x}");
        }
    }

    #[test]
    fn pairwise_sum_order_differs_from_naive() {
        let a: Vec<f64> = (0..200).map(|i| 0.1 * f64::from(i) + 1e-3).collect();
        let naive: f64 = a.iter().sum();
        assert!((np_sum(&a) - naive).abs() < 1e-9);
    }

    #[test]
    fn quantile_linear_basic() {
        let a = [1.0, 2.0, 3.0, 4.0];
        assert_eq!(np_quantile(&a, 0.5), 2.5);
        assert_eq!(np_quantile(&a, 0.0), 1.0);
        assert_eq!(np_quantile(&a, 1.0), 4.0);
    }
}
