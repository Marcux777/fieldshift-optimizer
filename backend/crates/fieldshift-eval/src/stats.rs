// AI-assisted (OpenBot agent)
//! Estatística do protocolo v1.1: bootstrap em blocos, Wilcoxon exato e classificação.
//! Porte de `evaluation.py` (block_bootstrap_ci, _avg_ranks, wilcoxon_exact, classify).

use crate::npcompat::{np_median, np_quantile, np_round, np_sum, Pcg64};

pub const OUTCOME_SUSTAINED: &str = "SUSTENTADO";
pub const OUTCOME_NEGATIVE: &str = "NEGATIVO";
pub const OUTCOME_INCONCLUSIVE: &str = "INCONCLUSIVO";
pub const OUTCOME_INFEASIBLE: &str = "INVIAVEL";
pub const OUTCOME_INSUFFICIENT: &str = "DADOS_INSUFICIENTES";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BootStat {
    Median,
    Mean,
}

/// IC 95% percentil por bootstrap em blocos não sobrepostos (`np.random.default_rng(seed)`).
pub fn block_bootstrap_ci(
    d: &[f64],
    block_size: usize,
    n_boot: usize,
    seed: u64,
    stat: BootStat,
) -> (f64, f64) {
    let n = d.len();
    if n == 0 {
        return (f64::NAN, f64::NAN);
    }
    let nb = n.div_ceil(block_size);
    let mut g = Pcg64::from_seed(seed);
    let idx = g.integers_below(nb as u32, n_boot * nb);
    block_bootstrap_ci_from_indices(d, block_size, &idx, n_boot, stat)
}

/// Mesmo cálculo com a matriz de índices dada (linha a linha), para isolar a estatística do RNG.
pub fn block_bootstrap_ci_from_indices(
    d: &[f64],
    block_size: usize,
    idx: &[u32],
    n_boot: usize,
    stat: BootStat,
) -> (f64, f64) {
    let n = d.len();
    let nb = n.div_ceil(block_size);
    let mut mat = vec![f64::NAN; nb * block_size];
    mat[..n].copy_from_slice(d);
    let mut vals = Vec::with_capacity(n_boot);
    let mut sample = Vec::with_capacity(nb * block_size);
    for r in 0..n_boot {
        sample.clear();
        for c in 0..nb {
            let b = idx[r * nb + c] as usize;
            sample.extend_from_slice(&mat[b * block_size..(b + 1) * block_size]);
        }
        let finite: Vec<f64> = sample.iter().copied().filter(|v| !v.is_nan()).collect();
        let v = match stat {
            BootStat::Median => np_median(&finite),
            // np.nanmean: NaN -> 0, soma em pares da linha inteira, divide pela contagem
            BootStat::Mean => {
                let zeroed: Vec<f64> = sample
                    .iter()
                    .map(|v| if v.is_nan() { 0.0 } else { *v })
                    .collect();
                np_sum(&zeroed) / finite.len() as f64
            }
        };
        vals.push(v);
    }
    (np_quantile(&vals, 0.025), np_quantile(&vals, 0.975))
}

fn avg_ranks(x: &[f64]) -> Vec<f64> {
    let mut order: Vec<usize> = (0..x.len()).collect();
    order.sort_by(|&a, &b| x[a].total_cmp(&x[b])); // estável, como mergesort
    let mut ranks = vec![0.0; x.len()];
    let mut i = 0;
    while i < x.len() {
        let mut j = i;
        while j + 1 < x.len() && x[order[j + 1]] == x[order[i]] {
            j += 1;
        }
        let r = (i + j) as f64 / 2.0 + 1.0;
        for &k in &order[i..=j] {
            ranks[k] = r;
        }
        i = j + 1;
    }
    ranks
}

#[derive(Clone, Debug, PartialEq)]
pub struct Wilcoxon {
    pub n_nonzero: usize,
    pub w_plus: f64,
    pub p_two_sided: f64,
}

/// Wilcoxon pareado exato bilateral (sinais por enumeração; zeros descartados).
pub fn wilcoxon_exact(d: &[f64], decimals: i32) -> Wilcoxon {
    let nz: Vec<f64> = d
        .iter()
        .map(|v| np_round(*v, decimals))
        .filter(|v| *v != 0.0)
        .collect();
    let n = nz.len();
    if n == 0 {
        return Wilcoxon {
            n_nonzero: 0,
            w_plus: 0.0,
            p_two_sided: 1.0,
        };
    }
    let abs: Vec<f64> = nz.iter().map(|v| v.abs()).collect();
    let ranks = avg_ranks(&abs);
    let pos: Vec<f64> = ranks
        .iter()
        .zip(&nz)
        .filter(|(_, v)| **v > 0.0)
        .map(|(r, _)| *r)
        .collect();
    let w_obs = np_sum(&pos);
    let center = np_sum(&ranks) / 2.0;
    let dev_obs = (w_obs - center).abs();
    let total = 1usize << n;
    let mut count = 0usize;
    for mask in 0..total {
        // itertools.product((0,1), repeat=n): o primeiro elemento é o mais significativo
        let mut w = 0.0;
        for (k, r) in ranks.iter().enumerate() {
            if (mask >> (n - 1 - k)) & 1 == 1 {
                w += r;
            }
        }
        if (w - center).abs() >= dev_obs - 1e-12 {
            count += 1;
        }
    }
    Wilcoxon {
        n_nonzero: n,
        w_plus: w_obs,
        p_two_sided: count as f64 / total as f64,
    }
}

/// Regras do item 3 do protocolo (`a` = P2, `b` = comparador).
pub fn classify(
    identical: bool,
    ci: Option<(f64, f64)>,
    m1_a: f64,
    m1_b: f64,
    m0_ok: bool,
    epsilon: f64,
    rel_gain: f64,
) -> (&'static str, &'static str) {
    if identical {
        return (
            OUTCOME_INCONCLUSIVE,
            "planos idênticos: a robustez não mudou a decisão",
        );
    }
    let Some((lo, hi)) = ci.filter(|(lo, hi)| !lo.is_nan() && !hi.is_nan()) else {
        return (OUTCOME_INCONCLUSIVE, "IC indisponível");
    };
    let reduction = m1_b - m1_a;
    let rel_ok = m1_b > 0.0 && reduction >= rel_gain * m1_b;
    if hi < 0.0 && rel_ok && reduction >= epsilon && m0_ok {
        return (OUTCOME_SUSTAINED, "IC 95% < 0 e M1 cai ≥10% e ≥ε");
    }
    if lo > 0.0 {
        return (OUTCOME_NEGATIVE, "IC 95% inteiramente > 0");
    }
    if np_round(m1_a, 9) >= np_round(m1_b, 9) {
        return (OUTCOME_NEGATIVE, "M1(P2) ≥ M1 do comparador");
    }
    (
        OUTCOME_INCONCLUSIVE,
        "IC cruza 0 ou ganho de M1 abaixo do limiar",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wilcoxon_minimum_p_for_seven_pairs() {
        let d: Vec<f64> = (1..=7).map(f64::from).collect();
        assert_eq!(wilcoxon_exact(&d, 9).p_two_sided, 2.0 / 128.0);
        assert_eq!(wilcoxon_exact(&[0.0; 7], 9).p_two_sided, 1.0);
        assert_eq!(wilcoxon_exact(&[1.0, -1.0, 2.0, -2.0], 9).p_two_sided, 1.0);
    }

    #[test]
    fn classification_rules() {
        let c = |id, ci, a, b| classify(id, Some(ci), a, b, true, 0.02, 0.10).0;
        assert_eq!(c(true, (-1.0, -0.5), 0.1, 0.5), "INCONCLUSIVO");
        assert_eq!(c(false, (-0.2, -0.05), 0.3, 0.5), "SUSTENTADO");
        assert_eq!(c(false, (-0.2, -0.05), 0.48, 0.5), "INCONCLUSIVO");
        assert_eq!(c(false, (0.01, 0.2), 0.3, 0.5), "NEGATIVO");
        assert_eq!(c(false, (-0.1, 0.1), 0.5, 0.5), "NEGATIVO");
        assert_eq!(c(false, (-0.1, 0.1), 0.4, 0.5), "INCONCLUSIVO");
    }

    #[test]
    fn bootstrap_constant_and_deterministic() {
        let d = [-0.1; 14];
        let (lo, hi) = block_bootstrap_ci(&d, 2, 1000, 20261114, BootStat::Median);
        assert!((lo + 0.1).abs() < 1e-15 && (hi + 0.1).abs() < 1e-15);
        let d2: Vec<f64> = (0..14).map(|i| -0.05 + 0.1 * f64::from(i) / 13.0).collect();
        let a = block_bootstrap_ci(&d2, 2, 1000, 20261114, BootStat::Median);
        assert_eq!(
            a,
            block_bootstrap_ci(&d2, 2, 1000, 20261114, BootStat::Median)
        );
        assert!(a.0 < 0.0 && 0.0 < a.1);
    }
}
