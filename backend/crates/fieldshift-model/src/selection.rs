//! AI-assisted first port of pure policy selection primitives.
//! IMPORTANT: not connected to the real agronomic catalog, scenario construction, or evaluation.
//! These tests are synthetic; scientific parity with the Python engine remains unverified.
#![allow(dead_code)]

#[derive(Debug, Clone, PartialEq)]
pub enum SelectionError { Empty, Ragged, NonFinite, InvalidWeights, InvalidPlanIds }

pub fn weight_grid() -> Vec<[f64; 3]> {
    let mut grid = Vec::with_capacity(15);
    for a in (0..=4).rev() {
        for b in (0..=4-a).rev() {
            grid.push([a as f64 / 4., b as f64 / 4., (4-a-b) as f64 / 4.]);
        }
    }
    grid
}
pub fn weighted_loss(components: &[[f64; 3]], weights: [f64; 3]) -> Result<Vec<f64>, SelectionError> {
    if !weights.iter().all(|w| w.is_finite() && *w >= 0.)
        || (weights.iter().sum::<f64>() - 1.).abs() > 1e-12 {
        return Err(SelectionError::InvalidWeights);
    }
    if components.is_empty() { return Err(SelectionError::Empty); }
    if !components.iter().flatten().all(|x| x.is_finite() && (0. ..=1.).contains(x)) {
        return Err(SelectionError::NonFinite);
    }
    Ok(components.iter().map(|g| g.iter().zip(weights.iter()).map(|(a,b)| a*b).sum()).collect())
}
/// Rows are candidate plans; columns are construction scenarios.
/// Independent oracle is computed for each scenario (column).
pub fn regrets(losses: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, SelectionError> {
    let n = losses.first().ok_or(SelectionError::Empty)?.len();
    if n == 0 { return Err(SelectionError::Empty); }
    if losses.iter().any(|r| r.len() != n) { return Err(SelectionError::Ragged); }
    if !losses.iter().flatten().all(|v| v.is_finite() && *v >= 0.) {
        return Err(SelectionError::NonFinite);
    }
    let mins: Vec<f64> = (0..n).map(|j| losses.iter().fold(f64::INFINITY, |m,r| m.min(r[j]))).collect();
    Ok(losses.iter().map(|row| row.iter().zip(&mins).map(|(v,m)| (v-m).max(0.)).collect()).collect())
}
fn sum_stable(xs: &[f64]) -> f64 {
    // Neumaier compensated sum, approximates but does not claim Python math.fsum equivalence.
    let (mut sum, mut compensation) = (0., 0.);
    for &x in xs {
        let t: f64 = sum + x;
        if sum.abs() >= x.abs() { compensation += (sum - t) + x; }
        else { compensation += (x - t) + sum; }
        sum = t;
    }
    sum + compensation
}
fn q(v: f64) -> i64 { (v * 1e9).round_ties_even() as i64 }
/// P2 = lexicographic (max regret, mean regret, stable plan ID).
pub fn select_mmr(losses: &[Vec<f64>], plan_ids: &[Vec<i32>]) -> Result<usize, SelectionError> {
    let regret = regrets(losses)?;
    if regret.len() != plan_ids.len() || plan_ids.iter().any(|id| id.is_empty()) {
        return Err(SelectionError::InvalidPlanIds);
    }
    (0..regret.len()).min_by_key(|&i| {
        (q(regret[i].iter().copied().fold(0.0_f64, f64::max)),
         q(sum_stable(&regret[i]) / regret[i].len() as f64),
         &plan_ids[i])
    }).ok_or(SelectionError::Empty)
}
/// P1 = lexicographic (loss under construction-only mean climate, mean loss, stable ID).
pub fn select_conv(losses: &[Vec<f64>], mean_climate_losses: &[f64],
    plan_ids: &[Vec<i32>]) -> Result<usize, SelectionError> {
    regrets(losses)?;
    if losses.len() != mean_climate_losses.len() || losses.len() != plan_ids.len()
        || plan_ids.iter().any(|id| id.is_empty()) {
        return Err(SelectionError::InvalidPlanIds);
    }
    if !mean_climate_losses.iter().all(|x| x.is_finite() && *x >= 0.) {
        return Err(SelectionError::NonFinite);
    }
    (0..losses.len()).min_by_key(|&i| (
        q(mean_climate_losses[i]),
        q(sum_stable(&losses[i]) / losses[i].len() as f64),
        &plan_ids[i]
    )).ok_or(SelectionError::Empty)
}
/// P1b = lexicographic (mean loss, max regret, stable plan ID).
pub fn select_exp(losses: &[Vec<f64>], plan_ids: &[Vec<i32>]) -> Result<usize, SelectionError> {
    let regret = regrets(losses)?;
    if regret.len() != plan_ids.len() || plan_ids.iter().any(|id| id.is_empty()) {
        return Err(SelectionError::InvalidPlanIds);
    }
    (0..losses.len()).min_by_key(|&i| {
        (q(sum_stable(&losses[i]) / losses[i].len() as f64),
         q(regret[i].iter().copied().fold(0.0_f64, f64::max)),
         &plan_ids[i])
    }).ok_or(SelectionError::Empty)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixed_weight_grid() {
        let grid = weight_grid();
        assert_eq!(grid.len(), 15);
        assert!(grid.contains(&[0.5,0.25,0.25]));
        assert_eq!(grid[0], [1.,0.,0.]);
    }
    #[test]
    fn no_bad_weights() {
        assert_eq!(weighted_loss(&[[0.,0.,0.]], [0.5,0.25,0.2]), Err(SelectionError::InvalidWeights));
    }
    #[test]
    fn oracle_and_mmr_use_construction_only() {
        let loss = vec![vec![0.0, 0.9],vec![0.5,0.5],vec![0.9,0.0]];
        let r = regrets(&loss).unwrap();
        assert_eq!(r[1], vec![0.5,0.5]);
        assert_eq!(select_mmr(&loss,&[vec![0],vec![1],vec![2]]),Ok(1));
    }
    #[test]
    fn deterministic_tiebreak() {
        let loss = vec![vec![0.3,0.3],vec![0.3,0.3]];
        assert_eq!(select_mmr(&loss,&[vec![3],vec![1]]), Ok(1));
        assert_eq!(select_exp(&loss,&[vec![3],vec![1]]), Ok(1));
    }
    #[test]
    fn conventional_uses_mean_climate_then_mean_loss() {
        let losses = vec![vec![0.3,0.7], vec![0.4,0.4]];
        let ids = vec![vec![1],vec![2]];
        assert_eq!(select_conv(&losses, &[0.2,0.3], &ids), Ok(0));
        assert_eq!(select_conv(&losses, &[0.3,0.3], &ids), Ok(1));
    }
    #[test]
    fn rejects_invalid_matrices() {
        assert_eq!(regrets(&[]), Err(SelectionError::Empty));
        assert_eq!(regrets(&[vec![0.1],vec![0.1,0.2]]), Err(SelectionError::Ragged));
        assert_eq!(regrets(&[vec![f64::NAN]]), Err(SelectionError::NonFinite));
    }
}
