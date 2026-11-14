// AI-assisted (OpenBot agent)
//! Vetores de objetivo, fronteira de Pareto (dominância estrita) e alternativas distintas.
//!
//! Porte fiel de `objective_vectors`, `pareto_front`, `hamming` e `select_alternatives` de
//! `src/fieldshift/optimization.py` (commit `16cc391`). O agrupamento de vetores idênticos usa os
//! valores JÁ ARREDONDADOS a 9 casas (`np.round`), como o Python; o representante é o menor ID.

use std::collections::{BTreeMap, HashMap};

use crate::catalog::{Plan, PlanId};
use crate::compat::{fsum, py_round};
use crate::selection::{q, Tensor3, ROUND};

pub const K_ALTERNATIVES: usize = 6;
pub const MIN_HAMMING: usize = 2;

/// F(x) = (média g1, max g1, média g2, média g3) sobre S_constr, arredondado por `q`.
pub fn objective_vectors(comp: &Tensor3) -> Vec<[f64; 4]> {
    (0..comp.n)
        .map(|i| {
            let (mut g1, mut g2, mut g3) = (Vec::new(), Vec::new(), Vec::new());
            for j in 0..comp.s {
                let g = comp.at(i, j);
                g1.push(g[0]);
                g2.push(g[1]);
                g3.push(g[2]);
            }
            let s = comp.s as f64;
            let max_g1 = g1.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            [
                q(fsum(&g1) / s),
                q(max_g1),
                q(fsum(&g2) / s),
                q(fsum(&g3) / s),
            ]
        })
        .collect()
}

fn vec_key(v: &[f64; 4]) -> [u64; 4] {
    // -0.0 == 0.0 no Python (mesmo hash): normaliza o zero.
    v.map(|x| {
        if x == 0.0 {
            0.0f64.to_bits()
        } else {
            x.to_bits()
        }
    })
}

/// Índices não dominados (dominância estrita; um representante por vetor igual: menor ID) e, para
/// cada representante, os índices com vetor idêntico.
pub fn pareto_front(f: &[[f64; 4]], ids: &[PlanId]) -> (Vec<usize>, BTreeMap<usize, Vec<usize>>) {
    let mut order: Vec<[u64; 4]> = Vec::new();
    let mut groups: HashMap<[u64; 4], Vec<usize>> = HashMap::new();
    for (i, v) in f.iter().enumerate() {
        let k = vec_key(v);
        if !groups.contains_key(&k) {
            order.push(k);
        }
        groups.entry(k).or_default().push(i);
    }
    let reps: Vec<usize> = order
        .iter()
        .map(|k| {
            groups[k]
                .iter()
                .copied()
                .min_by(|&a, &b| ids[a].cmp(&ids[b]))
                .unwrap_or(0)
        })
        .collect();
    let mut front: Vec<usize> = Vec::new();
    for &a in &reps {
        let dominated = reps.iter().any(|&b| {
            let le = (0..4).all(|k| f[b][k] <= f[a][k]);
            let lt = (0..4).any(|k| f[b][k] < f[a][k]);
            le && lt
        });
        if !dominated {
            front.push(a);
        }
    }
    front.sort_by(|&a, &b| ids[a].cmp(&ids[b]));
    let mut eq = BTreeMap::new();
    for &r in &front {
        let mut e: Vec<usize> = groups[&vec_key(&f[r])]
            .iter()
            .copied()
            .filter(|&i| i != r)
            .collect();
        e.sort_by(|&a, &b| ids[a].cmp(&ids[b]));
        eq.insert(r, e);
    }
    (front, eq)
}

pub fn hamming(a: &[usize; 4], b: &[usize; 4]) -> usize {
    a.iter().zip(b).filter(|(x, y)| x != y).count()
}

#[derive(Debug, Clone, PartialEq)]
pub struct Alternative {
    pub plan_index: usize,
    pub plan_id: PlanId,
    pub labels: Vec<String>,
    /// `"policy"` | `"extreme_g1"` | `"extreme_g2"` | `"extreme_g3"` | `"spread"`.
    pub reason: String,
    pub on_front: bool,
    pub hamming_relaxed: bool,
    /// Só presente nas alternativas vindas de políticas.
    pub policy_plan_ids: Option<BTreeMap<String, PlanId>>,
    pub min_hamming_to_others: usize,
}

fn norm4(a: &[f64; 4], b: &[f64; 4]) -> f64 {
    let d: [f64; 4] = [a[0] - b[0], a[1] - b[1], a[2] - b[2], a[3] - b[3]];
    (((d[0] * d[0] + d[1] * d[1]) + d[2] * d[2]) + d[3] * d[3]).sqrt()
}

fn dot4(f: &[f64; 4], w: &[f64; 4]) -> f64 {
    ((f[0] * w[0] + f[1] * w[1]) + f[2] * w[2]) + f[3] * w[3]
}

/// Alternativas distintas: assinatura (cultura de cada janela) sem repetição.
///
/// 1) planos das políticas (P0, P1, P1b, P2), um por assinatura (rótulos agrupados);
/// 2) extremos da fronteira (menor média de g1, g2, g3) com distância de Hamming >= delta;
/// 3) preenchimento por maior distância mínima (Hamming, depois euclidiana em F) com Hamming >= delta,
///    relaxando para 1 (com aviso) se faltar candidato. Nunca completa com duplicatas.
pub fn select_alternatives(
    plans: &[Plan],
    f: &[[f64; 4]],
    front: &[usize],
    policy_picks: &[(String, usize)],
    w: [f64; 3],
    k: usize,
    delta: usize,
) -> Vec<Alternative> {
    let ids: Vec<PlanId> = plans.iter().map(|p| p.id).collect();
    let front_set: Vec<[u64; 4]> = front.iter().map(|&i| vec_key(&f[i])).collect();
    let mut alts: Vec<Alternative> = Vec::new();
    let mut sigs: Vec<[usize; 4]> = Vec::new();

    let add = |alts: &mut Vec<Alternative>,
               sigs: &mut Vec<[usize; 4]>,
               i: usize,
               labels: Vec<String>,
               reason: &str,
               relaxed: bool| {
        sigs.push(plans[i].signature());
        alts.push(Alternative {
            plan_index: i,
            plan_id: plans[i].id,
            labels,
            reason: reason.to_string(),
            on_front: front_set.contains(&vec_key(&f[i])),
            hamming_relaxed: relaxed,
            policy_plan_ids: None,
            min_hamming_to_others: 0,
        });
    };

    for (label, i) in policy_picks {
        let sig = plans[*i].signature();
        if let Some(pos) = sigs.iter().position(|s| *s == sig) {
            alts[pos].labels.push(label.clone());
        } else {
            add(
                &mut alts,
                &mut sigs,
                *i,
                vec![label.clone()],
                "policy",
                false,
            );
        }
        let pos = sigs.iter().position(|s| *s == sig).unwrap_or(0);
        alts[pos]
            .policy_plan_ids
            .get_or_insert_with(BTreeMap::new)
            .insert(label.clone(), plans[*i].id);
    }

    // candidatos: representantes da fronteira, um por assinatura (menor w*·F, depois ID)
    let wv = [w[0], w[0], w[1], w[2]];
    let score = |i: usize| py_round(dot4(&f[i], &wv), ROUND);
    let mut best: BTreeMap<[usize; 4], usize> = BTreeMap::new();
    for &i in front {
        let s = plans[i].signature();
        if sigs.contains(&s) {
            continue;
        }
        match best.get(&s) {
            None => {
                best.insert(s, i);
            }
            Some(&b) => {
                let better = match score(i).partial_cmp(&score(b)) {
                    Some(std::cmp::Ordering::Less) => true,
                    Some(std::cmp::Ordering::Equal) => ids[i] < ids[b],
                    _ => false,
                };
                if better {
                    best.insert(s, i);
                }
            }
        }
    }
    let mut cands: Vec<usize> = best.values().copied().collect();
    cands.sort_by(|&a, &b| ids[a].cmp(&ids[b]));

    let min_ham = |sigs: &[[usize; 4]], i: usize| -> usize {
        sigs.iter()
            .map(|s| hamming(&plans[i].signature(), s))
            .min()
            .unwrap_or(99)
    };

    // extremos
    for (col, name) in [(0usize, "extreme_g1"), (2, "extreme_g2"), (3, "extreme_g3")] {
        if alts.len() >= k || cands.is_empty() {
            break;
        }
        let pool: Vec<usize> = cands
            .iter()
            .copied()
            .filter(|&i| min_ham(&sigs, i) >= delta)
            .collect();
        let Some(&i) = pool.iter().min_by(|&&a, &&b| {
            f[a][col]
                .partial_cmp(&f[b][col])
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| ids[a].cmp(&ids[b]))
        }) else {
            continue;
        };
        add(&mut alts, &mut sigs, i, vec![], name, false);
        cands.retain(|&c| c != i);
    }

    for (thr, relaxed) in [(delta, false), (1usize, true)] {
        while alts.len() < k {
            let pool: Vec<usize> = cands
                .iter()
                .copied()
                .filter(|&i| min_ham(&sigs, i) >= thr)
                .collect();
            if pool.is_empty() {
                break;
            }
            let dist = |i: usize| -> f64 {
                alts.iter()
                    .map(|a| norm4(&f[i], &f[a.plan_index]))
                    .fold(f64::INFINITY, f64::min)
            };
            let key = |i: usize| (min_ham(&sigs, i), py_round(dist(i), ROUND));
            let mut best_i = pool[0];
            for &j in &pool[1..] {
                let (kj, kb) = (key(j), key(best_i));
                let ord =
                    kj.0.cmp(&kb.0)
                        .then(kj.1.partial_cmp(&kb.1).unwrap_or(std::cmp::Ordering::Equal))
                        // empate: maior chave = menor ID (chave é o ID negado)
                        .then_with(|| ids[best_i].cmp(&ids[j]));
                if ord == std::cmp::Ordering::Greater {
                    best_i = j;
                }
            }
            add(&mut alts, &mut sigs, best_i, vec![], "spread", relaxed);
            cands.retain(|&c| c != best_i);
        }
    }

    let all_sigs = sigs.clone();
    for a in &mut alts {
        let sa = plans[a.plan_index].signature();
        a.min_hamming_to_others = all_sigs
            .iter()
            .filter(|s| **s != sa)
            .map(|s| hamming(&sa, s))
            .min()
            .unwrap_or(0);
    }
    alts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Occupation;

    fn id(i: i32) -> PlanId {
        [i, 0, 0, 0, 0, 0]
    }

    #[test]
    fn strict_dominance_example() {
        let f = [
            [1.0, 1.0, 0.0, 0.0],
            [1.0, 2.0, 0.0, 0.0],
            [2.0, 1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0, 0.0],
            [0.0, 3.0, 0.0, 0.0],
            [3.0, 0.0, 0.0, 0.0],
        ];
        let ids: Vec<PlanId> = (0..6).map(id).collect();
        let (front, eq) = pareto_front(&f, &ids);
        assert_eq!(front, vec![0, 4, 5]);
        assert_eq!(eq[&0], vec![3]);
    }

    #[test]
    fn matches_bruteforce_with_duplicates() {
        let mut x = 99u64;
        let mut next = || {
            x = x
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (x >> 33) as usize
        };
        for _ in 0..5 {
            let f: Vec<[f64; 4]> = (0..60)
                .map(|_| [(); 4].map(|()| (next() % 5) as f64 / 4.0))
                .collect();
            let ids: Vec<PlanId> = (0..60).map(|i| id(i * 37 % 60)).collect();
            let (front, _) = pareto_front(&f, &ids);
            let naive: Vec<usize> = (0..60)
                .filter(|&i| {
                    !(0..60).any(|j| {
                        (0..4).all(|k| f[j][k] <= f[i][k]) && (0..4).any(|k| f[j][k] < f[i][k])
                    })
                })
                .collect();
            let fv: std::collections::BTreeSet<[u64; 4]> =
                front.iter().map(|&i| vec_key(&f[i])).collect();
            let nv: std::collections::BTreeSet<[u64; 4]> =
                naive.iter().map(|&i| vec_key(&f[i])).collect();
            assert_eq!(fv, nv);
            for &r in &front {
                let same: Vec<usize> = (0..60)
                    .filter(|&i| vec_key(&f[i]) == vec_key(&f[r]))
                    .collect();
                assert_eq!(r, *same.iter().min_by_key(|&&i| ids[i]).unwrap());
            }
        }
    }

    #[test]
    fn alternatives_never_pad_with_duplicates() {
        let mk = |sig: [usize; 4], i: i32| {
            let occ = |w: usize, c: usize| Occupation {
                window: w,
                crop: c,
                sow_t: None,
                emerge_t: None,
                harvest_t: None,
                dec: None,
            };
            Plan {
                id: id(i),
                occupations: [
                    occ(0, sig[0]),
                    occ(1, sig[1]),
                    occ(2, sig[2]),
                    occ(3, sig[3]),
                ],
            }
        };
        let plans = vec![
            mk([0, 2, 0, 2], 0),
            mk([0, 2, 0, 2], 1),
            mk([1, 2, 1, 2], 2),
        ];
        let f = [[0.1; 4], [0.1; 4], [0.2; 4]];
        let ids: Vec<PlanId> = plans.iter().map(|p| p.id).collect();
        let (front, _) = pareto_front(&f, &ids);
        let alts = select_alternatives(
            &plans,
            &f,
            &front,
            &[("P1".into(), 0)],
            [0.5, 0.25, 0.25],
            6,
            2,
        );
        assert_eq!(alts.len(), 1);
        assert_eq!(alts[0].labels, vec!["P1".to_string()]);
    }
}
