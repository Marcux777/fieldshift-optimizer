//! AI-assisted pure Pareto frontier: four objectives, strict dominance,
//! identical-vector grouping and lexicographically smallest representative.
//! Matches the original Python protocol for the already-rounded objective vectors.
#![allow(dead_code)]

use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct Candidate {
    pub id: Vec<i32>,
    pub scores: [f64; 4],
}

#[derive(Debug, PartialEq)]
pub enum ParetoError {
    Empty,
    Invalid,
}

pub fn frontier(candidates: &[Candidate]) -> Result<Vec<Vec<i32>>, ParetoError> {
    if candidates.is_empty() { return Err(ParetoError::Empty); }
    if candidates.iter().any(|c| c.id.is_empty() ||
        c.scores.iter().any(|v| !v.is_finite() || *v < 0. || *v > 1.)) {
        return Err(ParetoError::Invalid);
    }
    let mut grouped: BTreeMap<[u64; 4], Vec<usize>> = BTreeMap::new();
    for (i, c) in candidates.iter().enumerate() {
        grouped.entry(c.scores.map(f64::to_bits)).or_default().push(i);
    }
    let representatives: Vec<usize> = grouped.values().map(|v| {
        *v.iter().min_by_key(|i| &candidates[**i].id).unwrap()
    }).collect();
    let mut front = Vec::new();
    for &i in &representatives {
        let a = candidates[i].scores;
        let dominated = representatives.iter().any(|&j| {
            let b = candidates[j].scores;
            b.iter().zip(a).all(|(bi, ai)| *bi <= ai)
                && b.iter().zip(a).any(|(bi, ai)| *bi < ai)
        });
        if !dominated { front.push(candidates[i].id.clone()); }
    }
    front.sort();
    Ok(front)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dominance_and_identical_vectors() {
        let plans = vec![
            Candidate { id: vec![3], scores: [0.4,0.4,0.4,0.4] },
            Candidate { id: vec![2], scores: [0.4,0.4,0.4,0.4] },
            Candidate { id: vec![5], scores: [0.6,0.6,0.6,0.6] },
            Candidate { id: vec![1], scores: [0.2,0.7,0.4,0.4] },
        ];
        assert_eq!(frontier(&plans), Ok(vec![vec![1],vec![2]]));
    }
    #[test]
    fn rejects_invalid_vectors() {
        assert_eq!(frontier(&[]), Err(ParetoError::Empty));
        assert_eq!(frontier(&[Candidate{id:vec![1],scores:[f64::NAN,0.,0.,0.]}]), Err(ParetoError::Invalid));
    }
}
