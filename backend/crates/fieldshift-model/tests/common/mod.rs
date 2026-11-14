// AI-assisted (OpenBot agent)
//! Utilitários dos testes diferenciais: leitura de fixtures (oráculo Python) e de `config/`.
//! As fixtures são lidas SÓ por testes; o produto nunca as usa.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

use fieldshift_model::catalog::{load_catalog, load_soil, Catalog, CatalogSources, Soil};
use fieldshift_model::indicators::IndicatorParams;
use fieldshift_model::scenarios::ClimateFrame;
use serde_json::Value;

pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

pub fn fixture(name: &str) -> Value {
    let p = repo_root().join("fixtures/reference").join(name);
    let bytes = std::fs::read(&p).unwrap_or_else(|e| panic!("fixture {}: {e}", p.display()));
    serde_json::from_slice(&bytes).unwrap_or_else(|e| panic!("fixture {name}: {e}"))
}

pub struct Config {
    pub catalog_toml: Vec<u8>,
    pub soil_toml: Vec<u8>,
    pub zarc_csv: Vec<u8>,
    pub indicators_toml: Vec<u8>,
}

pub fn config() -> Config {
    let dir = repo_root().join("config");
    let rd = |p: &str| std::fs::read(dir.join(p)).unwrap_or_else(|e| panic!("config/{p}: {e}"));
    Config {
        catalog_toml: rd("catalog.toml"),
        soil_toml: rd("soil.toml"),
        zarc_csv: rd("zarc/zarc-tabua-rio-verde-5218805-2026-2027.csv"),
        indicators_toml: rd("indicators.toml"),
    }
}

pub fn catalog(c: &Config) -> Catalog {
    load_catalog(&CatalogSources {
        catalog_toml: &c.catalog_toml,
        soil_toml: &c.soil_toml,
        zarc_csv: &c.zarc_csv,
    })
    .expect("catálogo")
}

pub fn soil(c: &Config, ad: Option<&str>, tx: Option<&str>) -> Soil {
    load_soil(&c.soil_toml, ad, tx).expect("solo")
}

pub fn params(c: &Config, cat: &Catalog) -> IndicatorParams {
    IndicatorParams::from_toml(&c.indicators_toml, cat).expect("indicadores")
}

fn f(v: &Value) -> f64 {
    v.as_f64().unwrap_or(f64::NAN)
}

/// Clima das fixtures (mesmo snapshot do baseline; datas ISO).
pub fn climate() -> ClimateFrame {
    let c = fixture("climate_daily.json");
    let dates = c["dates"].as_array().expect("dates");
    let cols: Vec<&Vec<Value>> = ["PRECTOTCORR", "T2M", "T2M_MAX", "T2M_MIN"]
        .iter()
        .map(|k| c[*k].as_array().expect("coluna"))
        .collect();
    let mut fr = ClimateFrame::new();
    for (i, d) in dates.iter().enumerate() {
        let s = d.as_str().expect("data");
        let (y, m, dd) = (
            s[0..4].parse().unwrap(),
            s[5..7].parse().unwrap(),
            s[8..10].parse().unwrap(),
        );
        fr.insert(
            y,
            m,
            dd,
            [
                f(&cols[0][i]),
                f(&cols[1][i]),
                f(&cols[2][i]),
                f(&cols[3][i]),
            ],
        )
        .expect("data única");
    }
    fr
}

pub fn ids_of(v: &Value) -> Vec<i32> {
    v.as_array()
        .expect("id")
        .iter()
        .map(|x| x.as_i64().expect("int") as i32)
        .collect()
}

/// Gerador congruencial simples (determinístico) para dados SINTÉTICOS de teste.
pub struct Lcg(pub u64);

impl Lcg {
    pub fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 33) as u32
    }
    /// Uniforme em [0, 1).
    pub fn unif(&mut self) -> f64 {
        f64::from(self.next_u32()) / 2147483648.0
    }
    /// Aproximadamente normal (soma de 12 uniformes).
    pub fn normal(&mut self, mu: f64, sd: f64) -> f64 {
        let s: f64 = (0..12).map(|_| self.unif()).sum();
        mu + sd * (s - 6.0)
    }
}

pub fn is_leap(y: i32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

pub fn month_days(y: i32, m: u8) -> u8 {
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

/// Clima SINTÉTICO só para testes (1981-01-01..2025-12-31); nunca apresentado como real.
pub fn synth_climate(seed: u64) -> ClimateFrame {
    let mut rng = Lcg(seed);
    let mut fr = ClimateFrame::new();
    for y in 1981..=2025 {
        let mut doy = 0.0f64;
        for m in 1..=12u8 {
            for d in 1..=month_days(y, m) {
                doy += 1.0;
                let wet = 0.5 + 0.45 * (2.0 * std::f64::consts::PI * (doy - 15.0) / 365.0).cos();
                let p = if rng.unif() < wet {
                    -rng.unif().max(1e-9).ln() * 7.0
                } else {
                    0.0
                };
                let season = (2.0 * std::f64::consts::PI * (doy - 80.0) / 365.0).sin();
                let tx = 30.0 + 3.0 * season + rng.normal(0.0, 0.7);
                let tn = 16.0 + 4.0 * season + rng.normal(0.0, 0.7);
                let p = (p * 100.0).round() / 100.0;
                fr.insert(y, m, d, [p, (tx + tn) / 2.0, tx, tn])
                    .expect("data única");
            }
        }
    }
    fr
}
