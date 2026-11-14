// AI-assisted (OpenBot agent)
//! Avaliador retrospectivo independente: porte de `src/fieldshift/evaluation.py` (protocolo v1.1).
//!
//! Recebe planos FIXADOS por política e RECALCULA as perdas a partir do plano e do clima via
//! [`LossModel`]; nunca aceita escores do otimizador. Medianas de e3, s̄ e P25 vêm SÓ da
//! construção. As regras de seleção de referência são reimplementadas aqui para auditar o
//! otimizador. Saída: [`EvalReport`] com o mesmo dicionário e o mesmo hash canônico do Python.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde_json::{json, Map, Value};

use crate::npcompat::{
    canonical_json, f64_bytes, np_mean, np_median, np_quantile, np_round, np_sum, py_float_repr,
    py_round, sha256_hex,
};
use crate::stats::{
    block_bootstrap_ci, classify, wilcoxon_exact, BootStat, OUTCOME_INFEASIBLE,
    OUTCOME_INSUFFICIENT,
};

pub const PROTOCOL_VERSION: &str = "1.1";
pub const PROTOCOL_SHA256: &str =
    "ee8e51c5b2eecc7319818a5645ef15b84d677eabab35caad1a7e9fc41deaefb9";
pub const DAYS_PER_YEAR: usize = 365;
pub const HORIZON_DAYS: usize = 730;
pub const POLICIES: [&str; 4] = ["P0", "P1", "P1b", "P2"];
/// Validade do cenário (protocolo §1.3): as quatro variáveis completas (VSDD M-2).
pub const VALIDITY_VARS: [&str; 4] = ["PRECTOTCORR", "T2M", "T2M_MAX", "T2M_MIN"];

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct EvalError(pub String);

fn err<T>(msg: impl Into<String>) -> Result<T, EvalError> {
    Err(EvalError(msg.into()))
}

/// 15 triplas (a,b,c)/4 com a+b+c = 4; ordem decrescente de exposição, depois cobertura.
pub fn weight_grid() -> Vec<[f64; 3]> {
    let mut g = Vec::with_capacity(15);
    for a in (0..=4).rev() {
        for b in (0..=(4 - a)).rev() {
            g.push([
                f64::from(a) / 4.0,
                f64::from(b) / 4.0,
                f64::from(4 - a - b) / 4.0,
            ]);
        }
    }
    g
}

#[derive(Clone, Debug)]
pub struct EvalProtocol {
    pub construction_first_start: i32,
    pub construction_last_start: i32,
    pub test_first_start: i32,
    pub test_last_start: i32,
    pub boundary: (i32, u32, u32),
    pub epsilon: f64,
    pub rel_gain: f64,
    pub n_boot: usize,
    pub block_size: usize,
    pub seed: u64,
    pub min_construction_pairs: usize,
    pub min_test_pairs: usize,
    pub decimals: i32,
    pub default_weights: [f64; 3],
    pub dry_months: Vec<u32>,
    pub dry_quantile: f64,
    pub min_dry_pairs_inferential: usize,
}

impl Default for EvalProtocol {
    fn default() -> Self {
        EvalProtocol {
            construction_first_start: 1981,
            construction_last_start: 2008,
            test_first_start: 2010,
            test_last_start: 2023,
            boundary: (2010, 9, 1),
            epsilon: 0.02,
            rel_gain: 0.10,
            n_boot: 10_000,
            block_size: 2,
            seed: 20_261_114,
            min_construction_pairs: 20,
            min_test_pairs: 10,
            decimals: 9,
            default_weights: [0.50, 0.25, 0.25],
            dry_months: vec![10, 11, 12, 1, 2, 3],
            dry_quantile: 0.25,
            min_dry_pairs_inferential: 3,
        }
    }
}

impl EvalProtocol {
    pub fn to_value(&self) -> Value {
        let (y, m, d) = self.boundary;
        json!({
            "construction_first_start": self.construction_first_start,
            "construction_last_start": self.construction_last_start,
            "test_first_start": self.test_first_start,
            "test_last_start": self.test_last_start,
            "boundary_date": format!("{y:04}-{m:02}-{d:02}"),
            "epsilon": fl(self.epsilon),
            "rel_gain": fl(self.rel_gain),
            "n_boot": self.n_boot,
            "block_size": self.block_size,
            "seed": self.seed,
            "min_construction_pairs": self.min_construction_pairs,
            "min_test_pairs": self.min_test_pairs,
            "decimals": self.decimals,
            "default_weights": self.default_weights.iter().map(|x| fl(*x)).collect::<Vec<_>>(),
            "dry_months": self.dry_months,
            "dry_quantile": fl(self.dry_quantile),
            "min_dry_pairs_inferential": self.min_dry_pairs_inferential,
            "version": PROTOCOL_VERSION,
            "sha256": PROTOCOL_SHA256,
        })
    }

    pub fn config_hash(&self) -> String {
        sha256_json(&self.to_value())
    }
}

/// Float para o JSON do relatório como `_jsonable` do Python: `round(x, 12)`; NaN vira null.
pub fn fl(x: f64) -> Value {
    if x.is_nan() {
        return Value::Null;
    }
    serde_json::Number::from_f64(py_round(x, 12)).map_or(Value::Null, Value::Number)
}

fn flo(x: Option<f64>) -> Value {
    x.map_or(Value::Null, fl)
}

pub fn sha256_json(v: &Value) -> String {
    sha256_hex(canonical_json(v).as_bytes())
}

// ---------------------------------------------------------------------------------------------
// Calendário e clima
// ---------------------------------------------------------------------------------------------

/// Dias desde 1970-01-01 (algoritmo civil de H. Hinnant).
pub fn days_from_civil(y: i32, m: u32, d: u32) -> i64 {
    let y = i64::from(y) - i64::from(m <= 2);
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let m = i64::from(m);
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + i64::from(d) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn is_leap(y: i32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn month_len(y: i32, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ => {
            if is_leap(y) {
                29
            } else {
                28
            }
        }
    }
}

/// Datas (y, m, d) do ano agrícola y (1/set de y a 31/ago de y+1), sem 29/fev: 365 dias.
pub fn agri_year_dates(y: i32) -> Vec<(i32, u32, u32)> {
    let mut out = Vec::with_capacity(DAYS_PER_YEAR);
    for (yy, m) in [(y, 9), (y, 10), (y, 11), (y, 12)]
        .into_iter()
        .chain((1..=8).map(|m| (y + 1, m)))
    {
        for d in 1..=month_len(yy, m) {
            if m == 2 && d == 29 {
                continue;
            }
            out.push((yy, m, d));
        }
    }
    out
}

/// Clima diário (ausente = NaN; -999 também é tratado como ausente).
#[derive(Clone, Debug, Default)]
pub struct ClimateDaily {
    pub dates: Vec<(i32, u32, u32)>,
    pub prectotcorr: Vec<f64>,
    pub t2m: Vec<f64>,
    pub t2m_max: Vec<f64>,
    pub t2m_min: Vec<f64>,
}

impl ClimateDaily {
    fn columns(&self) -> [&Vec<f64>; 4] {
        [&self.prectotcorr, &self.t2m, &self.t2m_max, &self.t2m_min]
    }
}

#[derive(Clone, Debug)]
pub struct AgriYear {
    pub start: i32,
    pub dates: (String, String),
    pub precip: Vec<f64>,
    pub tmax: Vec<f64>,
    pub tmin: Vec<f64>,
    /// ausentes por variável, na ordem de [`VALIDITY_VARS`]
    pub n_missing: [usize; 4],
}

impl AgriYear {
    pub fn label(&self) -> String {
        format!("{}/{:02}", self.start, (self.start + 1).rem_euclid(100))
    }
    pub fn valid(&self) -> bool {
        self.precip.len() == DAYS_PER_YEAR && self.n_missing.iter().all(|n| *n == 0)
    }
}

#[derive(Clone, Debug)]
pub struct Scenario {
    pub start: i32,
    pub label: String,
    pub precip: Vec<f64>,
    pub tmax: Vec<f64>,
    pub tmin: Vec<f64>,
    pub first_date: String,
    pub last_date: String,
    pub years: Vec<i32>,
    pub valid: bool,
    pub synthetic: bool,
}

impl Scenario {
    /// sha256(float64 bytes P ‖ TX ‖ TN): chave de conteúdo usada pelos testes diferenciais.
    pub fn content_key(&self) -> String {
        let mut b = f64_bytes(&self.precip);
        b.extend(f64_bytes(&self.tmax));
        b.extend(f64_bytes(&self.tmin));
        sha256_hex(&b)
    }
}

fn fmt_date(t: (i32, u32, u32)) -> String {
    format!("{:04}-{:02}-{:02}", t.0, t.1, t.2)
}

fn build_agri_years(
    climate: &ClimateDaily,
    first: i32,
    last: i32,
) -> Result<BTreeMap<i32, AgriYear>, EvalError> {
    let n = climate.dates.len();
    if climate.columns().iter().any(|c| c.len() != n) {
        return err("clima com colunas de tamanhos diferentes");
    }
    let mut index: HashMap<i64, usize> = HashMap::with_capacity(n);
    for (i, (y, m, d)) in climate.dates.iter().enumerate() {
        if index.insert(days_from_civil(*y, *m, *d), i).is_some() {
            return err("clima com datas duplicadas");
        }
    }
    let clean = |v: f64| if v == -999.0 { f64::NAN } else { v };
    let mut out = BTreeMap::new();
    for y in first..=last {
        let dates = agri_year_dates(y);
        let mut cols: [Vec<f64>; 4] = Default::default();
        for date in &dates {
            let row = index.get(&days_from_civil(date.0, date.1, date.2));
            for (k, col) in climate.columns().iter().enumerate() {
                cols[k].push(row.map_or(f64::NAN, |&r| clean(col[r])));
            }
        }
        let n_missing = [0, 1, 2, 3].map(|k| cols[k].iter().filter(|v| v.is_nan()).count());
        let [p, _t2m, tx, tn] = cols;
        out.insert(
            y,
            AgriYear {
                start: y,
                dates: (fmt_date(dates[0]), fmt_date(dates[dates.len() - 1])),
                precip: p,
                tmax: tx,
                tmin: tn,
                n_missing,
            },
        );
    }
    Ok(out)
}

fn pair_scenario(a: &AgriYear, b: &AgriYear) -> Scenario {
    Scenario {
        start: a.start,
        label: format!("{}–{}", a.label(), b.label()),
        precip: [a.precip.clone(), b.precip.clone()].concat(),
        tmax: [a.tmax.clone(), b.tmax.clone()].concat(),
        tmin: [a.tmin.clone(), b.tmin.clone()].concat(),
        first_date: a.dates.0.clone(),
        last_date: b.dates.1.clone(),
        years: vec![a.start, b.start],
        valid: a.valid() && b.valid(),
        synthetic: false,
    }
}

#[derive(Clone, Debug)]
pub struct ScenarioSplit {
    pub construction: Vec<Scenario>,
    pub test: Vec<Scenario>,
    pub excluded: Vec<Value>,
    pub construction_years: Vec<AgriYear>,
}

/// Pares de construção e teste; nenhum par cruza a fronteira 2009/10 → 2010/11.
pub fn split_scenarios(
    climate: &ClimateDaily,
    p: &EvalProtocol,
) -> Result<ScenarioSplit, EvalError> {
    let years = build_agri_years(climate, p.construction_first_start, p.test_last_start + 1)?;
    let mut excluded = Vec::new();
    let mut pairs = |first: i32, last: i32, role: &str| -> Vec<Scenario> {
        let mut out = Vec::new();
        for y in first..=last {
            let (a, b) = (&years[&y], &years[&(y + 1)]);
            let sc = pair_scenario(a, b);
            if sc.valid {
                out.push(sc);
            } else {
                let mut miss = Map::new();
                for yr in [a, b] {
                    for (k, var) in VALIDITY_VARS.iter().enumerate() {
                        if yr.n_missing[k] > 0 {
                            miss.insert(format!("{}:{var}", yr.label()), json!(yr.n_missing[k]));
                        }
                    }
                }
                excluded.push(json!({"role": role, "scenario": sc.label, "missing": miss}));
            }
        }
        out
    };
    let cons = pairs(
        p.construction_first_start,
        p.construction_last_start,
        "construcao",
    );
    let test = pairs(p.test_first_start, p.test_last_start, "teste");
    let boundary = fmt_date(p.boundary);
    for sc in &cons {
        if sc.last_date >= boundary {
            return err(format!(
                "vazamento: par de construção {} cruza {boundary}",
                sc.label
            ));
        }
    }
    for sc in &test {
        if sc.first_date < boundary {
            return err(format!(
                "par de teste {} começa antes de {boundary}",
                sc.label
            ));
        }
    }
    let cons_years: BTreeSet<i32> = cons.iter().flat_map(|s| s.years.iter().copied()).collect();
    Ok(ScenarioSplit {
        construction: cons,
        test,
        excluded,
        construction_years: cons_years.iter().map(|y| years[y].clone()).collect(),
    })
}

/// s̄: média diária (soma sequencial por ano, como `nanmean(axis=0)`) dos anos válidos, nos 2 anos.
pub fn mean_climate(years: &[&AgriYear]) -> Result<Scenario, EvalError> {
    if years.is_empty() {
        return err("s̄ não calculável: nenhum ano de construção válido");
    }
    let col = |get: &dyn Fn(&AgriYear) -> &Vec<f64>, name: &str| -> Result<Vec<f64>, EvalError> {
        let mut out = Vec::with_capacity(DAYS_PER_YEAR);
        for t in 0..DAYS_PER_YEAR {
            let mut tot = 0.0;
            let mut cnt = 0usize;
            for y in years {
                let v = get(y)[t];
                if !v.is_nan() {
                    tot += v;
                    cnt += 1;
                }
            }
            if cnt == 0 {
                return err(format!("s̄ não calculável: dia sem valor em {name}"));
            }
            out.push(tot / cnt as f64);
        }
        Ok(out)
    };
    let p = col(&|y| &y.precip, "precip")?;
    let tx = col(&|y| &y.tmax, "tmax")?;
    let tn = col(&|y| &y.tmin, "tmin")?;
    Ok(Scenario {
        start: -1,
        label: "clima_medio_construcao".into(),
        precip: [p.clone(), p].concat(),
        tmax: [tx.clone(), tx].concat(),
        tmin: [tn.clone(), tn].concat(),
        first_date: String::new(),
        last_date: String::new(),
        years: years.iter().map(|y| y.start).collect(),
        valid: true,
        synthetic: true,
    })
}

fn dry_mask_agri_year(start: i32, months: &[u32]) -> Vec<bool> {
    agri_year_dates(start)
        .iter()
        .map(|(_, m, _)| months.contains(m))
        .collect()
}

fn dry_season_total(precip: &[f64], start: i32, p: &EvalProtocol) -> f64 {
    let mask = dry_mask_agri_year(start, &p.dry_months);
    let sel: Vec<f64> = precip
        .iter()
        .zip(mask)
        .filter(|(_, m)| *m)
        .map(|(v, _)| *v)
        .collect();
    np_sum(&sel)
}

/// P25 da chuva out–mar nos anos agrícolas de construção.
pub fn dry_threshold(years: &[AgriYear], p: &EvalProtocol) -> f64 {
    let totals: Vec<f64> = years
        .iter()
        .map(|y| dry_season_total(&y.precip, y.start, p))
        .collect();
    np_quantile(&totals, p.dry_quantile)
}

pub fn construction_dry_totals(years: &[AgriYear], p: &EvalProtocol) -> Vec<f64> {
    years
        .iter()
        .map(|y| dry_season_total(&y.precip, y.start, p))
        .collect()
}

pub fn scenario_dry_season_totals(sc: &Scenario, p: &EvalProtocol) -> (f64, f64) {
    let a = dry_season_total(&sc.precip[..DAYS_PER_YEAR], sc.years[0], p);
    let b = dry_season_total(&sc.precip[DAYS_PER_YEAR..], sc.years[1], p);
    (a, b)
}

/// `_array_hash`: SHA-256 dos bytes de `np.round(a, 9)` de cada vetor.
pub fn array_hash(arrays: &[&[f64]]) -> String {
    let mut b = Vec::new();
    for a in arrays {
        let r: Vec<f64> = a.iter().map(|x| np_round(*x, 9)).collect();
        b.extend(f64_bytes(&r));
    }
    sha256_hex(&b)
}

/// SHA-256 da matriz de chuva (anos × 365) dos anos agrícolas da construção: o fingerprint
/// de `ConstructionClimatology.sha256()`, que fixa as medianas de e3.
pub fn construction_precip_fingerprint(construction: &[Scenario]) -> String {
    let mut years: BTreeMap<i32, &[f64]> = BTreeMap::new();
    for sc in construction {
        for (i, y) in sc.years.iter().enumerate() {
            years.insert(*y, &sc.precip[i * DAYS_PER_YEAR..(i + 1) * DAYS_PER_YEAR]);
        }
    }
    let mut b = Vec::new();
    for p in years.values() {
        b.extend(f64_bytes(p));
    }
    sha256_hex(&b)
}

/// Hash de s̄ no formato do RunResult do otimizador (bytes crus de P, TX, TN).
pub fn sbar_sha256_like_optimizer(sbar: &Scenario) -> String {
    sbar.content_key()
}

// ---------------------------------------------------------------------------------------------
// Modelo de perda e planos
// ---------------------------------------------------------------------------------------------

/// Adaptador sobre os indicadores. `fit` só vê cenários de construção e devolve o fingerprint
/// dos parâmetros dependentes de dados.
pub trait LossModel {
    fn fit(&mut self, construction: &[Scenario]) -> Result<String, EvalError>;
    fn components(&mut self, plan: usize, scenario: &Scenario) -> Result<[f64; 3], EvalError>;
}

/// Plano visto pelo avaliador: ID estável e genes por janela (cultura, semeadura).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanInfo {
    pub id: Vec<i64>,
    pub genes: Vec<(i64, Option<i64>)>,
}

/// G[i][j] = (g1, g2, g3) do plano i no cenário j, RECALCULADO do plano e do clima.
fn component_matrix(
    n_plans: usize,
    scenarios: &[&Scenario],
    model: &mut dyn LossModel,
) -> Result<Vec<Vec<[f64; 3]>>, EvalError> {
    let mut g = vec![vec![[0.0; 3]; scenarios.len()]; n_plans];
    for (i, row) in g.iter_mut().enumerate() {
        for (j, sc) in scenarios.iter().enumerate() {
            let c = model.components(i, sc)?;
            if c.iter().any(|v| !v.is_finite()) {
                return err("componente não finito (NaN/inf) na matriz de perdas");
            }
            if c.iter().any(|v| *v < -1e-12 || *v > 1.0 + 1e-12) {
                return err("componente fora de [0,1]");
            }
            row[j] = c;
        }
    }
    Ok(g)
}

fn losses(g: &[Vec<[f64; 3]>], w: &[f64; 3]) -> Vec<Vec<f64>> {
    g.iter()
        .map(|row| {
            row.iter()
                .map(|c| c[0] * w[0] + c[1] * w[1] + c[2] * w[2])
                .collect()
        })
        .collect()
}

/// R(x,s) = L(x,s) − min_x′ L(x′,s).
pub fn regret(l: &[Vec<f64>]) -> Vec<Vec<f64>> {
    if l.is_empty() {
        return Vec::new();
    }
    let ns = l[0].len();
    let mins: Vec<f64> = (0..ns)
        .map(|j| l.iter().map(|r| r[j]).fold(f64::INFINITY, f64::min))
        .collect();
    l.iter()
        .map(|r| r.iter().zip(&mins).map(|(v, m)| v - m).collect())
        .collect()
}

fn row_max(r: &[f64]) -> f64 {
    r.iter().copied().fold(f64::NEG_INFINITY, f64::max)
}

fn argmin_lex(keys: &[(f64, f64)], ids: &[&Vec<i64>]) -> usize {
    let mut best = 0;
    for i in 1..keys.len() {
        let a = (keys[i].0, keys[i].1);
        let b = (keys[best].0, keys[best].1);
        let less = a.0 < b.0 || (a.0 == b.0 && (a.1 < b.1 || (a.1 == b.1 && ids[i] < ids[best])));
        if less {
            best = i;
        }
    }
    best
}

/// x_MMR: menor regret máximo, depois menor regret médio, depois menor ID.
pub fn select_mmr(l: &[Vec<f64>], ids: &[&Vec<i64>], d: i32) -> usize {
    let r = regret(l);
    let keys: Vec<(f64, f64)> = r
        .iter()
        .map(|row| (np_round(row_max(row), d), np_round(np_mean(row), d)))
        .collect();
    argmin_lex(&keys, ids)
}

/// x_CONV: menor L(s̄), depois menor média de L, depois menor ID.
pub fn select_conv(lbar: &[f64], l: &[Vec<f64>], ids: &[&Vec<i64>], d: i32) -> usize {
    let keys: Vec<(f64, f64)> = l
        .iter()
        .zip(lbar)
        .map(|(row, b)| (np_round(*b, d), np_round(np_mean(row), d)))
        .collect();
    argmin_lex(&keys, ids)
}

/// x_EXP: menor média de L, depois menor regret máximo, depois menor ID.
pub fn select_exp(l: &[Vec<f64>], ids: &[&Vec<i64>], d: i32) -> usize {
    let r = regret(l);
    let keys: Vec<(f64, f64)> = l
        .iter()
        .zip(&r)
        .map(|(row, rr)| (np_round(np_mean(row), d), np_round(row_max(rr), d)))
        .collect();
    argmin_lex(&keys, ids)
}

/// Seleção de referência (P1, P1b, P2) a partir das regras do protocolo.
pub fn reference_selection(
    l_cons: &[Vec<f64>],
    l_bar: &[f64],
    ids: &[&Vec<i64>],
    d: i32,
) -> BTreeMap<&'static str, usize> {
    BTreeMap::from([
        ("P1", select_conv(l_bar, l_cons, ids, d)),
        ("P1b", select_exp(l_cons, ids, d)),
        ("P2", select_mmr(l_cons, ids, d)),
    ])
}

// ---------------------------------------------------------------------------------------------
// Relatório
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
pub struct PolicyMetrics {
    pub policy: String,
    pub plan_id: Option<Vec<i64>>,
    pub feasible: bool,
    pub m1_max_regret: Option<f64>,
    pub m2_mean_regret: Option<f64>,
    pub m2_median_regret: Option<f64>,
    pub m3_dry_mean_loss: Option<f64>,
    pub m3_dry_mean_regret: Option<f64>,
    pub m4a_frac_regret_le_eps: Option<f64>,
    pub m4b_iqr_loss: Option<f64>,
    pub m4c_loo_mean_hamming: Option<f64>,
    pub mean_loss_test: Option<f64>,
    pub loss_mean_climate: Option<f64>,
    pub per_scenario_regret: Vec<f64>,
    pub per_scenario_loss: Vec<f64>,
}

impl PolicyMetrics {
    fn to_value(&self) -> Value {
        json!({
            "policy": self.policy,
            "plan_id": self.plan_id,
            "feasible": self.feasible,
            "M1_max_regret": flo(self.m1_max_regret),
            "M2_mean_regret": flo(self.m2_mean_regret),
            "M2_median_regret": flo(self.m2_median_regret),
            "M3_dry_mean_loss": flo(self.m3_dry_mean_loss),
            "M3_dry_mean_regret": flo(self.m3_dry_mean_regret),
            "M4a_frac_regret_le_eps": flo(self.m4a_frac_regret_le_eps),
            "M4b_iqr_loss": flo(self.m4b_iqr_loss),
            "M4c_loo_mean_hamming": flo(self.m4c_loo_mean_hamming),
            "mean_loss_test": flo(self.mean_loss_test),
            "loss_mean_climate": flo(self.loss_mean_climate),
            "per_scenario_regret": self.per_scenario_regret.iter().map(|x| fl(*x)).collect::<Vec<_>>(),
            "per_scenario_loss": self.per_scenario_loss.iter().map(|x| fl(*x)).collect::<Vec<_>>(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct Comparison {
    pub a: String,
    pub b: String,
    pub identical_plans: bool,
    pub n: usize,
    pub wins: usize,
    pub ties: usize,
    pub losses: usize,
    pub median_d: f64,
    pub ci95_median_d: (f64, f64),
    pub mean_d: f64,
    pub ci95_mean_d: (f64, f64),
    pub wilcoxon: crate::stats::Wilcoxon,
    pub m1_a: f64,
    pub m1_b: f64,
    pub classification: &'static str,
    pub reason: &'static str,
}

impl Comparison {
    fn to_value(&self) -> Value {
        json!({
            "a": self.a, "b": self.b,
            "identical_plans": self.identical_plans,
            "n": self.n, "wins": self.wins, "ties": self.ties, "losses": self.losses,
            "median_D": fl(self.median_d),
            "ci95_median_D": [fl(self.ci95_median_d.0), fl(self.ci95_median_d.1)],
            "mean_D": fl(self.mean_d),
            "ci95_mean_D": [fl(self.ci95_mean_d.0), fl(self.ci95_mean_d.1)],
            "wilcoxon_nonoverlap": {
                "n_nonzero": self.wilcoxon.n_nonzero,
                "W_plus": fl(self.wilcoxon.w_plus),
                "p_two_sided": fl(self.wilcoxon.p_two_sided),
            },
            "m1_a": fl(self.m1_a), "m1_b": fl(self.m1_b),
            "classification": self.classification,
            "reason": self.reason,
        })
    }
}

pub const STANDARD_LIMITATIONS: [&str; 6] = [
    "Janelas ZARC calibradas com anos que se sobrepõem ao teste: restrição fixa e igual para todas as políticas (vazamento residual declarado, não removível).",
    "PRECTOTCORR pode ter correção de viés baseada em climatologia longa (vazamento residual declarado, não verificado).",
    "Uma célula NASA POWER (-18,0/-50,625); célula vizinha não avaliada nesta execução.",
    "Indicadores de exposição e de calendário; não medem produtividade, lucro, água nem solo.",
    "M4c (leave-one-pair-out) mantém fixas as medianas de e3 da construção completa (simplificação declarada).",
    "Bootstrap com 7 blocos e Wilcoxon com 7 pares: poder estatístico baixo.",
];

#[derive(Clone, Debug)]
pub struct EvalReport {
    pub outcome: String,
    pub headline: String,
    pub protocol: Value,
    pub protocol_config_hash: String,
    pub weights: [f64; 3],
    pub n_plans: usize,
    pub construction: Vec<String>,
    pub test: Vec<String>,
    pub excluded_scenarios: Vec<Value>,
    pub params_hash: Option<String>,
    pub mean_climate_hash: Option<String>,
    pub dry_threshold_mm: Option<f64>,
    pub dry_test_scenarios: Vec<String>,
    pub m0_feasibility: Option<f64>,
    pub p0_status: String,
    pub policies: Vec<PolicyMetrics>,
    pub m5_price_of_robustness: Option<f64>,
    pub comparisons: Vec<Comparison>,
    /// auditoria na ordem de inserção do Python (relevante só para o markdown)
    pub audit: Vec<(String, Value)>,
    pub sensitivity: Vec<Value>,
    pub limitations: Vec<String>,
    pub diagnostics: Vec<(String, Value)>,
}

impl EvalReport {
    pub fn audit_set(&mut self, key: &str, v: Value) {
        if let Some(e) = self.audit.iter_mut().find(|(k, _)| k == key) {
            e.1 = v;
        } else {
            self.audit.push((key.to_string(), v));
        }
    }

    pub fn audit_get(&self, key: &str) -> Option<&Value> {
        self.audit.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    pub fn to_value(&self) -> Value {
        let policies: Map<String, Value> = self
            .policies
            .iter()
            .map(|p| (p.policy.clone(), p.to_value()))
            .collect();
        let audit: Map<String, Value> = self.audit.iter().cloned().collect();
        let diagnostics: Map<String, Value> = self.diagnostics.iter().cloned().collect();
        json!({
            "outcome": self.outcome,
            "headline": self.headline,
            "protocol": self.protocol,
            "protocol_config_hash": self.protocol_config_hash,
            "weights": self.weights.iter().map(|x| fl(*x)).collect::<Vec<_>>(),
            "n_plans": self.n_plans,
            "construction": self.construction,
            "test": self.test,
            "excluded_scenarios": self.excluded_scenarios,
            "params_hash": self.params_hash,
            "mean_climate_hash": self.mean_climate_hash,
            "dry_threshold_mm": flo(self.dry_threshold_mm),
            "dry_test_scenarios": self.dry_test_scenarios,
            "M0_feasibility": flo(self.m0_feasibility),
            "p0_status": self.p0_status,
            "policies": policies,
            "M5_price_of_robustness": flo(self.m5_price_of_robustness),
            "comparisons": self.comparisons.iter().map(Comparison::to_value).collect::<Vec<_>>(),
            "audit": audit,
            "sensitivity": self.sensitivity,
            "limitations": self.limitations,
            "diagnostics": diagnostics,
        })
    }

    /// SHA-256 do JSON canônico (mesmo de `EvalReport.result_hash()` do Python).
    pub fn result_hash(&self) -> String {
        sha256_json(&self.to_value())
    }

    pub fn to_markdown(&self) -> String {
        render_markdown(self)
    }
}

/// Entradas da avaliação: X completo, planos escolhidos por política e verificador opcional.
pub struct EvalInput<'a> {
    pub plans: &'a [PlanInfo],
    pub policy_ids: BTreeMap<String, Option<Vec<i64>>>,
    pub p0_status: Option<String>,
    pub climate: &'a ClimateDaily,
    pub weights: Option<[f64; 3]>,
    pub check_plan: Option<&'a dyn Fn(usize) -> Vec<String>>,
    pub run_grid: bool,
    pub optimizer_params_hash: Option<String>,
}

fn select_cols<T: Clone>(m: &[Vec<T>], keep: &[usize]) -> Vec<Vec<T>> {
    m.iter()
        .map(|r| keep.iter().map(|&j| r[j].clone()).collect())
        .collect()
}

fn hamming(a: &[(i64, Option<i64>)], b: &[(i64, Option<i64>)]) -> usize {
    if a.len() != b.len() {
        return a.len().max(b.len());
    }
    a.iter().zip(b).filter(|(x, y)| x != y).count()
}

fn first_part(label: &str) -> &str {
    label.split('–').next().unwrap_or(label)
}

fn second_part(label: &str) -> &str {
    label.split('–').nth(1).unwrap_or(label)
}

/// Avalia planos fixados por política, recalculando todas as perdas (porte de `evaluate_plans`).
pub fn evaluate_plans(
    input: &EvalInput<'_>,
    model: &mut dyn LossModel,
    p: &EvalProtocol,
) -> Result<EvalReport, EvalError> {
    let w = input.weights.unwrap_or(p.default_weights);
    let split = split_scenarios(input.climate, p)?;
    let mut report = EvalReport {
        outcome: String::new(),
        headline: String::new(),
        protocol: p.to_value(),
        protocol_config_hash: p.config_hash(),
        weights: w,
        n_plans: input.plans.len(),
        construction: split.construction.iter().map(|s| s.label.clone()).collect(),
        test: split.test.iter().map(|s| s.label.clone()).collect(),
        excluded_scenarios: split.excluded.clone(),
        params_hash: None,
        mean_climate_hash: None,
        dry_threshold_mm: None,
        dry_test_scenarios: Vec::new(),
        m0_feasibility: None,
        p0_status: "nao_informado".into(),
        policies: Vec::new(),
        m5_price_of_robustness: None,
        comparisons: Vec::new(),
        audit: Vec::new(),
        sensitivity: Vec::new(),
        limitations: STANDARD_LIMITATIONS
            .iter()
            .map(|s| (*s).to_string())
            .collect(),
        diagnostics: Vec::new(),
    };

    if input.plans.is_empty() {
        report.outcome = OUTCOME_INFEASIBLE.into();
        report.headline = "INVIAVEL: nenhum plano viável sob ZARC ≤ 20% no solo informado; a avaliação não roda (limite não relaxado).".into();
        return Ok(report);
    }
    if split.construction.len() < p.min_construction_pairs || split.test.len() < p.min_test_pairs {
        report.outcome = OUTCOME_INSUFFICIENT.into();
        report.headline = format!(
            "DADOS_INSUFICIENTES: {} pares de construção válidos (mín. {}) e {} de teste (mín. {}); sem veredito.",
            split.construction.len(),
            p.min_construction_pairs,
            split.test.len(),
            p.min_test_pairs
        );
        return Ok(report);
    }

    let ids: Vec<&Vec<i64>> = input.plans.iter().map(|x| &x.id).collect();
    let mut id_to_idx: HashMap<&Vec<i64>, usize> = HashMap::new();
    for (i, id) in ids.iter().enumerate() {
        if id_to_idx.insert(id, i).is_some() {
            return err("IDs de plano duplicados em X");
        }
    }

    // Parâmetros e referências SÓ com construção.
    let fingerprint = model.fit(&split.construction)?;
    let cy: Vec<&AgriYear> = split.construction_years.iter().collect();
    let s_bar = mean_climate(&cy)?;
    let p25 = dry_threshold(&split.construction_years, p);
    report.params_hash = Some(fingerprint.clone());
    report.mean_climate_hash = Some(array_hash(&[&s_bar.precip, &s_bar.tmax, &s_bar.tmin]));
    report.dry_threshold_mm = Some(p25);
    if let Some(h) = &input.optimizer_params_hash {
        report.audit_set("optimizer_params_hash", json!(h));
        report.audit_set("params_hash_match", json!(*h == fingerprint));
    }

    let n = input.plans.len();
    let cons_refs: Vec<&Scenario> = split.construction.iter().collect();
    let test_refs: Vec<&Scenario> = split.test.iter().collect();
    let g_cons = component_matrix(n, &cons_refs, model)?;
    let g_bar: Vec<[f64; 3]> = component_matrix(n, &[&s_bar], model)?
        .into_iter()
        .map(|r| r[0])
        .collect();
    let g_test = component_matrix(n, &test_refs, model)?;

    // M0: o avaliador só aceita planos de X aprovados pelo verificador.
    let mut pol_idx: BTreeMap<String, usize> = BTreeMap::new();
    let mut violations: Vec<(String, Vec<String>)> = Vec::new();
    let mut m0_pass = 0usize;
    for pol in ["P1", "P1b", "P2"] {
        let Some(Some(pid)) = input.policy_ids.get(pol) else {
            return err(format!("política {pol} sem plano"));
        };
        let mut v = Vec::new();
        match id_to_idx.get(pid) {
            None => v.push("plano fora de X (viola ZARC ≤ 20% ou calendário)".to_string()),
            Some(&i) => {
                if let Some(check) = input.check_plan {
                    v.extend(check(i));
                }
            }
        }
        if v.is_empty() {
            m0_pass += 1;
            pol_idx.insert(pol.to_string(), id_to_idx[pid]);
        } else {
            violations.push((pol.to_string(), v));
        }
    }
    report.m0_feasibility = Some(m0_pass as f64 / 3.0);
    if !violations.is_empty() {
        report.audit_set(
            "M0_violations",
            json!(violations.iter().cloned().collect::<BTreeMap<_, _>>()),
        );
        return err(format!("gate M0 < 100%: {violations:?}"));
    }

    let p0 = input.policy_ids.get("P0").cloned().flatten();
    match &p0 {
        None => {
            report.p0_status = input
                .p0_status
                .clone()
                .unwrap_or_else(|| "nao_informado".into())
        }
        Some(pid) => {
            let v0 = match (id_to_idx.get(pid), input.check_plan) {
                (Some(&i), Some(check)) => check(i),
                _ => Vec::new(),
            };
            if let (Some(&i), true) = (id_to_idx.get(pid), v0.is_empty()) {
                pol_idx.insert("P0".into(), i);
                report.p0_status = "viavel".into();
            } else {
                report.p0_status = "inviavel_sob_ZARC_20".into();
                let v = if v0.is_empty() {
                    vec!["plano fora de X".to_string()]
                } else {
                    v0
                };
                report.audit_set("P0_violations", json!(v));
            }
        }
    }

    // Auditoria: a escolha do otimizador bate com a regra do protocolo?
    let l_cons = losses(&g_cons, &w);
    let l_bar: Vec<f64> = g_bar
        .iter()
        .map(|c| c[0] * w[0] + c[1] * w[1] + c[2] * w[2])
        .collect();
    let l_test = losses(&g_test, &w);
    let refsel = reference_selection(&l_cons, &l_bar, &ids, p.decimals);
    report.audit_set(
        "reference_selection",
        json!(refsel
            .iter()
            .map(|(k, v)| (k.to_string(), ids[*v].clone()))
            .collect::<BTreeMap<_, _>>()),
    );
    report.audit_set(
        "optimizer_matches_reference",
        json!(refsel
            .iter()
            .map(|(k, v)| (k.to_string(), ids[*v] == ids[pol_idx[*k]]))
            .collect::<BTreeMap<_, _>>()),
    );

    let r_test = regret(&l_test);
    let r_cons = regret(&l_cons);
    let eps = p.epsilon;
    let dry_flags: Vec<bool> = split
        .test
        .iter()
        .map(|sc| {
            let (a, b) = scenario_dry_season_totals(sc, p);
            a < p25 || b < p25
        })
        .collect();
    report.dry_test_scenarios = split
        .test
        .iter()
        .zip(&dry_flags)
        .filter(|(_, f)| **f)
        .map(|(s, _)| s.label.clone())
        .collect();
    let nonoverlap: Vec<bool> = split
        .test
        .iter()
        .map(|s| (s.start - p.test_first_start).rem_euclid(2) == 0)
        .collect();

    let loo = leave_one_out(input, &split, model, &g_cons, &w, &ids, p, &pol_idx)?;

    for pol in POLICIES {
        let Some(&i) = pol_idx.get(pol) else {
            if pol == "P0" {
                report.policies.push(PolicyMetrics {
                    policy: pol.into(),
                    plan_id: p0.clone(),
                    feasible: false,
                    ..Default::default()
                });
            }
            continue;
        };
        let r = &r_test[i];
        let lt = &l_test[i];
        let dry_l: Vec<f64> = lt
            .iter()
            .zip(&dry_flags)
            .filter(|(_, f)| **f)
            .map(|(v, _)| *v)
            .collect();
        let dry_r: Vec<f64> = r
            .iter()
            .zip(&dry_flags)
            .filter(|(_, f)| **f)
            .map(|(v, _)| *v)
            .collect();
        let any_dry = !dry_l.is_empty();
        report.policies.push(PolicyMetrics {
            policy: pol.into(),
            plan_id: Some(ids[i].clone()),
            feasible: true,
            m1_max_regret: Some(row_max(r)),
            m2_mean_regret: Some(np_mean(r)),
            m2_median_regret: Some(np_median(r)),
            m3_dry_mean_loss: any_dry.then(|| np_mean(&dry_l)),
            m3_dry_mean_regret: any_dry.then(|| np_mean(&dry_r)),
            m4a_frac_regret_le_eps: Some(
                r.iter().filter(|v| **v <= eps).count() as f64 / r.len() as f64,
            ),
            m4b_iqr_loss: Some(np_quantile(lt, 0.75) - np_quantile(lt, 0.25)),
            m4c_loo_mean_hamming: loo.get(pol).copied(),
            mean_loss_test: Some(np_mean(lt)),
            loss_mean_climate: Some(l_bar[i]),
            per_scenario_regret: r.clone(),
            per_scenario_loss: lt.clone(),
        });
    }

    report.m5_price_of_robustness = Some(l_bar[pol_idx["P2"]] - l_bar[pol_idx["P1"]]);
    let cmr: Map<String, Value> = pol_idx
        .iter()
        .map(|(k, &v)| (k.clone(), fl(row_max(&r_cons[v]))))
        .collect();
    let n_dry = dry_flags.iter().filter(|f| **f).count();
    report.diagnostics = vec![
        (
            "construction_max_regret_by_policy".into(),
            Value::Object(cmr),
        ),
        ("n_dry_test".into(), json!(n_dry)),
        (
            "M3_inferential".into(),
            json!(n_dry >= p.min_dry_pairs_inferential),
        ),
    ];

    let m0_ok = report.m0_feasibility == Some(1.0);
    let sel_ref: BTreeMap<String, usize> = pol_idx.clone();
    for b in ["P1b", "P1"] {
        report.comparisons.push(compare(
            "P2",
            b,
            &r_test,
            &sel_ref,
            &ids,
            &nonoverlap,
            m0_ok,
            p,
        ));
    }
    let primary = &report.comparisons[0];
    let secondary = &report.comparisons[1];
    report.outcome = primary.classification.into();
    report.headline = format!(
        "{}: P2 (minimax regret) vs P1b (perda esperada) no teste {}–{}; mediana D = {:+.4}, IC 95% [{:+.4}, {:+.4}], M1 {:.4} vs {:.4}. P2 vs P1 (clima médio): {}.",
        primary.classification,
        first_part(&split.test[0].label),
        second_part(&split.test[split.test.len() - 1].label),
        primary.median_d,
        primary.ci95_median_d.0,
        primary.ci95_median_d.1,
        primary.m1_a,
        primary.m1_b,
        secondary.classification,
    );

    if input.run_grid {
        report.sensitivity = grid(&g_cons, &g_bar, &g_test, &ids, &nonoverlap, p);
    }
    Ok(report)
}

#[allow(clippy::too_many_arguments)]
fn compare(
    a: &str,
    b: &str,
    r_test: &[Vec<f64>],
    idx: &BTreeMap<String, usize>,
    ids: &[&Vec<i64>],
    nonoverlap: &[bool],
    m0_ok: bool,
    p: &EvalProtocol,
) -> Comparison {
    let (ia, ib) = (idx[a], idx[b]);
    let ra = &r_test[ia];
    let rb = &r_test[ib];
    let d: Vec<f64> = ra
        .iter()
        .zip(rb)
        .map(|(x, y)| np_round(x - y, p.decimals))
        .collect();
    let eps = p.epsilon;
    let identical = ids[ia] == ids[ib];
    let ci_med = block_bootstrap_ci(&d, p.block_size, p.n_boot, p.seed, BootStat::Median);
    let ci_mean = block_bootstrap_ci(&d, p.block_size, p.n_boot, p.seed, BootStat::Mean);
    let (m1a, m1b) = (row_max(ra), row_max(rb));
    let (cls, reason) = classify(identical, Some(ci_med), m1a, m1b, m0_ok, eps, p.rel_gain);
    let d_non: Vec<f64> = d
        .iter()
        .zip(nonoverlap)
        .filter(|(_, m)| **m)
        .map(|(v, _)| *v)
        .collect();
    Comparison {
        a: a.into(),
        b: b.into(),
        identical_plans: identical,
        n: d.len(),
        wins: d.iter().filter(|v| **v < -eps).count(),
        ties: d.iter().filter(|v| v.abs() <= eps).count(),
        losses: d.iter().filter(|v| **v > eps).count(),
        median_d: np_median(&d),
        ci95_median_d: ci_med,
        mean_d: np_mean(&d),
        ci95_mean_d: ci_mean,
        wilcoxon: wilcoxon_exact(&d_non, p.decimals),
        m1_a: m1a,
        m1_b: m1b,
        classification: cls,
        reason,
    }
}

#[allow(clippy::too_many_arguments)]
fn leave_one_out(
    input: &EvalInput<'_>,
    split: &ScenarioSplit,
    model: &mut dyn LossModel,
    g_cons: &[Vec<[f64; 3]>],
    w: &[f64; 3],
    ids: &[&Vec<i64>],
    p: &EvalProtocol,
    pol_idx: &BTreeMap<String, usize>,
) -> Result<BTreeMap<&'static str, f64>, EvalError> {
    let n = split.construction.len();
    let years_by_start: BTreeMap<i32, &AgriYear> = split
        .construction_years
        .iter()
        .map(|y| (y.start, y))
        .collect();
    let mut bar_cache: BTreeMap<Vec<i32>, Vec<f64>> = BTreeMap::new();
    let mut hams: BTreeMap<&'static str, Vec<f64>> =
        BTreeMap::from([("P1", vec![]), ("P1b", vec![]), ("P2", vec![])]);
    let np = input.plans.len();
    for k in 0..n {
        let keep: Vec<usize> = (0..n).filter(|j| *j != k).collect();
        let lc = losses(&select_cols(g_cons, &keep), w);
        let yrs: Vec<i32> = keep
            .iter()
            .flat_map(|&j| split.construction[j].years.iter().copied())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        if !bar_cache.contains_key(&yrs) {
            let ys: Vec<&AgriYear> = yrs.iter().map(|y| years_by_start[y]).collect();
            let sb = mean_climate(&ys)?;
            let gb = component_matrix(np, &[&sb], model)?;
            let lb: Vec<f64> = gb
                .iter()
                .map(|r| r[0][0] * w[0] + r[0][1] * w[1] + r[0][2] * w[2])
                .collect();
            bar_cache.insert(yrs.clone(), lb);
        }
        let sel = reference_selection(&lc, &bar_cache[&yrs], ids, p.decimals);
        for (pol, v) in hams.iter_mut() {
            let a = &input.plans[sel[pol]].genes;
            let b = &input.plans[pol_idx[*pol]].genes;
            v.push(hamming(a, b) as f64);
        }
    }
    Ok(hams.into_iter().map(|(k, v)| (k, np_mean(&v))).collect())
}

fn grid(
    g_cons: &[Vec<[f64; 3]>],
    g_bar: &[[f64; 3]],
    g_test: &[Vec<[f64; 3]>],
    ids: &[&Vec<i64>],
    nonoverlap: &[bool],
    p: &EvalProtocol,
) -> Vec<Value> {
    let mut rows = Vec::new();
    for (gi, w) in weight_grid().iter().enumerate() {
        let lc = losses(g_cons, w);
        let lb: Vec<f64> = g_bar
            .iter()
            .map(|c| c[0] * w[0] + c[1] * w[1] + c[2] * w[2])
            .collect();
        let lt = losses(g_test, w);
        let sel: BTreeMap<String, usize> = reference_selection(&lc, &lb, ids, p.decimals)
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();
        let rt = regret(&lt);
        let mut row = json!({
            "grid_index": gi + 1,
            "weights": w.iter().map(|x| fl(*x)).collect::<Vec<_>>(),
            "is_default": *w == p.default_weights,
            "plans": sel.iter().map(|(k, v)| (k.clone(), json!(ids[*v]))).collect::<Map<_, _>>(),
            "M1": sel.iter().map(|(k, v)| (k.clone(), fl(row_max(&rt[*v])))).collect::<Map<_, _>>(),
            "M2_mean": sel.iter().map(|(k, v)| (k.clone(), fl(np_mean(&rt[*v])))).collect::<Map<_, _>>(),
        });
        for b in ["P1b", "P1"] {
            let c = compare("P2", b, &rt, &sel, ids, nonoverlap, true, p);
            row[format!("P2_vs_{b}")] = json!({
                "classification": c.classification,
                "median_D": fl(c.median_d),
                "ci95": [fl(c.ci95_median_d.0), fl(c.ci95_median_d.1)],
                "identical": c.identical_plans,
            });
        }
        rows.push(row);
    }
    rows
}

// ---------------------------------------------------------------------------------------------
// Markdown (mesmo formato e destaque para qualquer desfecho)
// ---------------------------------------------------------------------------------------------

fn fmt_opt(v: Option<f64>, nd: usize) -> String {
    v.map_or_else(|| "—".to_string(), |x| format!("{x:.nd$}"))
}

fn py_list_ints(v: &Option<Vec<i64>>) -> String {
    match v {
        None => "None".into(),
        Some(v) => format!(
            "[{}]",
            v.iter().map(i64::to_string).collect::<Vec<_>>().join(", ")
        ),
    }
}

fn py_list_floats(v: &[f64]) -> String {
    format!(
        "[{}]",
        v.iter()
            .map(|x| py_float_repr(*x))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn py_bool(b: bool) -> &'static str {
    if b {
        "True"
    } else {
        "False"
    }
}

/// `json.dumps(obj, ensure_ascii=False)` do Python (separadores ", " e ": ", ordem de inserção).
fn py_json_pretty_inline(v: &Value) -> String {
    match v {
        Value::Object(m) => {
            let parts: Vec<String> = m
                .iter()
                .map(|(k, x)| {
                    format!(
                        "{}: {}",
                        canonical_json(&json!(k)),
                        py_json_pretty_inline(x)
                    )
                })
                .collect();
            format!("{{{}}}", parts.join(", "))
        }
        Value::Array(a) => {
            format!(
                "[{}]",
                a.iter()
                    .map(py_json_pretty_inline)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
        other => canonical_json(other),
    }
}

pub fn render_markdown(r: &EvalReport) -> String {
    let mut lines: Vec<String> = vec![
        format!("**DESFECHO: {}** — {}", r.outcome, r.headline),
        String::new(),
    ];
    lines.push(format!(
        "Protocolo v{} (SHA-256 {}); config do protocolo {}.",
        PROTOCOL_VERSION, PROTOCOL_SHA256, r.protocol_config_hash
    ));
    lines.push(format!(
        "Pesos (exposição, cobertura, diversidade) = {}. |X| = {}. P0: {}. M0 = {}.",
        py_list_floats(&r.weights),
        r.n_plans,
        r.p0_status,
        fmt_opt(r.m0_feasibility, 2)
    ));
    lines.push(format!(
        "Construção: {} pares; teste: {} pares; excluídos: {}.",
        r.construction.len(),
        r.test.len(),
        r.excluded_scenarios.len()
    ));
    lines.push(String::new());
    if !r.policies.is_empty() {
        lines.push("| Política | Plano | M1 regret máx | M2 média | M2 mediana | M3 L seco | M3 R seco | M4a R≤ε | M4b IQR L | M4c LOO Hamming | L(s̄) |".into());
        lines.push("|---|---|---|---|---|---|---|---|---|---|---|".into());
        for pol in POLICIES {
            let Some(pm) = r.policies.iter().find(|x| x.policy == pol) else {
                continue;
            };
            if !pm.feasible {
                lines.push(format!(
                    "| {pol} | {} | inviável sob ZARC ≤ 20% (fora da comparação) | | | | | | | | |",
                    py_list_ints(&pm.plan_id)
                ));
                continue;
            }
            lines.push(format!(
                "| {pol} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
                py_list_ints(&pm.plan_id),
                fmt_opt(pm.m1_max_regret, 4),
                fmt_opt(pm.m2_mean_regret, 4),
                fmt_opt(pm.m2_median_regret, 4),
                fmt_opt(pm.m3_dry_mean_loss, 4),
                fmt_opt(pm.m3_dry_mean_regret, 4),
                fmt_opt(pm.m4a_frac_regret_le_eps, 2),
                fmt_opt(pm.m4b_iqr_loss, 4),
                fmt_opt(pm.m4c_loo_mean_hamming, 2),
                fmt_opt(pm.loss_mean_climate, 4),
            ));
        }
        let n_dry = r
            .diagnostics
            .iter()
            .find(|(k, _)| k == "n_dry_test")
            .map(|(_, v)| v.to_string())
            .unwrap_or_default();
        let inferential = r
            .diagnostics
            .iter()
            .any(|(k, v)| k == "M3_inferential" && v == &json!(true));
        lines.push(String::new());
        lines.push(format!(
            "M3: {} pares secos no teste (P25 out–mar da construção = {} mm){}.",
            n_dry,
            fmt_opt(r.dry_threshold_mm, 1),
            if inferential {
                ""
            } else {
                "; n < 3, só descritivo"
            }
        ));
        lines.push(format!(
            "M5 preço da robustez L(P2, s̄) − L(P1, s̄) = {}.",
            fmt_opt(r.m5_price_of_robustness, 4)
        ));
        lines.push(String::new());
        lines.push("| Comparação | Idênticos | V/E/D (ε) | Mediana D | IC 95% mediana | Média D | IC 95% média | Wilcoxon 7 pares (p) | Classe | Motivo |".into());
        lines.push("|---|---|---|---|---|---|---|---|---|---|".into());
        for c in &r.comparisons {
            lines.push(format!(
                "| {} vs {} | {} | {}/{}/{} | {:.4} | [{:.4}, {:.4}] | {:.4} | [{:.4}, {:.4}] | {:.4} (n≠0={}) | **{}** | {} |",
                c.a, c.b, py_bool(c.identical_plans), c.wins, c.ties, c.losses,
                c.median_d, c.ci95_median_d.0, c.ci95_median_d.1, c.mean_d,
                c.ci95_mean_d.0, c.ci95_mean_d.1, c.wilcoxon.p_two_sided, c.wilcoxon.n_nonzero,
                c.classification, c.reason
            ));
        }
    }
    if !r.sensitivity.is_empty() {
        lines.push(String::new());
        lines.push(
            "Sensibilidade (grade de 15 pesos; só relatada, não muda o veredito em w*):".into(),
        );
        lines.push(String::new());
        lines.push("| # | Pesos | P2 vs P1b | P2 vs P1 | M1 P1 / P1b / P2 |".into());
        lines.push("|---|---|---|---|---|".into());
        for row in &r.sensitivity {
            let ws: Vec<f64> = row["weights"]
                .as_array()
                .map(|a| a.iter().filter_map(Value::as_f64).collect())
                .unwrap_or_default();
            let m1 = |k: &str| fmt_opt(row["M1"][k].as_f64(), 4);
            lines.push(format!(
                "| {}{} | {} | {} | {} | {} / {} / {} |",
                row["grid_index"],
                if row["is_default"] == json!(true) {
                    " (padrão)"
                } else {
                    ""
                },
                py_list_floats(&ws),
                row["P2_vs_P1b"]["classification"].as_str().unwrap_or(""),
                row["P2_vs_P1"]["classification"].as_str().unwrap_or(""),
                m1("P1"),
                m1("P1b"),
                m1("P2")
            ));
        }
    }
    if !r.audit.is_empty() {
        // ordem de inserção do Python (o Map do serde_json é ordenado)
        let parts: Vec<String> = r
            .audit
            .iter()
            .map(|(k, v)| {
                format!(
                    "{}: {}",
                    canonical_json(&json!(k)),
                    py_json_pretty_inline(v)
                )
            })
            .collect();
        lines.push(String::new());
        lines.push(format!("Auditoria: {{{}}}", parts.join(", ")));
    }
    lines.push(String::new());
    lines.push("Limitações:".into());
    for l in &r.limitations {
        lines.push(format!("- {l}"));
    }
    lines.join("\n") + "\n"
}
