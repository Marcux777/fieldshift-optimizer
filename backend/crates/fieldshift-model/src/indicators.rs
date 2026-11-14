// AI-assisted (OpenBot agent)
//! Indicadores g1 (exposição climática), g2 (cobertura vegetal) e g3 (diversidade), todos em [0, 1].
//!
//! Porte fiel de `src/fieldshift/indicators.py` (commit `16cc391`). Função pura:
//! `loss_components(plan, climate_pair, params) -> [g1, g2, g3]`. Só precipitação e temperatura.
//!
//! Paridade numérica: `ndarray.sum()` do numpy usa soma em pares (blocos de 8 acumuladores);
//! `np_sum` a reproduz, porque comparações como `sum(P[s, s+10)) < 20` são sensíveis ao último bit.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Mutex;

pub use crate::compat::np_sum;
use crate::compat::{fsum, median, sha256_hex};

use crate::catalog::{Catalog, Crop, Plan, DAYS_PER_YEAR, HORIZON_DAYS};

const TOL: f64 = 1e-12;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[error("{0}")]
pub struct IndicatorError(pub String);

type Res<T> = Result<T, IndicatorError>;

fn err<T>(msg: impl Into<String>) -> Res<T> {
    Err(IndicatorError(msg.into()))
}

/// Mediana por janela dos anos agrícolas de CONSTRUÇÃO (`n × 365`), só para o e3.
#[derive(Debug)]
pub struct ConstructionClimatology {
    p: Vec<f64>,
    n_years: usize,
    pub labels: Vec<i32>,
    memo: Mutex<HashMap<(usize, usize), f64>>,
}

impl ConstructionClimatology {
    pub fn new(precip_years: &[Vec<f64>], labels: Vec<i32>) -> Res<Self> {
        if precip_years.is_empty() {
            return err("precip_years deve ter forma (n>=1, 365)");
        }
        let mut p = Vec::with_capacity(precip_years.len() * DAYS_PER_YEAR as usize);
        for row in precip_years {
            if row.len() != DAYS_PER_YEAR as usize {
                return err("precip_years deve ter forma (n>=1, 365)");
            }
            if row.iter().any(|v| !v.is_finite()) {
                return err("climatologia de construção com dado ausente");
            }
            p.extend_from_slice(row);
        }
        Ok(Self {
            p,
            n_years: precip_years.len(),
            labels,
            memo: Mutex::new(HashMap::new()),
        })
    }

    pub fn n_years(&self) -> usize {
        self.n_years
    }

    /// Mediana (numpy) da soma de P em `[lo, hi)` do ano agrícola, entre os anos de construção.
    pub fn median_window_sum(&self, lo: i32, hi: i32) -> Res<f64> {
        if !(0 <= lo && lo < hi && hi <= DAYS_PER_YEAR) {
            return err(format!("janela fora do ano agrícola: [{lo}, {hi})"));
        }
        let key = (lo as usize, hi as usize);
        let mut memo = self
            .memo
            .lock()
            .map_err(|_| IndicatorError("lock envenenado".into()))?;
        if let Some(v) = memo.get(&key) {
            return Ok(*v);
        }
        let w = DAYS_PER_YEAR as usize;
        let sums: Vec<f64> = (0..self.n_years)
            .map(|y| np_sum(&self.p[y * w + key.0..y * w + key.1]))
            .collect();
        let m = median(&sums);
        memo.insert(key, m);
        Ok(m)
    }

    /// SHA-256 dos bytes float64 little-endian (ordem de linha), como `ndarray.tobytes()`.
    pub fn sha256(&self) -> String {
        let bytes: Vec<u8> = self.p.iter().flat_map(|v| v.to_le_bytes()).collect();
        sha256_hex(&bytes)
    }
}

/// Clima diário de um par de anos agrícolas (730 dias, sem 29/fev), sem nenhum dado ausente.
#[derive(Debug)]
pub struct ClimatePair {
    pub p: Vec<f64>,
    pub tx: Vec<f64>,
    pub tn: Vec<f64>,
    pub label: String,
    cache: Mutex<HashMap<(usize, i32), OccurrenceResult>>,
}

impl ClimatePair {
    pub fn new(p: Vec<f64>, tx: Vec<f64>, tn: Vec<f64>, label: &str) -> Res<Self> {
        for (name, a) in [("P", &p), ("TX", &tx), ("TN", &tn)] {
            if a.len() != HORIZON_DAYS as usize {
                return err(format!(
                    "{name} deve ter {HORIZON_DAYS} dias, veio {}",
                    a.len()
                ));
            }
            if a.iter().any(|v| !v.is_finite()) {
                return err(format!(
                    "{name} com dado ausente: cenário inválido não chega aqui"
                ));
            }
        }
        Ok(Self {
            p,
            tx,
            tn,
            label: label.to_string(),
            cache: Mutex::new(HashMap::new()),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Thermal {
    pub heat_c: Option<f64>,
    pub frost_c: Option<f64>,
    pub frost_strict: bool,
}

#[derive(Debug)]
pub struct IndicatorParams {
    pub crops: Vec<Crop>,
    pub emergence_days: i32,
    pub brachiaria_desiccation_days: i32,
    pub thermal: BTreeMap<String, Thermal>,
    pub e1_window_days: i32,
    pub e1_min_mm: f64,
    pub e2_dry_day_mm: f64,
    pub e5_window_days: i32,
    pub e5_rain_day_mm: f64,
    pub e5_rain_days_min: i32,
    pub component_weights: [f64; 5],
    pub horizon_days: i32,
    pub species_universe: Vec<String>,
    pub max_species: i32,
    pub max_repetitions: i32,
    pub default_weights: [f64; 3],
    pub rounding_decimals: i32,
    pub sha256: String,
    pub reference: Option<ConstructionClimatology>,
}

impl IndicatorParams {
    /// Cópia sem a climatologia de referência (a referência tem estado interno e não é clonável).
    pub fn clone_without_reference(&self) -> Self {
        Self {
            crops: self.crops.clone(),
            emergence_days: self.emergence_days,
            brachiaria_desiccation_days: self.brachiaria_desiccation_days,
            thermal: self.thermal.clone(),
            e1_window_days: self.e1_window_days,
            e1_min_mm: self.e1_min_mm,
            e2_dry_day_mm: self.e2_dry_day_mm,
            e5_window_days: self.e5_window_days,
            e5_rain_day_mm: self.e5_rain_day_mm,
            e5_rain_days_min: self.e5_rain_days_min,
            component_weights: self.component_weights,
            horizon_days: self.horizon_days,
            species_universe: self.species_universe.clone(),
            max_species: self.max_species,
            max_repetitions: self.max_repetitions,
            default_weights: self.default_weights,
            rounding_decimals: self.rounding_decimals,
            sha256: self.sha256.clone(),
            reference: None,
        }
    }

    /// Troca a climatologia de referência (mediana de e3, só da construção).
    pub fn with_reference(mut self, r: ConstructionClimatology) -> Self {
        self.reference = Some(r);
        self
    }

    /// Lê `indicators.toml` (bytes) usando as culturas do catálogo.
    pub fn from_toml(raw: &[u8], cat: &Catalog) -> Res<Self> {
        type Table = toml::map::Map<String, toml::Value>;
        let text = std::str::from_utf8(raw).map_err(|_| IndicatorError("não é UTF-8".into()))?;
        let cfg: Table = text
            .parse()
            .map_err(|e| IndicatorError(format!("indicators.toml inválido: {e}")))?;
        let tb = |t: &Table, k: &str| -> Res<Table> {
            t.get(k)
                .and_then(|v| v.as_table())
                .cloned()
                .ok_or_else(|| IndicatorError(format!("tabela ausente: {k}")))
        };
        let num = |t: &Table, k: &str| -> Res<f64> {
            t.get(k)
                .and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64)))
                .ok_or_else(|| IndicatorError(format!("número ausente: {k}")))
        };
        let int = |t: &Table, k: &str| -> Res<i64> {
            t.get(k)
                .and_then(|v| v.as_integer())
                .ok_or_else(|| IndicatorError(format!("inteiro ausente: {k}")))
        };
        let ex = tb(&cfg, "exposure")?;
        let e4 = tb(&ex, "e4")?;
        let mut thermal = BTreeMap::new();
        for (key, v) in &e4 {
            if let Some(t) = v.as_table() {
                thermal.insert(
                    key.clone(),
                    Thermal {
                        heat_c: t.get("heat_c").and_then(|x| x.as_float()),
                        frost_c: t.get("frost_c").and_then(|x| x.as_float()),
                        frost_strict: t
                            .get("frost_strict")
                            .and_then(|x| x.as_bool())
                            .unwrap_or(false),
                    },
                );
            }
        }
        let cw_v: Vec<f64> = ex
            .get("component_weights")
            .and_then(|v| v.as_array())
            .ok_or_else(|| IndicatorError("component_weights ausente".into()))?
            .iter()
            .map(|x| x.as_float().or_else(|| x.as_integer().map(|i| i as f64)))
            .collect::<Option<_>>()
            .ok_or_else(|| IndicatorError("component_weights inválido".into()))?;
        if cw_v.len() != 5
            || (cw_v.iter().sum::<f64>() - 1.0).abs() > TOL
            || cw_v.iter().any(|w| *w < 0.0)
        {
            return err("component_weights deve ter 5 pesos >= 0 somando 1");
        }
        let loss = tb(&cfg, "loss")?;
        let dw = tb(&loss, "default_weights")?;
        let weights = [
            num(&dw, "exposure")?,
            num(&dw, "cover")?,
            num(&dw, "diversity")?,
        ];
        if (weights.iter().sum::<f64>() - 1.0).abs() > TOL {
            return err("default_weights deve somar 1");
        }
        let div = tb(&cfg, "diversity")?;
        let cover = tb(&cfg, "cover")?;
        if int(&cover, "T")? != HORIZON_DAYS as i64 {
            return err("cover.T deve ser 730");
        }
        let universe: Vec<String> = div
            .get("species_universe")
            .and_then(|v| v.as_array())
            .ok_or_else(|| IndicatorError("species_universe ausente".into()))?
            .iter()
            .filter_map(|x| x.as_str().map(str::to_string))
            .collect();
        let (e1, e2, e5) = (tb(&ex, "e1")?, tb(&ex, "e2")?, tb(&ex, "e5")?);
        Ok(Self {
            crops: cat.crops.clone(),
            emergence_days: cat.emergence_days,
            brachiaria_desiccation_days: cat.brachiaria_desiccation_days,
            thermal,
            e1_window_days: int(&e1, "window_days")? as i32,
            e1_min_mm: num(&e1, "establishment_min_mm")?,
            e2_dry_day_mm: num(&e2, "dry_day_mm")?,
            e5_window_days: int(&e5, "window_days")? as i32,
            e5_rain_day_mm: num(&e5, "rain_day_mm")?,
            e5_rain_days_min: int(&e5, "rain_days_min")? as i32,
            component_weights: [cw_v[0], cw_v[1], cw_v[2], cw_v[3], cw_v[4]],
            horizon_days: int(&cover, "T")? as i32,
            species_universe: universe,
            max_species: int(&div, "max_species")? as i32,
            max_repetitions: int(&div, "max_repetitions")? as i32,
            default_weights: weights,
            rounding_decimals: int(&loss, "rounding_decimals")? as i32,
            sha256: sha256_hex(raw),
            reference: None,
        })
    }
}

// --------------------------------------------------------------------------- ocorrência
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OccurrenceResult {
    pub e1: f64,
    pub e2: f64,
    pub e3: f64,
    pub e4: f64,
    pub e5: f64,
    pub eps: f64,
}

/// Maior sequência de `true` consecutivos.
pub fn longest_true_run(mask: &[bool]) -> usize {
    let (mut run, mut best) = (0usize, 0usize);
    for &m in mask {
        if m {
            run += 1;
            best = best.max(run);
        } else {
            run = 0;
        }
    }
    best
}

/// (emergência e, colheita h, início da fase crítica, fim da fase crítica, n_crit).
pub fn crop_phase_bounds(
    crop: &Crop,
    emergence_days: i32,
    sow_t: i32,
) -> (i32, i32, i32, i32, i32) {
    let (p1, p2, p3) = (crop.phase_days[0], crop.phase_days[1], crop.phase_days[2]);
    let e = sow_t + emergence_days;
    let c0 = e + p1 + p2;
    (e, e + crop.cycle_days, c0, c0 + p3, p3)
}

fn slice(a: &[f64], lo: i32, hi: i32) -> &[f64] {
    let n = a.len() as i32;
    let (lo, hi) = (lo.clamp(0, n), hi.clamp(0, n));
    if lo >= hi {
        &[]
    } else {
        &a[lo as usize..hi as usize]
    }
}

/// e1..e5 e eps de uma ocorrência de cultura de grão (memoizado no par; função pura).
pub fn occurrence_indicators(
    pair: &ClimatePair,
    params: &IndicatorParams,
    crop_idx: usize,
    sow_t: i32,
) -> Res<OccurrenceResult> {
    {
        let cache = pair
            .cache
            .lock()
            .map_err(|_| IndicatorError("lock envenenado".into()))?;
        if let Some(r) = cache.get(&(crop_idx, sow_t)) {
            return Ok(*r);
        }
    }
    let crop = &params.crops[crop_idx];
    if crop.is_fallow || !crop.grain {
        return err(format!("{} não é ocorrência de grão", crop.key));
    }
    let Some(reference) = params.reference.as_ref() else {
        return err("IndicatorParams sem climatologia de construção (e3 exige a mediana)");
    };
    let (e, h, c0, c1, n_crit) = crop_phase_bounds(crop, params.emergence_days, sow_t);
    if sow_t < 0 || h > HORIZON_DAYS {
        return err(format!("ocupação fora do horizonte: s={sow_t}, h={h}"));
    }
    let (p, tx, tn) = (&pair.p, &pair.tx, &pair.tn);
    let e1 = if np_sum(slice(p, sow_t, sow_t + params.e1_window_days)) < params.e1_min_mm {
        1.0
    } else {
        0.0
    };
    let crit: Vec<bool> = slice(p, c0, c1)
        .iter()
        .map(|v| *v < params.e2_dry_day_mm)
        .collect();
    let e2 = longest_true_run(&crit) as f64 / f64::from(n_crit);
    let pcrit = np_sum(slice(p, c0, c1));
    let year0 = DAYS_PER_YEAR * sow_t.div_euclid(DAYS_PER_YEAR);
    let pref = reference.median_window_sum(c0 - year0, c1 - year0)?;
    let e3 = if pref == 0.0 {
        0.0
    } else {
        f64::max(0.0, 1.0 - pcrit / pref)
    };
    let th = params.thermal.get(&crop.key).copied().unwrap_or_default();
    let mut comps: Vec<f64> = Vec::new();
    if let Some(heat) = th.heat_c {
        let n = slice(tx, c0, c1).iter().filter(|v| **v > heat).count();
        comps.push(n as f64 / f64::from(n_crit));
    }
    if let Some(frost) = th.frost_c {
        let seg = slice(tn, e, h);
        let n = if th.frost_strict {
            seg.iter().filter(|v| **v < frost).count()
        } else {
            seg.iter().filter(|v| **v <= frost).count()
        };
        comps.push(n as f64 / f64::from(crop.cycle_days));
    }
    let e4 = if comps.is_empty() {
        0.0
    } else {
        fsum(&comps) / comps.len() as f64
    };
    let rainy = slice(p, h - params.e5_window_days, h)
        .iter()
        .filter(|v| **v >= params.e5_rain_day_mm)
        .count() as i32;
    let e5 = if rainy >= params.e5_rain_days_min {
        1.0
    } else {
        0.0
    };
    let w = &params.component_weights;
    let eps = fsum(&[w[0] * e1, w[1] * e2, w[2] * e3, w[3] * e4, w[4] * e5]);
    if !(-TOL..=1.0 + TOL).contains(&eps) {
        return err(format!("eps fora de [0,1]: {eps}"));
    }
    let res = OccurrenceResult {
        e1,
        e2,
        e3,
        e4,
        e5,
        eps: eps.clamp(0.0, 1.0),
    };
    pair.cache
        .lock()
        .map_err(|_| IndicatorError("lock envenenado".into()))?
        .insert((crop_idx, sow_t), res);
    Ok(res)
}

// --------------------------------------------------------------------------- g1, g2, g3
pub fn exposure_g1(plan: &Plan, pair: &ClimatePair, params: &IndicatorParams) -> Res<f64> {
    let mut vals = Vec::new();
    for o in &plan.occupations {
        if let (false, true, Some(s)) = (o.is_fallow(), params.crops[o.crop].grain, o.sow_t) {
            vals.push(occurrence_indicators(pair, params, o.crop, s)?.eps);
        }
    }
    if vals.is_empty() {
        return err("plano sem cultura de grão");
    }
    Ok(fsum(&vals) / vals.len() as f64)
}

/// Dias (0..T-1) com cobertura viva: `[e, h)` de cada ocorrência sem falha de estabelecimento, mais a
/// braquiária do consórcio (de h até 15 d antes de J3; em J4 até o fim do horizonte).
pub fn cover_mask(plan: &Plan, pair: &ClimatePair, params: &IndicatorParams) -> Res<Vec<bool>> {
    let t = params.horizon_days;
    let mut cov = vec![false; t as usize];
    let s3 = plan.occupations[2].sow_t.unwrap_or(0);
    for o in &plan.occupations {
        let (Some(s), Some(e), Some(h)) = (o.sow_t, o.emerge_t, o.harvest_t) else {
            continue;
        };
        let crop = &params.crops[o.crop];
        let res = occurrence_indicators(pair, params, o.crop, s)?;
        if res.e1 == 1.0 {
            continue;
        }
        for d in e..h {
            cov[d as usize] = true;
        }
        if crop.brachiaria_cover_after_harvest {
            let end = if o.window == 3 {
                t
            } else {
                t.min(s3 - params.brachiaria_desiccation_days)
            };
            if end > h {
                for d in h..end {
                    cov[d as usize] = true;
                }
            }
        }
    }
    Ok(cov)
}

pub fn cover_g2(plan: &Plan, pair: &ClimatePair, params: &IndicatorParams) -> Res<f64> {
    let t = f64::from(params.horizon_days);
    let cov = cover_mask(plan, pair, params)?;
    let c1 = cov.iter().filter(|b| **b).count() as f64 / t;
    let unc: Vec<bool> = cov.iter().map(|b| !*b).collect();
    let c2 = longest_true_run(&unc) as f64;
    Ok(0.5 * (1.0 - c1) + 0.5 * c2 / t)
}

/// (D1 espécies distintas, D2 repetições r1+r2+r3).
pub fn diversity_terms(plan: &Plan, params: &IndicatorParams) -> Res<(i32, i32)> {
    let crops: Vec<&Crop> = plan
        .occupations
        .iter()
        .map(|o| &params.crops[o.crop])
        .collect();
    let mut species: BTreeSet<&str> = BTreeSet::new();
    for c in &crops {
        for s in &c.species {
            species.insert(s);
        }
    }
    let unknown: Vec<&&str> = species
        .iter()
        .filter(|s| !params.species_universe.iter().any(|u| u == **s))
        .collect();
    if !unknown.is_empty() {
        return err(format!("espécies fora do universo: {unknown:?}"));
    }
    let o = &plan.occupations;
    let r1 = i32::from(o[0].crop == o[2].crop);
    let r2 = i32::from(o[1].crop == o[3].crop && !o[1].is_fallow());
    let milho = |c: &Crop| c.species.iter().any(|s| s == "milho");
    let r3 =
        i32::from((milho(crops[0]) && milho(crops[1])) || (milho(crops[2]) && milho(crops[3])));
    Ok((species.len() as i32, r1 + r2 + r3))
}

pub fn diversity_g3(plan: &Plan, params: &IndicatorParams) -> Res<f64> {
    let (d1, d2) = diversity_terms(plan, params)?;
    if !(1..=params.max_species).contains(&d1) || d2 > params.max_repetitions {
        return err(format!("D1={d1} ou D2={d2} fora do intervalo teórico"));
    }
    Ok(
        0.5 * f64::from(params.max_species - d1) / f64::from(params.max_species - 1)
            + 0.5 * f64::from(d2) / f64::from(params.max_repetitions),
    )
}

/// `[g1, g2, g3]` do plano no par de anos agrícolas. Função pura; todos em [0, 1].
pub fn loss_components(plan: &Plan, pair: &ClimatePair, params: &IndicatorParams) -> Res<[f64; 3]> {
    let g = [
        exposure_g1(plan, pair, params)?,
        cover_g2(plan, pair, params)?,
        diversity_g3(plan, params)?,
    ];
    for v in g {
        if !(-TOL..=1.0 + TOL).contains(&v) {
            return err(format!("g fora de [0,1]: {g:?}"));
        }
    }
    Ok([
        g[0].clamp(0.0, 1.0),
        g[1].clamp(0.0, 1.0),
        g[2].clamp(0.0, 1.0),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn np_sum_follows_numpy_pairwise_order() {
        let a = vec![0.1f64; 10];
        let naive: f64 = a.iter().fold(0.0, |s, x| s + x);
        assert_eq!(naive, 0.999_999_999_999_999_9);
        assert_eq!(np_sum(&a), 1.0);
        assert_eq!(np_sum(&[]), 0.0);
        assert_eq!(np_sum(&[1.5]), 1.5);
        let b: Vec<f64> = (1..=300).map(f64::from).collect();
        assert_eq!(np_sum(&b), 45150.0);
    }
}
