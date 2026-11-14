// AI-assisted (OpenBot agent)
//! Pipeline de otimização: X -> cenários de construção -> P0/P1/P1b/P2 -> Pareto -> grade de pesos.
//!
//! Porte fiel de `run`, `RunResult`, `compute_config_hash` e correlatos de
//! `src/fieldshift/optimization.py` (commit `16cc391`). Enumeração exaustiva; sem solver.
//!
//! Protocolo v1.1: o teste NUNCA entra na escolha. A construção é fatiada do `ClimateFrame` antes de
//! qualquer cálculo; o teste só gera a contagem de validade (portão de dados insuficientes).

use std::collections::BTreeMap;

use crate::compat::{fsum, py_round, sha256_hex, to_canonical};
use serde_json::{json, Map, Number, Value};

use crate::catalog::{
    describe_plan, diagnose_infeasible, enumerate_plans, fallow_summary, plan_index, resolve_spec,
    summarize_rejections, Catalog, CatalogError, Plan, PlanId, PlanSpec, Soil, ZARC_RISK_LIMIT_PCT,
};
use crate::indicators::{loss_components, ClimatePair, IndicatorError, IndicatorParams};
use crate::pareto::{
    objective_vectors, pareto_front, select_alternatives, Alternative, K_ALTERNATIVES, MIN_HAMMING,
};
use crate::scenarios::{
    build_scenarios, construction_climatology, ClimateFrame, ScenarioError, Split,
    CONSTRUCTION_FIRST_Y, CONSTRUCTION_LAST_PAIR_Y, TEST_FIRST_Y, TEST_LAST_PAIR_Y,
};
use crate::selection::{
    loss_matrix, loss_vector, regret_matrix, row_max, select_conv, select_exp, select_mmr,
    weight_grid, Matrix, Selection, Tensor3, DEFAULT_WEIGHTS, ROUND,
};

pub const PROTOCOL_VERSION: &str = "1.1";
pub const MIN_VALID_CONSTRUCTION: usize = 20;
pub const MIN_VALID_TEST: usize = 10;
pub const POLICIES: [&str; 4] = ["P0", "P1", "P1b", "P2"];

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[error("{0}")]
pub struct OptimizationError(pub String);

impl From<CatalogError> for OptimizationError {
    fn from(e: CatalogError) -> Self {
        Self(e.0)
    }
}
impl From<IndicatorError> for OptimizationError {
    fn from(e: IndicatorError) -> Self {
        Self(e.0)
    }
}
impl From<ScenarioError> for OptimizationError {
    fn from(e: ScenarioError) -> Self {
        Self(e.0)
    }
}

type Res<T> = Result<T, OptimizationError>;

fn err<T>(msg: impl Into<String>) -> Res<T> {
    Err(OptimizationError(msg.into()))
}

/// Opções de `run` (padrões = os do Python).
#[derive(Debug, Clone)]
pub struct RunOptions {
    pub p0: Option<PlanSpec>,
    pub weights: Option<[f64; 3]>,
    pub k_alternatives: usize,
    pub min_hamming: usize,
}

impl Default for RunOptions {
    fn default() -> Self {
        Self {
            p0: None,
            weights: None,
            k_alternatives: K_ALTERNATIVES,
            min_hamming: MIN_HAMMING,
        }
    }
}

// --------------------------------------------------------------------------- JSON auxiliar
/// `round(v, 12)` do Python (`_r`) como número JSON float.
fn fl(x: f64) -> Value {
    match Number::from_f64(py_round(x, 12)) {
        Some(n) => Value::Number(n),
        None => Value::Null,
    }
}

/// Número JSON float sem arredondar (hash de configuração).
fn raw_fl(x: f64) -> Value {
    Number::from_f64(x).map_or(Value::Null, Value::Number)
}

fn fl_list(v: &[f64]) -> Value {
    Value::Array(v.iter().map(|x| fl(*x)).collect())
}

fn id_value(id: &PlanId) -> Value {
    json!(id.to_vec())
}

fn canonical(v: &Value) -> String {
    to_canonical(v)
}

fn counts_value(m: &BTreeMap<String, i64>) -> Value {
    Value::Object(m.iter().map(|(k, v)| (k.clone(), json!(v))).collect())
}

// --------------------------------------------------------------------------- resultado
/// Resultado serializável (espelha o `dict` do Python). `meta` (tempos, versões) fica fora do hash.
#[derive(Debug, Clone, PartialEq)]
pub struct RunResult {
    pub body: Map<String, Value>,
    pub meta: Map<String, Value>,
}

impl RunResult {
    pub fn status(&self) -> &str {
        self.body
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("")
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.body.get(key)
    }

    /// ID do plano de uma política (`None` se a política não tem plano, ex.: P0 inviável).
    pub fn policy_plan_id(&self, name: &str) -> Option<PlanId> {
        let ids = self
            .body
            .get("policies")?
            .get(name)?
            .get("plan")?
            .get("id")?
            .as_array()?;
        let v: Vec<i32> = ids
            .iter()
            .filter_map(|x| x.as_i64().map(|i| i as i32))
            .collect();
        v.try_into().ok()
    }

    /// Dicionário (opcionalmente com `meta`).
    pub fn to_value(&self, include_meta: bool) -> Value {
        let mut m = self.body.clone();
        if include_meta {
            m.insert("meta".into(), Value::Object(self.meta.clone()));
        }
        Value::Object(m)
    }

    /// SHA-256 do JSON canônico sem `meta` (sem horários): duas execuções => mesmo hash.
    pub fn result_hash(&self) -> String {
        sha256_hex(canonical(&self.to_value(false)).as_bytes())
    }
}

fn base_body(status: &str, message: &str) -> Map<String, Value> {
    let mut m = Map::new();
    m.insert("status".into(), json!(status));
    m.insert("message".into(), json!(message));
    m.insert("snapshot_id".into(), json!(""));
    m.insert("config_hash".into(), json!(""));
    m.insert("protocol_version".into(), json!(PROTOCOL_VERSION));
    m.insert("soil".into(), json!({}));
    m.insert("risk_limit_pct".into(), json!(ZARC_RISK_LIMIT_PCT));
    m.insert("weights".into(), fl_list(&DEFAULT_WEIGHTS));
    m.insert(
        "weight_grid".into(),
        Value::Array(weight_grid().iter().map(|w| fl_list(w)).collect()),
    );
    m.insert("n_plans".into(), json!(0));
    m.insert("x_hash".into(), json!(""));
    m.insert("fallow".into(), json!({}));
    m.insert("rejections_summary".into(), json!({}));
    m.insert("rejections".into(), json!([]));
    m.insert("diagnosis".into(), json!({}));
    m.insert("scenarios".into(), json!({}));
    m.insert("climatology".into(), json!({}));
    m.insert("policies".into(), json!({}));
    m.insert("objectives".into(), json!([]));
    m.insert("pareto".into(), json!({}));
    m.insert("alternatives".into(), json!([]));
    m.insert("sensitivity".into(), json!([]));
    m.insert("fallow_usage".into(), json!({}));
    m
}

fn x_hash(plans: &[Plan]) -> String {
    let rows: Vec<String> = plans
        .iter()
        .map(|p| {
            let cells: Vec<String> = p.id.iter().map(|x| x.to_string()).collect();
            format!("[{}]", cells.join(", "))
        })
        .collect();
    sha256_hex(format!("[{}]", rows.join(", ")).as_bytes())
}

/// SHA-256 da configuração congelada (arquivos, soil, pesos, grade, P0, protocolo).
pub fn compute_config_hash(
    catalog: &Catalog,
    soil: &Soil,
    params: &IndicatorParams,
    weights: [f64; 3],
    p0: &PlanSpec,
    k: usize,
    delta: usize,
) -> String {
    let files: Map<String, Value> = catalog
        .files_sha256
        .iter()
        .map(|(k, v)| (k.clone(), json!(v)))
        .collect();
    let cfg = json!({
        "files": Value::Object(files),
        "indicators": params.sha256,
        "soil": soil.describe(),
        "risk_limit_pct": ZARC_RISK_LIMIT_PCT,
        "weights": weights.iter().map(|x| raw_fl(*x)).collect::<Vec<_>>(),
        "grid": weight_grid().iter()
            .map(|w| Value::Array(w.iter().map(|x| raw_fl(*x)).collect()))
            .collect::<Vec<_>>(),
        "p0": p0.to_value(),
        "protocol": {
            "version": PROTOCOL_VERSION,
            "construction": [CONSTRUCTION_FIRST_Y, CONSTRUCTION_LAST_PAIR_Y],
            "test": [TEST_FIRST_Y, TEST_LAST_PAIR_Y],
            "min_valid": [MIN_VALID_CONSTRUCTION, MIN_VALID_TEST],
            "round": ROUND,
            "alternatives": [k, delta],
        },
    });
    sha256_hex(canonical(&cfg).as_bytes())
}

/// `(n_planos, n_pares, 3)` com g1, g2, g3.
pub fn components_tensor(
    plans: &[Plan],
    pairs: &[&ClimatePair],
    params: &IndicatorParams,
) -> Res<Tensor3> {
    let mut out = Tensor3::zeros(plans.len(), pairs.len());
    for (j, pair) in pairs.iter().enumerate() {
        for (i, plan) in plans.iter().enumerate() {
            out.set(i, j, loss_components(plan, pair, params)?);
        }
    }
    Ok(out)
}

fn policy_entry(
    catalog: &Catalog,
    plans: &[Plan],
    i: usize,
    l: &Matrix,
    lbar: &[f64],
    sel: Option<&Selection>,
) -> Value {
    let r = regret_matrix(l);
    let n = l.cols as f64;
    let mut e = json!({
        "status": "OK",
        "plan": describe_plan(catalog, &plans[i]),
        "metrics_construction": {
            "L_sbar": fl(lbar[i]),
            "mean_L": fl(fsum(l.row(i)) / n),
            "max_regret": fl(r.row(i).iter().copied().fold(f64::NEG_INFINITY, f64::max)),
            "mean_regret": fl(fsum(r.row(i)) / n),
        },
        "fallow_windows": plans[i].fallow_windows(),
    });
    if let (Some(s), Some(obj)) = (sel, e.as_object_mut()) {
        obj.insert(
            "ties".into(),
            json!({"tied_after_level": s.tied_after_level, "id_tiebreak": s.id_tiebreak}),
        );
    }
    e
}

fn alt_value(a: &Alternative) -> Value {
    let mut m = Map::new();
    m.insert("plan_id".into(), id_value(&a.plan_id));
    m.insert("labels".into(), json!(a.labels));
    m.insert("reason".into(), json!(a.reason));
    m.insert("on_front".into(), json!(a.on_front));
    m.insert("hamming_relaxed".into(), json!(a.hamming_relaxed));
    if let Some(p) = &a.policy_plan_ids {
        let mm: Map<String, Value> = p.iter().map(|(k, v)| (k.clone(), id_value(v))).collect();
        m.insert("policy_plan_ids".into(), Value::Object(mm));
    }
    m.insert(
        "min_hamming_to_others".into(),
        json!(a.min_hamming_to_others),
    );
    Value::Object(m)
}

fn f64_bytes(parts: &[&[f64]]) -> Vec<u8> {
    parts
        .iter()
        .flat_map(|p| p.iter().flat_map(|v| v.to_le_bytes()))
        .collect()
}

/// Otimização completa. Devolve `RunResult` com status `OK`, `INVIAVEL` ou `DADOS_INSUFICIENTES`.
pub fn run(
    climate: &ClimateFrame,
    catalog: &Catalog,
    soil: &Soil,
    params: &IndicatorParams,
    snapshot_id: &str,
    opts: &RunOptions,
) -> Res<RunResult> {
    let w = opts.weights.unwrap_or(params.default_weights);
    if w.iter().any(|x| *x < 0.0 || !x.is_finite()) || (w.iter().sum::<f64>() - 1.0).abs() > 1e-12 {
        return err("weights deve ter 3 valores >= 0 somando 1");
    }
    let p0 = opts
        .p0
        .clone()
        .unwrap_or_else(|| catalog.default_p0.clone());
    let cfg_hash = compute_config_hash(
        catalog,
        soil,
        params,
        w,
        &p0,
        opts.k_alternatives,
        opts.min_hamming,
    );
    let common = |m: &mut Map<String, Value>| {
        m.insert("snapshot_id".into(), json!(snapshot_id));
        m.insert("config_hash".into(), json!(cfg_hash));
        m.insert("soil".into(), soil.describe());
        m.insert("weights".into(), fl_list(&w));
    };

    let (plans, rejections) =
        enumerate_plans(catalog, soil, f64::from(ZARC_RISK_LIMIT_PCT) / 100.0)?;
    let rej_values: Vec<Value> = rejections.iter().map(|r| r.to_value()).collect();
    if plans.is_empty() {
        let diag = diagnose_infeasible(&rejections);
        let msg = diag["message"].as_str().unwrap_or("").to_string();
        let mut b = base_body("INVIAVEL", &msg);
        b.insert("diagnosis".into(), diag);
        b.insert("rejections".into(), Value::Array(rej_values));
        b.insert(
            "rejections_summary".into(),
            counts_value(&summarize_rejections(&rejections)),
        );
        common(&mut b);
        return Ok(RunResult {
            body: b,
            meta: Map::new(),
        });
    }

    let cset = build_scenarios(climate, Split::Construction)?;
    // Só contagem de validade (portão); nunca entra na escolha.
    let tset = build_scenarios(climate, Split::Test)?;
    let scen_info = json!({"construction": cset.summary(), "test": tset.summary()});
    let (n_c, n_t) = (cset.n_valid(), tset.n_valid());
    if n_c < MIN_VALID_CONSTRUCTION || n_t < MIN_VALID_TEST {
        let msg = format!(
            "dados insuficientes: {n_c}/{} pares válidos na construção (mínimo {MIN_VALID_CONSTRUCTION}) e {n_t}/{} no teste (mínimo {MIN_VALID_TEST})",
            cset.n_expected, tset.n_expected
        );
        let mut b = base_body("DADOS_INSUFICIENTES", &msg);
        b.insert("scenarios".into(), scen_info);
        b.insert("n_plans".into(), json!(plans.len()));
        b.insert("x_hash".into(), json!(x_hash(&plans)));
        common(&mut b);
        return Ok(RunResult {
            body: b,
            meta: Map::new(),
        });
    }

    let (reference, sbar) = construction_climatology(&cset)?;
    let ref_labels = reference.labels.clone();
    let ref_n = reference.n_years();
    let ref_sha = reference.sha256();
    let prm = params.clone_without_reference().with_reference(reference);
    let valid = cset.valid();
    let pairs: Vec<&ClimatePair> = valid.iter().filter_map(|s| s.pair.as_ref()).collect();
    let comp = components_tensor(&plans, &pairs, &prm)?;
    let comp_bar: Vec<[f64; 3]> = plans
        .iter()
        .map(|p| loss_components(p, &sbar, &prm))
        .collect::<Result<_, _>>()?;
    let ids: Vec<PlanId> = plans.iter().map(|p| p.id).collect();

    let choose = |ww: [f64; 3]| -> (Selection, Selection, Selection, Matrix, Vec<f64>) {
        let l = loss_matrix(&comp, ww);
        let lb = loss_vector(&comp_bar, ww);
        (
            select_conv(&l, &lb, &ids),
            select_exp(&l, &ids),
            select_mmr(&l, &ids),
            l,
            lb,
        )
    };

    let (s1, s1b, s2, l, lbar) = choose(w);
    let mut policies = Map::new();
    let mut picks: Vec<(String, usize)> = Vec::new();
    let (p0_plan, p0_reasons) = resolve_spec(catalog, soil, &p0, &plans, 0.20)?;
    match p0_plan {
        None => {
            policies.insert(
                "P0".into(),
                json!({
                    "status": "INVIAVEL_SOB_ZARC",
                    "spec": p0.to_value(),
                    "reasons": p0_reasons,
                    "message": "plano atual inviável sob ZARC <= 20%; sai da comparação de regret",
                }),
            );
        }
        Some(pl) => {
            let i0 = plan_index(&plans, &pl.id).unwrap_or(0);
            let mut e = policy_entry(catalog, &plans, i0, &l, &lbar, None);
            if let Some(o) = e.as_object_mut() {
                o.insert("spec".into(), p0.to_value());
            }
            policies.insert("P0".into(), e);
            picks.push(("P0".into(), i0));
        }
    }
    for (name, sel) in [("P1", &s1), ("P1b", &s1b), ("P2", &s2)] {
        policies.insert(
            name.into(),
            policy_entry(catalog, &plans, sel.index, &l, &lbar, Some(sel)),
        );
        picks.push((name.into(), sel.index));
    }

    let f = objective_vectors(&comp);
    let (front, equivalents) = pareto_front(&f, &ids);
    let alts = select_alternatives(
        &plans,
        &f,
        &front,
        &picks,
        w,
        opts.k_alternatives,
        opts.min_hamming,
    );

    let mut sens = Vec::new();
    let mut use_fb: BTreeMap<&str, i64> = BTreeMap::from([("P1", 0), ("P1b", 0), ("P2", 0)]);
    for ww in weight_grid() {
        let (a1, a1b, a2, _, _) = choose(ww);
        sens.push(json!({
            "weights": fl_list(&ww),
            "P1": id_value(&ids[a1.index]),
            "P1b": id_value(&ids[a1b.index]),
            "P2": id_value(&ids[a2.index]),
            "same_plan": a1.index == a1b.index && a1b.index == a2.index,
            "id_tiebreak": {"P1": a1.id_tiebreak, "P1b": a1b.id_tiebreak, "P2": a2.id_tiebreak},
        }));
        for (name, a) in [("P1", &a1), ("P1b", &a1b), ("P2", &a2)] {
            if !plans[a.index].fallow_windows().is_empty() {
                *use_fb.entry(name).or_insert(0) += 1;
            }
        }
    }

    let fb = fallow_summary(&plans);
    let mut w_star = Map::new();
    for n in POLICIES {
        if let Some(p) = policies.get(n) {
            if p["status"] == "OK" {
                w_star.insert(n.into(), p["fallow_windows"].clone());
            }
        }
    }
    let fallow_usage = json!({
        "x_plans_with_fallow": fb.get("com_pousio").copied().unwrap_or(0),
        "policies_w_star": Value::Object(w_star),
        "grid_selections_with_fallow": use_fb,
        "grid_size": weight_grid().len(),
        "front_with_fallow": front.iter().filter(|&&i| !plans[i].fallow_windows().is_empty()).count(),
        "front_size": front.len(),
        "alternatives_with_fallow": alts.iter()
            .filter(|a| !plans[a.plan_index].fallow_windows().is_empty()).count(),
    });
    let objectives: Vec<Value> = (0..plans.len())
        .map(|i| {
            let mut row = vec![id_value(&plans[i].id)];
            row.extend(f[i].iter().map(|x| fl(*x)));
            Value::Array(row)
        })
        .collect();
    let eq_map: Map<String, Value> = equivalents
        .iter()
        .filter(|(_, e)| !e.is_empty())
        .map(|(r, e)| {
            let key = ids[*r]
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join(",");
            (
                key,
                Value::Array(e.iter().map(|&i| id_value(&ids[i])).collect()),
            )
        })
        .collect();
    let pareto = json!({
        "front_ids": front.iter().map(|&i| id_value(&ids[i])).collect::<Vec<_>>(),
        "equivalents": Value::Object(eq_map),
        "n_front": front.len(),
        "objectives": ["mean_g1", "max_g1", "mean_g2", "mean_g3"],
    });
    let climatology = json!({
        "n_years": ref_n,
        "years": ref_labels,
        "precip_sha256": ref_sha,
        "sbar_sha256": sha256_hex(&f64_bytes(&[&sbar.p, &sbar.tx, &sbar.tn])),
    });

    let mut b = base_body("OK", "ok");
    b.insert("n_plans".into(), json!(plans.len()));
    b.insert("x_hash".into(), json!(x_hash(&plans)));
    b.insert("fallow".into(), counts_value(&fb));
    b.insert(
        "rejections_summary".into(),
        counts_value(&summarize_rejections(&rejections)),
    );
    b.insert("rejections".into(), Value::Array(rej_values));
    b.insert("scenarios".into(), scen_info);
    b.insert("climatology".into(), climatology);
    b.insert("policies".into(), Value::Object(policies));
    b.insert("objectives".into(), Value::Array(objectives));
    b.insert("pareto".into(), pareto);
    b.insert(
        "alternatives".into(),
        Value::Array(alts.iter().map(alt_value).collect()),
    );
    b.insert("sensitivity".into(), Value::Array(sens));
    b.insert("fallow_usage".into(), fallow_usage);
    common(&mut b);
    Ok(RunResult {
        body: b,
        meta: Map::new(),
    })
}

/// Média e máximo de regret de um plano (auxiliar para relatórios e testes).
pub fn regret_stats(l: &Matrix, i: usize) -> (f64, f64) {
    let r = regret_matrix(l);
    (fsum(r.row(i)) / l.cols as f64, row_max(&r)[i])
}
