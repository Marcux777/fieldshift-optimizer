// AI-assisted (OpenBot agent)
//! Product I/O boundary. No reports, fixtures, Python or network are read here.
use anyhow::Result;
use fieldshift_eval::{
    adapter::{plan_checker, plan_infos, IndicatorLossModel},
    evaluation::{
        evaluate_plans, mean_climate, split_scenarios, ClimateDaily, EvalInput, EvalProtocol,
    },
};
use fieldshift_io::snapshot;
use fieldshift_model::{
    catalog::{self, Catalog, CatalogSources, PlanSpec, Soil},
    indicators::IndicatorParams,
    optimization::{self, RunOptions, RunResult},
    scenarios::ClimateFrame,
};
use fieldshift_numeric::canon::sha256_hex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    sync::atomic::{AtomicUsize, Ordering},
};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CurrentPlan {
    pub j1: String,
    pub dec1: i32,
    pub j2: String,
    pub j3: String,
    pub dec3: i32,
    pub j4: String,
}
impl CurrentPlan {
    fn spec(&self) -> PlanSpec {
        PlanSpec {
            j1: self.j1.clone(),
            dec1: self.dec1,
            j2: self.j2.clone(),
            j3: self.j3.clone(),
            dec3: self.dec3,
            j4: self.j4.clone(),
        }
    }
}
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Inputs {
    pub ad_class: String,
    pub texture: String,
    pub weights: [f64; 3],
    pub p0: Option<CurrentPlan>,
}
impl Default for Inputs {
    fn default() -> Self {
        Self {
            ad_class: "AD4".into(),
            texture: "argilosa".into(),
            weights: [0.5, 0.25, 0.25],
            p0: None,
        }
    }
}
impl Inputs {
    pub fn validate(&self) -> Result<(), String> {
        if !["AD1", "AD2", "AD3", "AD4", "AD5", "AD6"].contains(&self.ad_class.as_str()) {
            return Err("ad_class must be AD1–AD6".into());
        }
        if !["arenosa", "media", "argilosa"].contains(&self.texture.as_str()) {
            return Err("texture must be arenosa, media or argilosa".into());
        }
        if self.weights.iter().any(|w| !w.is_finite() || *w < 0.0)
            || (self.weights.iter().sum::<f64>() - 1.0).abs() > 1e-12
        {
            return Err("weights must be finite, non-negative and sum to 1".into());
        }
        if let Some(p) = &self.p0 {
            if !(1..=36).contains(&p.dec1) || !(1..=36).contains(&p.dec3) {
                return Err("P0 sowing periods must be between 1 and 36".into());
            }
        }
        Ok(())
    }
}

pub struct Engine {
    pub catalog: Catalog,
    pub manifest: Value,
    pub quality: Value,
    soil_toml: Vec<u8>,
    params: IndicatorParams,
    climate: ClimateFrame,
    daily: ClimateDaily,
    pub protocol_sha256: String,
    pub default_config_hash: String,
    run_calls: AtomicUsize,
    evaluate_calls: AtomicUsize,
    plans_calls: AtomicUsize,
}
impl Engine {
    pub fn load(root: &Path) -> Result<Self> {
        let config = root.join("config");
        let cat_bytes = fs::read(config.join("catalog.toml"))?;
        let soil_toml = fs::read(config.join("soil.toml"))?;
        // The frozen catalog verifies the ZARC bytes against its declared SHA-256.
        let zarc = fs::read(config.join("zarc/zarc-tabua-rio-verde-5218805-2026-2027.csv"))?;
        let catalog = catalog::load_catalog(&CatalogSources {
            catalog_toml: &cat_bytes,
            soil_toml: &soil_toml,
            zarc_csv: &zarc,
        })?;
        let params =
            IndicatorParams::from_toml(&fs::read(config.join("indicators.toml"))?, &catalog)?;
        let snap = snapshot::latest_snapshot(&root.join("data/snapshots"))?;
        let climate_data = snapshot::load_climate(&snap)?;
        let quality = climate_data.quality_report();
        let manifest = snapshot::read_manifest(&snap)?;
        let climate = ClimateFrame::from_rows(
            climate_data
                .dates
                .iter()
                .zip(&climate_data.values)
                .map(|(d, v)| (d.year, d.month, d.day, *v)),
        )?;
        let daily = ClimateDaily {
            dates: climate_data
                .dates
                .iter()
                .map(|d| (d.year, u32::from(d.month), u32::from(d.day)))
                .collect(),
            prectotcorr: climate_data.values.iter().map(|v| v[0]).collect(),
            t2m: climate_data.values.iter().map(|v| v[1]).collect(),
            t2m_max: climate_data.values.iter().map(|v| v[2]).collect(),
            t2m_min: climate_data.values.iter().map(|v| v[3]).collect(),
        };
        let protocol_sha256 = sha256_hex(&fs::read(root.join("docs/EVALUATION_PROTOCOL.md"))?);
        anyhow::ensure!(
            protocol_sha256 == fieldshift_eval::evaluation::PROTOCOL_SHA256,
            "Frozen protocol hash differs; refusing to calculate"
        );
        let default_soil = catalog::load_soil(&soil_toml, Some("AD4"), Some("argilosa"))?;
        let default_config_hash = optimization::compute_config_hash(
            &catalog,
            &default_soil,
            &params,
            fieldshift_model::selection::DEFAULT_WEIGHTS,
            &catalog.default_p0,
            fieldshift_model::pareto::K_ALTERNATIVES,
            fieldshift_model::pareto::MIN_HAMMING,
        );
        Ok(Self {
            catalog,
            manifest,
            quality,
            soil_toml,
            params,
            climate,
            daily,
            protocol_sha256,
            default_config_hash,
            run_calls: AtomicUsize::new(0),
            evaluate_calls: AtomicUsize::new(0),
            plans_calls: AtomicUsize::new(0),
        })
    }
    pub fn is_default_inputs(&self, i: &Inputs) -> bool {
        i.ad_class == "AD4"
            && i.texture == "argilosa"
            && i.weights == fieldshift_model::selection::DEFAULT_WEIGHTS
            && match &i.p0 {
                None => true,
                Some(p) => p.spec() == self.catalog.default_p0,
            }
    }
    pub fn call_counts(&self) -> (usize, usize, usize) {
        (
            self.run_calls.load(Ordering::SeqCst),
            self.evaluate_calls.load(Ordering::SeqCst),
            self.plans_calls.load(Ordering::SeqCst),
        )
    }
    #[doc(hidden)]
    pub fn with_truncated_climate(mut self, rows: usize) -> Result<Self> {
        let len = rows.min(self.daily.dates.len());
        self.daily.dates.truncate(len);
        self.daily.prectotcorr.truncate(len);
        self.daily.t2m.truncate(len);
        self.daily.t2m_max.truncate(len);
        self.daily.t2m_min.truncate(len);
        self.climate = ClimateFrame::from_rows((0..len).map(|idx| {
            let (y, m, d) = self.daily.dates[idx];
            (
                y,
                m as u8,
                d as u8,
                [
                    self.daily.prectotcorr[idx],
                    self.daily.t2m[idx],
                    self.daily.t2m_max[idx],
                    self.daily.t2m_min[idx],
                ],
            )
        }))?;
        Ok(self)
    }
    #[doc(hidden)]
    pub fn with_infeasible_zarc(mut self) -> Self {
        for risks in self.catalog.zarc.values_mut() {
            *risks = [40; 36];
        }
        self
    }
    fn soil(&self, i: &Inputs) -> Result<Soil, String> {
        i.validate()?;
        catalog::load_soil(&self.soil_toml, Some(&i.ad_class), Some(&i.texture))
            .map_err(|e| e.to_string())
    }
    pub fn run(&self, i: &Inputs) -> Result<RunResult, String> {
        let soil = self.soil(i)?;
        if let Some(p) = &i.p0 {
            for k in [&p.j1, &p.j2, &p.j3, &p.j4] {
                self.catalog.index_of(k).map_err(|e| e.to_string())?;
            }
        }
        self.run_calls.fetch_add(1, Ordering::SeqCst);
        let snapshot_id = self.manifest["snapshot_id"]
            .as_str()
            .ok_or("Snapshot ID missing")?;
        optimization::run(
            &self.climate,
            &self.catalog,
            &soil,
            &self.params,
            snapshot_id,
            &RunOptions {
                p0: i.p0.as_ref().map(CurrentPlan::spec),
                weights: Some(i.weights),
                ..Default::default()
            },
        )
        .map_err(|e| e.to_string())
    }
    pub fn run_value(&self, i: &Inputs) -> Result<Value, String> {
        let rr = self.run(i)?;
        let hash = rr.result_hash();
        let mut v = rr.to_value(true);
        v["result_hash"] = json!(hash);
        Ok(v)
    }
    pub fn evaluate(&self, i: &Inputs) -> Result<Value, String> {
        let rr = self.run(i)?;
        let soil = self.soil(i)?;
        self.evaluate_calls.fetch_add(1, Ordering::SeqCst);
        let (plans, _) =
            catalog::enumerate_plans(&self.catalog, &soil, 0.20).map_err(|e| e.to_string())?;
        let infos = plan_infos(&plans);
        let ids = BTreeMap::from_iter(["P0", "P1", "P1b", "P2"].map(|key| {
            (
                key.to_string(),
                rr.policy_plan_id(key)
                    .map(|id| id.into_iter().map(i64::from).collect()),
            )
        }));
        let check = plan_checker(&self.catalog, &soil, &plans);
        let proto = EvalProtocol::default();
        let mut model = IndicatorLossModel::new(&plans, &self.params);
        let body = rr.to_value(false);
        let input = EvalInput {
            plans: &infos,
            policy_ids: ids,
            p0_status: if body["policies"]["P0"]["status"] == "INVIAVEL_SOB_ZARC" {
                Some("inviavel_sob_ZARC_20".into())
            } else {
                None
            },
            climate: &self.daily,
            weights: Some(i.weights),
            check_plan: Some(&check),
            run_grid: true,
            optimizer_params_hash: body["climatology"]["precip_sha256"]
                .as_str()
                .map(String::from),
        };
        let mut report = evaluate_plans(&input, &mut model, &proto).map_err(|e| e.to_string())?;
        if rr.status() == "OK" {
            let ids = infos
                .iter()
                .map(|p| {
                    format!(
                        "[{}]",
                        p.id.iter()
                            .map(i64::to_string)
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            report.audit_set(
                "x_hash_match",
                json!(sha256_hex(format!("[{ids}]").as_bytes()) == body["x_hash"]),
            );
            let split = split_scenarios(&self.daily, &proto).map_err(|e| e.to_string())?;
            let sb = mean_climate(&split.construction_years.iter().collect::<Vec<_>>())
                .map_err(|e| e.to_string())?;
            report.audit_set(
                "sbar_hash_match",
                json!(sb.content_key() == body["climatology"]["sbar_sha256"]),
            );
            report.audit_set("optimizer_status", json!(rr.status()));
        }
        let hash = report.result_hash();
        let mut v = report.to_value();
        v["result_hash"] = json!(hash);
        Ok(v)
    }
    pub fn plans(&self, i: &Inputs) -> Result<Value, String> {
        let soil = self.soil(i)?;
        self.plans_calls.fetch_add(1, Ordering::SeqCst);
        let (plans, rejections) =
            catalog::enumerate_plans(&self.catalog, &soil, 0.20).map_err(|e| e.to_string())?;
        Ok(json!({"status":if plans.is_empty(){"INVIAVEL"}else{"OK"},
            "n_plans":plans.len(),"plans":plans.iter().map(|p|catalog::describe_plan(&self.catalog,p)).collect::<Vec<_>>(),
            "rejections":rejections.iter().map(|r|r.to_value()).collect::<Vec<_>>(),
            "diagnosis":catalog::diagnose_infeasible(&rejections)}))
    }
    pub fn catalog_value(&self) -> Value {
        json!({"crops":self.catalog.crops.iter().map(|c|json!({"key":c.key,"label":c.label,
            "windows":c.windows,"cycle_days":c.cycle_days})).collect::<Vec<_>>(),
            "windows":self.catalog.windows.iter().map(|(name,w)|json!({"name":name,
                "crops":w.crops,"decendios":w.decendios})).collect::<Vec<_>>(),
            "soils":["AD1","AD2","AD3","AD4","AD5","AD6"],
            "textures":["arenosa","media","argilosa"],"default_p0":self.catalog.default_p0.to_value(),
            "weight_grid":fieldshift_model::selection::weight_grid(),"risk_limit":0.20})
    }
}
