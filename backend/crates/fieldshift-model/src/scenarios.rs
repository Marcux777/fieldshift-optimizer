// AI-assisted (OpenBot agent)
//! Cenários, split construção/teste e regra de dados ausentes (protocolo v1.1).
//!
//! Porte fiel da parte de cenários de `src/fieldshift/optimization.py` (commit `16cc391`):
//!
//! * cenário = par de anos agrícolas consecutivos (y, y+1), de 1/set a 31/ago, sem 29/fev (730 dias);
//! * construção: pares y = 1981..2008 (28); teste: y = 2010..2023 (14); o par 2009 não existe;
//! * a fatia por split é feita ANTES de qualquer cálculo: a construção nunca vê dado de teste;
//! * cenário inválido (NaN em qualquer das 4 variáveis, ou dia ausente) sai para TODAS as políticas.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{json, Value};

use crate::catalog::{DAYS_PER_YEAR, HORIZON_DAYS};
use crate::indicators::{ClimatePair, ConstructionClimatology};

pub const VARIABLES: [&str; 4] = ["PRECTOTCORR", "T2M", "T2M_MAX", "T2M_MIN"];
pub const CONSTRUCTION_FIRST_Y: i32 = 1981;
/// Par (2008/09, 2009/10).
pub const CONSTRUCTION_LAST_PAIR_Y: i32 = 2008;
/// Último ano agrícola de construção: 2009/10.
pub const CONSTRUCTION_LAST_YEAR: i32 = 2009;
pub const TEST_FIRST_Y: i32 = 2010;
/// Par (2023/24, 2024/25).
pub const TEST_LAST_PAIR_Y: i32 = 2023;
/// 31/08/2010 e 01/09/2010.
const CONSTRUCTION_END: (i32, u8, u8) = (2010, 8, 31);
const TEST_START: (i32, u8, u8) = (2010, 9, 1);

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[error("{0}")]
pub struct ScenarioError(pub String);

type Res<T> = Result<T, ScenarioError>;

fn err<T>(msg: impl Into<String>) -> Res<T> {
    Err(ScenarioError(msg.into()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Split {
    Construction,
    Test,
}

impl Split {
    pub fn name(self) -> &'static str {
        match self {
            Split::Construction => "construction",
            Split::Test => "test",
        }
    }
}

type Date = (i32, u8, u8);

/// Série diária com as 4 variáveis (ordem `VARIABLES`); ausente = NaN; datas únicas.
#[derive(Debug, Clone, Default)]
pub struct ClimateFrame {
    days: BTreeMap<Date, [f64; 4]>,
}

impl ClimateFrame {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, y: i32, m: u8, d: u8, vals: [f64; 4]) -> Res<()> {
        if self.days.insert((y, m, d), vals).is_some() {
            return err(format!(
                "índice do clima deve ser único: {y}-{m:02}-{d:02} duplicado"
            ));
        }
        Ok(())
    }

    pub fn from_rows<I: IntoIterator<Item = (i32, u8, u8, [f64; 4])>>(rows: I) -> Res<Self> {
        let mut f = Self::new();
        for (y, m, d, v) in rows {
            f.insert(y, m, d, v)?;
        }
        Ok(f)
    }

    pub fn len(&self) -> usize {
        self.days.len()
    }

    pub fn is_empty(&self) -> bool {
        self.days.is_empty()
    }

    pub fn get(&self, date: Date) -> Option<&[f64; 4]> {
        self.days.get(&date)
    }

    pub fn first_date(&self) -> Option<Date> {
        self.days.keys().next().copied()
    }

    pub fn last_date(&self) -> Option<Date> {
        self.days.keys().next_back().copied()
    }

    /// Cópia dos dias em `[lo, hi]` (limites opcionais, inclusivos).
    pub fn slice(&self, lo: Option<Date>, hi: Option<Date>) -> ClimateFrame {
        let days = self
            .days
            .iter()
            .filter(|(k, _)| lo.is_none_or(|l| **k >= l) && hi.is_none_or(|h| **k <= h))
            .map(|(k, v)| (*k, *v))
            .collect();
        ClimateFrame { days }
    }

    /// Edita um valor (usado por testes de vazamento e de ausentes).
    pub fn set(&mut self, date: Date, var: usize, v: f64) -> bool {
        match self.days.get_mut(&date) {
            Some(row) => {
                row[var] = v;
                true
            }
            None => false,
        }
    }

    pub fn remove(&mut self, date: Date) -> bool {
        self.days.remove(&date).is_some()
    }
}

/// Um ano agrícola (set..ago, 365 dias, sem 29/fev).
#[derive(Debug, Clone)]
pub struct AgYear {
    pub y: i32,
    pub p: Vec<f64>,
    pub tx: Vec<f64>,
    pub tn: Vec<f64>,
    /// Ausentes por variável (ordem `VARIABLES`).
    pub missing: [usize; 4],
}

impl AgYear {
    pub fn complete(&self) -> bool {
        self.missing.iter().all(|m| *m == 0)
    }
}

const AG_MONTHS: [(i32, u8, u8); 12] = [
    (0, 9, 30),
    (0, 10, 31),
    (0, 11, 30),
    (0, 12, 31),
    (1, 1, 31),
    (1, 2, 28),
    (1, 3, 31),
    (1, 4, 30),
    (1, 5, 31),
    (1, 6, 30),
    (1, 7, 31),
    (1, 8, 31),
];

pub fn agri_year_frame(frame: &ClimateFrame, y: i32) -> AgYear {
    let (mut p, mut tx, mut tn) = (Vec::new(), Vec::new(), Vec::new());
    let mut missing = [0usize; 4];
    for (dy, m, ndays) in AG_MONTHS {
        for d in 1..=ndays {
            let row = frame.get((y + dy, m, d)).copied().unwrap_or([f64::NAN; 4]);
            for (k, v) in row.iter().enumerate() {
                if v.is_nan() {
                    missing[k] += 1;
                }
            }
            p.push(row[0]);
            tx.push(row[2]);
            tn.push(row[3]);
        }
    }
    debug_assert_eq!(p.len(), DAYS_PER_YEAR as usize);
    AgYear {
        y,
        p,
        tx,
        tn,
        missing,
    }
}

#[derive(Debug)]
pub struct Scenario {
    pub y: i32,
    pub label: String,
    pub split: &'static str,
    pub valid: bool,
    pub reason: String,
    pub pair: Option<ClimatePair>,
}

#[derive(Debug)]
pub struct ScenarioSet {
    pub split: Split,
    pub scenarios: Vec<Scenario>,
    pub ag_years: BTreeMap<i32, AgYear>,
    pub n_expected: usize,
}

impl ScenarioSet {
    pub fn valid(&self) -> Vec<&Scenario> {
        self.scenarios.iter().filter(|s| s.valid).collect()
    }

    pub fn n_valid(&self) -> usize {
        self.scenarios.iter().filter(|s| s.valid).count()
    }

    pub fn missing_log(&self) -> Vec<Value> {
        let mut out = Vec::new();
        for (y, a) in &self.ag_years {
            if a.complete() {
                continue;
            }
            let mut m = serde_json::Map::new();
            m.insert("ag_year".into(), json!(y));
            for (k, n) in VARIABLES.iter().zip(a.missing) {
                if n > 0 {
                    m.insert((*k).into(), json!(n));
                }
            }
            out.push(Value::Object(m));
        }
        out
    }

    pub fn excluded(&self) -> Vec<Value> {
        self.scenarios
            .iter()
            .filter(|s| !s.valid)
            .map(|s| json!({"label": s.label, "y": s.y, "reason": s.reason}))
            .collect()
    }

    pub fn summary(&self) -> Value {
        json!({
            "n_expected": self.n_expected,
            "n_valid": self.n_valid(),
            "excluded": self.excluded(),
            "missing_log": self.missing_log(),
        })
    }
}

fn pair_label(y: i32) -> String {
    format!("{y}/{:02}+{}/{:02}", (y + 1) % 100, y + 1, (y + 2) % 100)
}

/// Representação `repr` de um `dict[str, int]` do Python (para o motivo de exclusão).
fn py_dict_repr(items: &[(String, usize)]) -> String {
    let body: Vec<String> = items.iter().map(|(k, v)| format!("'{k}': {v}")).collect();
    format!("{{{}}}", body.join(", "))
}

/// Cenários do split. A fatia é feita ANTES de qualquer cálculo.
pub fn build_scenarios(climate: &ClimateFrame, split: Split) -> Res<ScenarioSet> {
    let (frame, years, pair_ys): (ClimateFrame, Vec<i32>, Vec<i32>) = match split {
        Split::Construction => {
            let f = climate.slice(None, Some(CONSTRUCTION_END));
            if f.last_date().is_some_and(|d| d >= TEST_START) {
                return err("dado de teste na construção");
            }
            (
                f,
                (CONSTRUCTION_FIRST_Y..=CONSTRUCTION_LAST_YEAR).collect(),
                (CONSTRUCTION_FIRST_Y..=CONSTRUCTION_LAST_PAIR_Y).collect(),
            )
        }
        Split::Test => (
            climate.slice(Some(TEST_START), None),
            (TEST_FIRST_Y..=TEST_LAST_PAIR_Y + 1).collect(),
            (TEST_FIRST_Y..=TEST_LAST_PAIR_Y).collect(),
        ),
    };
    let ag: BTreeMap<i32, AgYear> = years
        .iter()
        .map(|y| (*y, agri_year_frame(&frame, *y)))
        .collect();
    let mut scen = Vec::new();
    for &y in &pair_ys {
        let (a, b) = (&ag[&y], &ag[&(y + 1)]);
        let label = pair_label(y);
        if a.complete() && b.complete() {
            let cat = |u: &Vec<f64>, v: &Vec<f64>| [u.as_slice(), v.as_slice()].concat();
            let pair = ClimatePair::new(
                cat(&a.p, &b.p),
                cat(&a.tx, &b.tx),
                cat(&a.tn, &b.tn),
                &label,
            )
            .map_err(|e| ScenarioError(e.0))?;
            scen.push(Scenario {
                y,
                label,
                split: split.name(),
                valid: true,
                reason: String::new(),
                pair: Some(pair),
            });
        } else {
            let mut bad: Vec<(String, usize)> = Vec::new();
            for (yy, ag_y) in [(y, a), (y + 1, b)] {
                for (k, n) in VARIABLES.iter().zip(ag_y.missing) {
                    if n > 0 {
                        bad.push((format!("{yy}:{k}"), n));
                    }
                }
            }
            scen.push(Scenario {
                y,
                label,
                split: split.name(),
                valid: false,
                reason: format!("dados ausentes {}", py_dict_repr(&bad)),
                pair: None,
            });
        }
    }
    Ok(ScenarioSet {
        split,
        scenarios: scen,
        ag_years: ag,
        n_expected: pair_ys.len(),
    })
}

/// (clima de referência de e3, s_barra) só com anos dos pares válidos da construção.
///
/// s_barra = média por dia dos anos (soma sequencial na ordem cronológica, depois `/ n`, como
/// `numpy.mean(axis=0)`), repetida nos dois anos do par.
pub fn construction_climatology(sset: &ScenarioSet) -> Res<(ConstructionClimatology, ClimatePair)> {
    if sset.split != Split::Construction {
        return err("a climatologia só pode vir da construção");
    }
    let mut years: BTreeSet<i32> = BTreeSet::new();
    for s in sset.valid() {
        years.insert(s.y);
        years.insert(s.y + 1);
    }
    if years.is_empty() {
        return err("sem anos válidos na construção");
    }
    let ys: Vec<i32> = years.into_iter().collect();
    let n = ys.len() as f64;
    let mean = |pick: fn(&AgYear) -> &Vec<f64>| -> Vec<f64> {
        let mut acc = vec![0.0f64; DAYS_PER_YEAR as usize];
        for y in &ys {
            for (a, v) in acc.iter_mut().zip(pick(&sset.ag_years[y])) {
                *a += v;
            }
        }
        let m: Vec<f64> = acc.iter().map(|a| a / n).collect();
        [m.clone(), m].concat()
    };
    let (sp, stx, stn) = (mean(|a| &a.p), mean(|a| &a.tx), mean(|a| &a.tn));
    if ![&sp, &stx, &stn]
        .iter()
        .all(|a| a.iter().all(|v| v.is_finite()))
    {
        return err("algum dia de s_barra ficou sem valor; execução abortada (sem imputação)");
    }
    let precip: Vec<Vec<f64>> = ys.iter().map(|y| sset.ag_years[y].p.clone()).collect();
    let reference =
        ConstructionClimatology::new(&precip, ys.clone()).map_err(|e| ScenarioError(e.0))?;
    let sbar = ClimatePair::new(sp, stx, stn, "s_barra").map_err(|e| ScenarioError(e.0))?;
    debug_assert_eq!(sbar.p.len(), HORIZON_DAYS as usize);
    Ok((reference, sbar))
}
