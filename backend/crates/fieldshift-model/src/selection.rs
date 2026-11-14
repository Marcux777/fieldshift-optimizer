// AI-assisted (OpenBot agent)
//! Perdas, regret e seleção das políticas P1 (clima médio), P1b (esperança) e P2 (minimax regret).
//!
//! Porte fiel de `loss_matrix`, `regret_matrix`, `select_*` e `q` de `src/fieldshift/optimization.py`
//! (commit `16cc391`). Convenções numéricas:
//!
//! * `L = w1 g1 + w2 g2 + w3 g3`, sempre nessa ordem de soma (sem FMA);
//! * médias por plano com `fsum` (soma exata), dividida pelo nº de cenários;
//! * comparações por `q(v) = np.round(v, 9)`; desempate lexicográfico e, por fim, o menor ID.

use crate::catalog::PlanId;
use crate::compat::{fsum, np_round};

pub const ROUND: i32 = 9;

/// Perdas ou regrets: linhas = planos, colunas = cenários (ordem cronológica).
#[derive(Debug, Clone, PartialEq)]
pub struct Matrix {
    pub rows: usize,
    pub cols: usize,
    pub data: Vec<f64>,
}

impl Matrix {
    pub fn zeros(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            data: vec![0.0; rows * cols],
        }
    }
    pub fn at(&self, i: usize, j: usize) -> f64 {
        self.data[i * self.cols + j]
    }
    pub fn row(&self, i: usize) -> &[f64] {
        &self.data[i * self.cols..(i + 1) * self.cols]
    }
}

/// Componentes (g1, g2, g3) por plano × cenário: `data[i * s + j]`.
#[derive(Debug, Clone, PartialEq)]
pub struct Tensor3 {
    pub n: usize,
    pub s: usize,
    pub data: Vec<[f64; 3]>,
}

impl Tensor3 {
    pub fn zeros(n: usize, s: usize) -> Self {
        Self {
            n,
            s,
            data: vec![[0.0; 3]; n * s],
        }
    }
    pub fn at(&self, i: usize, j: usize) -> [f64; 3] {
        self.data[i * self.s + j]
    }
    pub fn set(&mut self, i: usize, j: usize, v: [f64; 3]) {
        self.data[i * self.s + j] = v;
    }
}

/// Grade de 15 pesos (exposição, cobertura, diversidade): triplas (a,b,c)/4 com a+b+c = 4,
/// na ordem decrescente de exposição e depois de cobertura.
pub fn weight_grid() -> Vec<[f64; 3]> {
    let mut out = Vec::with_capacity(15);
    for a in (0..=4).rev() {
        for b in (0..=4 - a).rev() {
            out.push([
                f64::from(a) / 4.0,
                f64::from(b) / 4.0,
                f64::from(4 - a - b) / 4.0,
            ]);
        }
    }
    out
}

pub const DEFAULT_WEIGHTS: [f64; 3] = [0.50, 0.25, 0.25];

/// `q(v) = np.round(v, 9)`.
pub fn q(v: f64) -> f64 {
    np_round(v, ROUND)
}

/// `L = w1 g1 + w2 g2 + w3 g3`, sempre na mesma ordem de soma.
pub fn loss_matrix(comp: &Tensor3, w: [f64; 3]) -> Matrix {
    let mut m = Matrix::zeros(comp.n, comp.s);
    for (dst, g) in m.data.iter_mut().zip(&comp.data) {
        *dst = w[0] * g[0] + w[1] * g[1] + w[2] * g[2];
    }
    m
}

/// `L` de um vetor de componentes (um por plano) para um único cenário (ex.: s_barra).
pub fn loss_vector(comp: &[[f64; 3]], w: [f64; 3]) -> Vec<f64> {
    comp.iter()
        .map(|g| w[0] * g[0] + w[1] * g[1] + w[2] * g[2])
        .collect()
}

/// `R[i, s] = L[i, s] - min_i L[i, s]` (>= 0; zero no oráculo de cada cenário).
pub fn regret_matrix(l: &Matrix) -> Matrix {
    let mut mins = vec![f64::INFINITY; l.cols];
    for i in 0..l.rows {
        for (j, m) in mins.iter_mut().enumerate() {
            *m = m.min(l.at(i, j));
        }
    }
    let mut r = Matrix::zeros(l.rows, l.cols);
    for i in 0..l.rows {
        for (j, m) in mins.iter().enumerate() {
            r.data[i * l.cols + j] = l.at(i, j) - m;
        }
    }
    r
}

/// Média por plano com `fsum` (independe da ordem dos cenários).
pub fn mean_over_scenarios(m: &Matrix) -> Vec<f64> {
    (0..m.rows)
        .map(|i| fsum(m.row(i)) / m.cols as f64)
        .collect()
}

/// Máximo por plano.
pub fn row_max(m: &Matrix) -> Vec<f64> {
    (0..m.rows)
        .map(|i| m.row(i).iter().copied().fold(f64::NEG_INFINITY, f64::max))
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    pub index: usize,
    /// Nº de planos ainda empatados após cada critério.
    pub tied_after_level: Vec<usize>,
    /// `true` se foi preciso o ID para desempatar.
    pub id_tiebreak: bool,
}

/// Mínimo lexicográfico por critérios numéricos (já arredondados por `q`) e, por fim, o menor ID.
pub fn lexicographic_min(keys: &[Vec<f64>], ids: &[PlanId]) -> Selection {
    let mut cand: Vec<usize> = (0..ids.len()).collect();
    let mut tied = Vec::new();
    for k in keys {
        let min = cand.iter().map(|&i| k[i]).fold(f64::INFINITY, f64::min);
        cand.retain(|&i| k[i] == min);
        tied.push(cand.len());
        if cand.len() == 1 {
            break;
        }
    }
    let pick = cand
        .iter()
        .copied()
        .min_by(|&a, &b| ids[a].cmp(&ids[b]))
        .unwrap_or(0);
    Selection {
        index: pick,
        tied_after_level: tied,
        id_tiebreak: cand.len() > 1,
    }
}

fn qv(v: Vec<f64>) -> Vec<f64> {
    v.into_iter().map(q).collect()
}

/// P2 (minimax regret): max R, média R, ID.
pub fn select_mmr(l: &Matrix, ids: &[PlanId]) -> Selection {
    let r = regret_matrix(l);
    lexicographic_min(&[qv(row_max(&r)), qv(mean_over_scenarios(&r))], ids)
}

/// P1 (clima médio): L(x, s_barra), média L, ID.
pub fn select_conv(l: &Matrix, lbar: &[f64], ids: &[PlanId]) -> Selection {
    lexicographic_min(&[qv(lbar.to_vec()), qv(mean_over_scenarios(l))], ids)
}

/// P1b (esperança): média L, max R, ID.
pub fn select_exp(l: &Matrix, ids: &[PlanId]) -> Selection {
    let r = regret_matrix(l);
    lexicographic_min(&[qv(mean_over_scenarios(l)), qv(row_max(&r))], ids)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mat(rows: &[&[f64]]) -> Matrix {
        let cols = rows[0].len();
        Matrix {
            rows: rows.len(),
            cols,
            data: rows.iter().flat_map(|r| r.iter().copied()).collect(),
        }
    }

    fn ids(v: &[[i32; 2]]) -> Vec<PlanId> {
        v.iter().map(|x| [x[0], x[1], 0, 0, 0, 0]).collect()
    }

    #[test]
    fn grid_is_15_unique_quarter_triples() {
        let g = weight_grid();
        assert_eq!(g.len(), 15);
        assert_eq!(g[0], [1.0, 0.0, 0.0]);
        assert_eq!(g[4], DEFAULT_WEIGHTS);
        assert_eq!(g[14], [0.0, 0.0, 1.0]);
        for w in &g {
            assert_eq!(w.iter().sum::<f64>(), 1.0);
            assert!(w.iter().all(|x| (x * 4.0).fract() == 0.0));
        }
        for i in 0..g.len() {
            for j in i + 1..g.len() {
                assert_ne!(g[i], g[j]);
            }
        }
    }

    // Instância de brinquedo 3 planos × 3 cenários, calculada à mão (mesma do teste Python).
    #[test]
    fn toy_3x3_by_hand() {
        let l = mat(&[&[1.0, 3.0, 2.0], &[2.0, 2.0, 2.0], &[3.0, 1.0, 4.0]]);
        let r = regret_matrix(&l);
        assert_eq!(r.data, vec![0.0, 2.0, 0.0, 1.0, 1.0, 0.0, 2.0, 0.0, 2.0]);
        let id = ids(&[[0, 2], [0, 1], [0, 3]]);
        assert_eq!(select_mmr(&l, &id).index, 1);
        let exp = select_exp(&l, &id);
        assert_eq!(
            (exp.index, exp.tied_after_level.clone(), exp.id_tiebreak),
            (1, vec![2, 1], false)
        );
        let lbar = [1.5, 1.5, 3.0];
        let conv = select_conv(&l, &lbar, &id);
        assert_eq!(
            (conv.index, conv.tied_after_level.clone(), conv.id_tiebreak),
            (1, vec![2, 2], true)
        );
        assert_eq!(
            select_conv(&l, &lbar, &ids(&[[0, 1], [0, 2], [0, 3]])).index,
            0
        );
    }

    #[test]
    fn mmr_second_criterion_is_mean_regret() {
        let l = mat(&[&[0.0, 2.0, 2.0], &[2.0, 2.0, 0.0], &[2.0, 0.0, 1.0]]);
        let s = select_mmr(&l, &ids(&[[0, 1], [0, 2], [0, 3]]));
        assert_eq!(
            (s.index, s.tied_after_level, s.id_tiebreak),
            (2, vec![3, 1], false)
        );
    }

    #[test]
    fn tolerance_is_nine_decimals_then_id() {
        let l = mat(&[&[0.1, 0.5], &[0.1 + 1e-12, 0.5]]);
        let s = select_mmr(&l, &ids(&[[0, 2], [0, 1]]));
        assert_eq!((s.index, s.id_tiebreak), (1, true));
        let l2 = mat(&[&[0.1, 0.5], &[0.1 + 1e-6, 0.5]]);
        assert_eq!(select_mmr(&l2, &ids(&[[0, 2], [0, 1]])).index, 0);
    }

    #[test]
    fn regret_nonnegative_and_oracle_zero() {
        let mut data = Vec::new();
        let mut x = 12345u64;
        for _ in 0..40 * 9 {
            x = x
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            data.push(((x >> 11) as f64) / ((1u64 << 53) as f64));
        }
        let l = Matrix {
            rows: 40,
            cols: 9,
            data,
        };
        let r = regret_matrix(&l);
        assert!(r.data.iter().all(|v| *v >= 0.0));
        for j in 0..9 {
            assert!((0..40).any(|i| r.at(i, j) == 0.0));
        }
    }

    #[test]
    fn selection_invariant_to_plan_and_scenario_order() {
        let (n, s) = (30usize, 8usize);
        let mut x = 7u64;
        let mut next = || {
            x = x
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (x >> 33) as usize
        };
        let data: Vec<f64> = (0..n * s).map(|_| (next() % 4) as f64 / 4.0).collect();
        let lbar: Vec<f64> = (0..n).map(|_| (next() % 4) as f64 / 4.0).collect();
        let id: Vec<PlanId> = (0..n)
            .map(|i| [(i / 6) as i32, (i % 6) as i32, 0, 0, 0, 0])
            .collect();
        let l = Matrix {
            rows: n,
            cols: s,
            data,
        };
        let base = (
            id[select_mmr(&l, &id).index],
            id[select_exp(&l, &id).index],
            id[select_conv(&l, &lbar, &id).index],
        );
        for _ in 0..6 {
            let mut pp: Vec<usize> = (0..n).collect();
            let mut ss: Vec<usize> = (0..s).collect();
            for i in (1..n).rev() {
                pp.swap(i, next() % (i + 1));
            }
            for i in (1..s).rev() {
                ss.swap(i, next() % (i + 1));
            }
            let mut d2 = Vec::new();
            for &p in &pp {
                for &c in &ss {
                    d2.push(l.at(p, c));
                }
            }
            let l2 = Matrix {
                rows: n,
                cols: s,
                data: d2,
            };
            let id2: Vec<PlanId> = pp.iter().map(|&p| id[p]).collect();
            let lb2: Vec<f64> = pp.iter().map(|&p| lbar[p]).collect();
            let got = (
                id2[select_mmr(&l2, &id2).index],
                id2[select_exp(&l2, &id2).index],
                id2[select_conv(&l2, &lb2, &id2).index],
            );
            assert_eq!(got, base);
        }
    }

    #[test]
    fn loss_matrix_fixed_order() {
        let t = Tensor3 {
            n: 1,
            s: 1,
            data: vec![[0.2, 0.4, 0.6]],
        };
        assert_eq!(
            loss_matrix(&t, [0.5, 0.25, 0.25]).data[0],
            0.5 * 0.2 + 0.25 * 0.4 + 0.25 * 0.6
        );
        assert_eq!(loss_matrix(&t, [0.0, 0.0, 1.0]).data[0], 0.6);
    }
}
