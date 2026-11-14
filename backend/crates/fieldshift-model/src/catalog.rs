// AI-assisted (OpenBot agent)
//! Catálogo agronômico, calendário e enumeração dos planos viáveis (ZARC <= 20%).
//!
//! Porte fiel de `src/fieldshift/catalog.py` (commit `16cc391`). Núcleo puro: recebe os TEXTOS dos
//! arquivos de configuração (`catalog.toml`, `soil.toml`, tábua ZARC em CSV); quem lê o disco é o
//! adaptador (`fieldshift-io`, CLI, testes).
//!
//! * Dia-índice `t`: 0 = 1/set do ano y; 365 dias por ano agrícola (29/fev descartado); horizonte 730.
//! * Plano `x = (o1, o2, o3, o4)`: J1/J3 = (cultura de verão, decêndio de semeadura); J2/J4 = 2ª safra
//!   com semeadura DERIVADA (1º dia >= colheita anterior + margem num decêndio a 20%).
//! * Pousio em J2/J4 só quando nenhuma 2ª safra é viável.
//! * ZARC: só decêndios com risco exatamente <= 20%. Não há como elevar o limite. X vazio => INVIÁVEL.

use std::collections::BTreeMap;

use crate::compat::sha256_hex;
use serde::Serialize;
use serde_json::{json, Value};

pub const DAYS_PER_YEAR: i32 = 365;
pub const N_YEARS: i32 = 2;
pub const HORIZON_DAYS: i32 = DAYS_PER_YEAR * N_YEARS;
pub const ZARC_RISK_LIMIT_PCT: i32 = 20;
pub const WINDOWS: [&str; 4] = ["J1", "J2", "J3", "J4"];

/// Meses do ano agrícola (set..ago) no calendário sem 29/fev.
const MONTH_ORDER: [i32; 12] = [9, 10, 11, 12, 1, 2, 3, 4, 5, 6, 7, 8];
const MONTH_NAMES: [&str; 13] = [
    "", "jan", "fev", "mar", "abr", "mai", "jun", "jul", "ago", "set", "out", "nov", "dez",
];

fn month_len(m: i32) -> i32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => 28,
        _ => 0,
    }
}

fn month_start(m: i32) -> i32 {
    let mut acc = 0;
    for &mm in &MONTH_ORDER {
        if mm == m {
            return acc;
        }
        acc += month_len(mm);
    }
    0
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[error("{0}")]
pub struct CatalogError(pub String);

type Res<T> = Result<T, CatalogError>;

fn err<T>(msg: impl Into<String>) -> Res<T> {
    Err(CatalogError(msg.into()))
}

/// Dia-índice (0..364) de mês/dia no ano agrícola de 365 dias (1/set = 0).
pub fn md_to_t(month: i32, day: i32) -> Res<i32> {
    if !(1..=12).contains(&month) || day < 1 || day > month_len(month) {
        return err(format!(
            "data inválida no calendário de 365 dias: {month}/{day}"
        ));
    }
    Ok(month_start(month) + day - 1)
}

/// `"09-25"` -> dia-índice.
pub fn parse_md(text: &str) -> Res<i32> {
    let (mm, dd) = text
        .split_once('-')
        .ok_or_else(|| CatalogError(format!("data MM-DD inválida: {text:?}")))?;
    let m: i32 = mm
        .parse()
        .map_err(|_| CatalogError(format!("mês inválido: {mm:?}")))?;
    let d: i32 = dd
        .parse()
        .map_err(|_| CatalogError(format!("dia inválido: {dd:?}")))?;
    md_to_t(m, d)
}

/// Dia-índice (qualquer ano do horizonte) -> (mês, dia).
pub fn t_to_md(t: i32) -> (i32, i32) {
    let u = t.rem_euclid(DAYS_PER_YEAR);
    for &m in MONTH_ORDER.iter().rev() {
        if u >= month_start(m) {
            return (m, u - month_start(m) + 1);
        }
    }
    (9, 1)
}

/// `"26/jan"` (com sufixo `" (ano 2)"` quando t >= 365).
pub fn t_label(t: i32) -> String {
    let (m, d) = t_to_md(t);
    let mut s = format!("{d:02}/{}", MONTH_NAMES[m as usize]);
    if t >= DAYS_PER_YEAR {
        s.push_str(" (ano 2)");
    }
    s
}

/// Primeiro dia (0..364) do decêndio ZARC `dec` (1..36; dec1 = 1–10/jan, dec25 = 1–10/set).
pub fn dec_start_t(dec: i32) -> Res<i32> {
    if !(1..=36).contains(&dec) {
        return err(format!("decêndio inválido: {dec}"));
    }
    let (month, part) = ((dec - 1) / 3 + 1, (dec - 1) % 3);
    md_to_t(month, 1 + 10 * part)
}

/// Último dia (inclusive) do decêndio.
pub fn dec_end_t(dec: i32) -> Res<i32> {
    if !(1..=36).contains(&dec) {
        return err(format!("decêndio inválido: {dec}"));
    }
    let (month, part) = ((dec - 1) / 3 + 1, (dec - 1) % 3);
    let day = match part {
        0 => 10,
        1 => 20,
        _ => month_len(month),
    };
    md_to_t(month, day)
}

/// Decêndio ZARC (1..36) do dia-índice t (qualquer ano).
pub fn t_to_dec(t: i32) -> i32 {
    let (m, d) = t_to_md(t);
    let part = if d <= 10 {
        0
    } else if d <= 20 {
        1
    } else {
        2
    };
    (m - 1) * 3 + part + 1
}

/// Posição cronológica do decêndio no ano agrícola (dec25 -> 0, dec36 -> 11, dec1 -> 12 ...).
pub fn dec_ordinal(dec: i32) -> i32 {
    (dec - 25).rem_euclid(36)
}

// --------------------------------------------------------------------------- dados
#[derive(Debug, Clone, PartialEq)]
pub struct Crop {
    pub key: String,
    pub label: String,
    pub species: Vec<String>,
    pub grain: bool,
    pub windows: Vec<String>,
    pub zarc_name: Option<String>,
    pub zarc_soil_key: Option<String>,
    pub cycle_days: i32,
    pub phase_days: Vec<i32>,
    pub brachiaria_cover_after_harvest: bool,
    pub legal_window: Option<(i32, i32)>,
    pub is_fallow: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WindowSpec {
    pub name: String,
    pub year_index: i32,
    pub crops: Vec<String>,
    pub sowing_rule: String,
    pub decendios: Vec<i32>,
    /// `"false"` | `"true"` | `"only_if_no_feasible_crop"`.
    pub fallow_allowed: String,
}

/// Solo informado: classe AD1–AD6 e textura (arenosa/media/argilosa).
#[derive(Debug, Clone, PartialEq)]
pub struct Soil {
    pub ad_class: String,
    pub texture: String,
    pub cod_ad: BTreeMap<String, String>,
    pub cod_texture: BTreeMap<String, String>,
}

impl Soil {
    pub fn cod_solo(&self, key: &str) -> Res<String> {
        let (map, k) = match key {
            "ad" => (&self.cod_ad, &self.ad_class),
            "texture" => (&self.cod_texture, &self.texture),
            _ => return err(format!("zarc_soil_key desconhecido: '{key}'")),
        };
        map.get(k)
            .cloned()
            .ok_or_else(|| CatalogError(format!("solo sem código ZARC: {k}")))
    }

    pub fn describe(&self) -> Value {
        json!({"ad_class": self.ad_class, "texture": self.texture})
    }
}

pub type ZarcKey = (String, String);

#[derive(Debug, Clone)]
pub struct Catalog {
    /// Ordem do TOML; o pousio é a última entrada.
    pub crops: Vec<Crop>,
    pub windows: BTreeMap<String, WindowSpec>,
    pub emergence_days: i32,
    pub margin_days: i32,
    /// Último dia (inclusive) em que a semeadura derivada pode ocorrer (30/abr).
    pub j2_search_end_t: i32,
    pub brachiaria_desiccation_days: i32,
    /// (zarc_name, cod_solo) -> 36 riscos (%).
    pub zarc: BTreeMap<ZarcKey, [i32; 36]>,
    pub default_p0: PlanSpec,
    pub files_sha256: BTreeMap<String, String>,
    pub soil_defaults: BTreeMap<String, String>,
    pub cod_ad: BTreeMap<String, String>,
    pub cod_texture: BTreeMap<String, String>,
}

impl Catalog {
    pub fn fallow_index(&self) -> usize {
        self.crops
            .iter()
            .position(|c| c.is_fallow)
            .unwrap_or(self.crops.len() - 1)
    }

    pub fn index_of(&self, key: &str) -> Res<usize> {
        self.crops
            .iter()
            .position(|c| c.key == key)
            .ok_or_else(|| CatalogError(format!("cultura desconhecida: '{key}'")))
    }

    pub fn zarc_row(&self, crop: &Crop, soil: &Soil) -> Res<&[i32; 36]> {
        let (Some(name), Some(sk)) = (&crop.zarc_name, &crop.zarc_soil_key) else {
            return err(format!("{} não tem linha ZARC", crop.key));
        };
        let key = (name.clone(), soil.cod_solo(sk)?);
        self.zarc.get(&key).ok_or_else(|| {
            CatalogError(format!(
                "tábua ZARC sem linha para ('{}', '{}')",
                key.0, key.1
            ))
        })
    }

    pub fn zarc_risk(&self, crop: &Crop, soil: &Soil, dec: i32) -> Res<i32> {
        Ok(self.zarc_row(crop, soil)?[(dec - 1) as usize])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Occupation {
    /// 0..=3 para J1..J4.
    pub window: usize,
    /// Índice no catálogo (pousio = `fallow_index`).
    pub crop: usize,
    pub sow_t: Option<i32>,
    pub emerge_t: Option<i32>,
    /// Exclusivo: ciclo vivo = `[emerge_t, harvest_t)`.
    pub harvest_t: Option<i32>,
    /// Decêndio ZARC da semeadura.
    pub dec: Option<i32>,
}

impl Occupation {
    pub fn is_fallow(&self) -> bool {
        self.sow_t.is_none()
    }
    pub fn window_name(&self) -> &'static str {
        WINDOWS[self.window]
    }
}

pub type PlanId = [i32; 6];

/// Plano imutável. `id` = (c1, d1, c2, c3, d3, c4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub id: PlanId,
    pub occupations: [Occupation; 4],
}

impl Plan {
    pub fn signature(&self) -> [usize; 4] {
        let o = &self.occupations;
        [o[0].crop, o[1].crop, o[2].crop, o[3].crop]
    }

    pub fn fallow_windows(&self) -> Vec<&'static str> {
        self.occupations
            .iter()
            .filter(|o| o.is_fallow())
            .map(|o| o.window_name())
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Rejection {
    pub window: String,
    pub crop: String,
    pub dec: Option<i32>,
    pub reason: String,
    pub detail: String,
}

impl Rejection {
    fn new(window: &str, crop: &str, dec: Option<i32>, reason: &str, detail: &str) -> Self {
        Rejection {
            window: window.into(),
            crop: crop.into(),
            dec,
            reason: reason.into(),
            detail: detail.into(),
        }
    }
    pub fn to_value(&self) -> Value {
        json!({"window": self.window, "crop": self.crop, "dec": self.dec,
               "reason": self.reason, "detail": self.detail})
    }
}

/// Plano informado pelo usuário (P0): cultura e decêndio de J1/J3 e cultura de J2/J4.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanSpec {
    pub j1: String,
    pub dec1: i32,
    pub j2: String,
    pub j3: String,
    pub dec3: i32,
    pub j4: String,
}

impl PlanSpec {
    pub fn to_value(&self) -> Value {
        json!({"j1": self.j1, "dec1": self.dec1, "j2": self.j2,
               "j3": self.j3, "dec3": self.dec3, "j4": self.j4})
    }
}

// --------------------------------------------------------------------------- leitura (textos)
type Table = toml::map::Map<String, toml::Value>;

fn tbl<'a>(t: &'a Table, k: &str) -> Res<&'a Table> {
    t.get(k)
        .and_then(|v| v.as_table())
        .ok_or_else(|| CatalogError(format!("tabela ausente: {k}")))
}

fn int(t: &Table, k: &str) -> Res<i64> {
    t.get(k)
        .and_then(|v| v.as_integer())
        .ok_or_else(|| CatalogError(format!("inteiro ausente: {k}")))
}

fn string(t: &Table, k: &str) -> Res<String> {
    t.get(k)
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .ok_or_else(|| CatalogError(format!("texto ausente: {k}")))
}

fn str_list(v: &toml::Value) -> Res<Vec<String>> {
    v.as_array()
        .ok_or_else(|| CatalogError("lista esperada".into()))?
        .iter()
        .map(|x| {
            x.as_str()
                .map(str::to_string)
                .ok_or_else(|| CatalogError("texto esperado".into()))
        })
        .collect()
}

fn int_list(v: &toml::Value) -> Res<Vec<i32>> {
    v.as_array()
        .ok_or_else(|| CatalogError("lista esperada".into()))?
        .iter()
        .map(|x| {
            x.as_integer()
                .map(|i| i as i32)
                .ok_or_else(|| CatalogError("inteiro esperado".into()))
        })
        .collect()
}

fn parse_toml(text: &str, what: &str) -> Res<Table> {
    text.parse::<Table>()
        .map_err(|e| CatalogError(format!("{what}: TOML inválido: {e}")))
}

fn utf8(bytes: &[u8], what: &str) -> Res<String> {
    std::str::from_utf8(bytes)
        .map(str::to_string)
        .map_err(|_| CatalogError(format!("{what}: não é UTF-8")))
}

/// Solo informado (padrão: `[default]` de soil.toml, hipótese AD4 + argilosa).
pub fn load_soil(soil_toml: &[u8], ad_class: Option<&str>, texture: Option<&str>) -> Res<Soil> {
    let cfg = parse_toml(&utf8(soil_toml, "soil.toml")?, "soil.toml")?;
    let cod = |section: &str| -> Res<BTreeMap<String, String>> {
        tbl(&cfg, section)?
            .iter()
            .map(|(k, v)| {
                let t = v
                    .as_table()
                    .ok_or_else(|| CatalogError(format!("{section}.{k}")))?;
                Ok((k.clone(), string(t, "zarc_cod_solo")?))
            })
            .collect()
    };
    let cod_ad = cod("ad_classes")?;
    let cod_tx = cod("textures")?;
    let def = tbl(&cfg, "default")?;
    let ad = match ad_class {
        Some(a) => a.to_string(),
        None => string(def, "ad_class")?,
    };
    let tx = match texture {
        Some(t) => t.to_string(),
        None => string(def, "texture")?,
    };
    if !cod_ad.contains_key(&ad) {
        let valid: Vec<&String> = cod_ad.keys().collect();
        return err(format!(
            "classe AD desconhecida: '{ad}' (válidas: {valid:?})"
        ));
    }
    if !cod_tx.contains_key(&tx) {
        let valid: Vec<&String> = cod_tx.keys().collect();
        return err(format!("textura desconhecida: '{tx}' (válidas: {valid:?})"));
    }
    Ok(Soil {
        ad_class: ad,
        texture: tx,
        cod_ad,
        cod_texture: cod_tx,
    })
}

fn load_zarc(csv_bytes: &[u8], zc: &Table, names: &[String]) -> Res<BTreeMap<ZarcKey, [i32; 36]>> {
    let bytes = csv_bytes
        .strip_prefix(&[0xEF, 0xBB, 0xBF])
        .unwrap_or(csv_bytes);
    let sep = string(zc, "csv_separator")?;
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(sep.as_bytes().first().copied().unwrap_or(b';'))
        .has_headers(true)
        .from_reader(bytes);
    let head: Vec<String> = rdr
        .headers()
        .map_err(|e| CatalogError(format!("CSV ZARC: {e}")))?
        .iter()
        .map(str::to_string)
        .collect();
    let col = |name: &str| -> Res<usize> {
        head.iter()
            .position(|h| h == name)
            .ok_or_else(|| CatalogError(format!("CSV ZARC sem coluna {name}")))
    };
    let prefix = string(zc, "decendio_column_prefix")?;
    let dec_cols: Vec<usize> = (1..=36)
        .map(|d| col(&format!("{prefix}{d}")))
        .collect::<Res<_>>()?;
    let (c_geo, c_cic, c_man, c_name, c_solo) = (
        col("geocodigo")?,
        col("Cod_Ciclo")?,
        col("Cod_Outros_Manejos")?,
        col("Nome_cultura")?,
        col("Cod_Solo")?,
    );
    let (f_geo, f_cic, f_man) = (
        string(zc, "filter_geocodigo")?,
        string(zc, "filter_cod_ciclo")?,
        string(zc, "filter_cod_outros_manejos")?,
    );
    let mut out: BTreeMap<ZarcKey, [i32; 36]> = BTreeMap::new();
    for rec in rdr.records() {
        let r = rec.map_err(|e| CatalogError(format!("CSV ZARC: {e}")))?;
        if r[c_geo] != *f_geo
            || r[c_cic] != *f_cic
            || r[c_man] != *f_man
            || !names.iter().any(|n| *n == r[c_name])
        {
            continue;
        }
        let key = (r[c_name].to_string(), r[c_solo].to_string());
        if out.contains_key(&key) {
            return err(format!(
                "tábua ZARC com linha duplicada para ('{}', '{}')",
                key.0, key.1
            ));
        }
        let mut row = [0i32; 36];
        for (i, &c) in dec_cols.iter().enumerate() {
            row[i] = r[c]
                .trim()
                .parse::<i32>()
                .map_err(|_| CatalogError(format!("risco ZARC inválido: {:?}", &r[c])))?;
        }
        out.insert(key, row);
    }
    Ok(out)
}

/// Fontes brutas de configuração (bytes dos arquivos).
pub struct CatalogSources<'a> {
    pub catalog_toml: &'a [u8],
    pub soil_toml: &'a [u8],
    pub zarc_csv: &'a [u8],
}

/// Lê catalog.toml + soil.toml + tábua ZARC (confere o SHA-256 declarado).
pub fn load_catalog(src: &CatalogSources) -> Res<Catalog> {
    let cfg = parse_toml(&utf8(src.catalog_toml, "catalog.toml")?, "catalog.toml")?;
    let cal = tbl(&cfg, "calendar")?;
    if int(cal, "days_per_agri_year")? != DAYS_PER_YEAR as i64
        || int(cal, "horizon_days")? != HORIZON_DAYS as i64
    {
        return err("calendário do catálogo diverge do módulo (365/730 dias)");
    }
    let zc = tbl(&cfg, "zarc")?;
    if int(zc, "risk_limit_percent")? != ZARC_RISK_LIMIT_PCT as i64 {
        return err("risk_limit_percent deve ser 20 (decisão 1: não relaxar)");
    }
    let zarc_sha = sha256_hex(src.zarc_csv);
    let declared = string(zc, "sha256")?;
    if zarc_sha != declared {
        return err(format!(
            "SHA-256 da tábua ZARC diverge: {zarc_sha} != {declared}"
        ));
    }

    let mut crops: Vec<Crop> = Vec::new();
    for (key, v) in tbl(&cfg, "crops")? {
        let c = v
            .as_table()
            .ok_or_else(|| CatalogError(format!("crops.{key}")))?;
        let phase_days = match c.get("phase_days") {
            Some(p) => int_list(p)?,
            None => vec![],
        };
        let cycle_days = c
            .get("cycle_days")
            .and_then(|x| x.as_integer())
            .unwrap_or(0) as i32;
        let legal_window = match c.get("legal_window") {
            Some(lw) => {
                let l = str_list(lw)?;
                Some((parse_md(&l[0])?, parse_md(&l[1])?))
            }
            None => None,
        };
        if !phase_days.is_empty() && phase_days.iter().sum::<i32>() != cycle_days {
            return err(format!("{key}: phase_days não soma cycle_days"));
        }
        crops.push(Crop {
            key: key.clone(),
            label: string(c, "label")?,
            species: str_list(
                c.get("species")
                    .ok_or_else(|| CatalogError("species".into()))?,
            )?,
            grain: c.get("grain").and_then(|x| x.as_bool()).unwrap_or(false),
            windows: str_list(
                c.get("windows")
                    .ok_or_else(|| CatalogError("windows".into()))?,
            )?,
            zarc_name: c
                .get("zarc_name")
                .and_then(|x| x.as_str())
                .map(str::to_string),
            zarc_soil_key: c
                .get("zarc_soil_key")
                .and_then(|x| x.as_str())
                .map(str::to_string),
            cycle_days,
            phase_days,
            brachiaria_cover_after_harvest: c
                .get("brachiaria_cover_after_harvest")
                .and_then(|x| x.as_bool())
                .unwrap_or(false),
            legal_window,
            is_fallow: key == "pousio",
        });
    }
    if !crops.last().is_some_and(|c| c.is_fallow) {
        return err("o pousio deve ser a última entrada de [crops]");
    }

    let mut windows = BTreeMap::new();
    for (name, v) in tbl(&cfg, "windows")? {
        let w = v
            .as_table()
            .ok_or_else(|| CatalogError(format!("windows.{name}")))?;
        let fa = w
            .get("fallow_allowed")
            .ok_or_else(|| CatalogError("fallow_allowed".into()))?;
        let fallow_allowed = match fa {
            toml::Value::Boolean(b) => b.to_string(),
            toml::Value::String(s) => s.clone(),
            _ => return err("fallow_allowed inválido"),
        };
        windows.insert(
            name.clone(),
            WindowSpec {
                name: name.clone(),
                year_index: int(w, "agri_year_index")? as i32,
                crops: str_list(w.get("crops").ok_or_else(|| CatalogError("crops".into()))?)?,
                sowing_rule: string(w, "sowing_rule")?,
                decendios: int_list(
                    w.get("decendios")
                        .ok_or_else(|| CatalogError("decendios".into()))?,
                )?,
                fallow_allowed,
            },
        );
    }
    if WINDOWS.iter().any(|w| !windows.contains_key(*w)) || windows.len() != 4 {
        return err("catálogo deve definir J1..J4");
    }

    let soil_cfg = parse_toml(&utf8(src.soil_toml, "soil.toml")?, "soil.toml")?;
    let names: Vec<String> = crops.iter().filter_map(|c| c.zarc_name.clone()).collect();
    let zarc = load_zarc(src.zarc_csv, zc, &names)?;
    let succ = tbl(&cfg, "succession")?;
    let mut files = BTreeMap::new();
    files.insert("catalog.toml".to_string(), sha256_hex(src.catalog_toml));
    files.insert("soil.toml".to_string(), sha256_hex(src.soil_toml));
    files.insert("zarc".to_string(), zarc_sha);

    let def = tbl(&soil_cfg, "default")?;
    let mut soil_defaults = BTreeMap::new();
    for k in ["ad_class", "texture"] {
        soil_defaults.insert(k.to_string(), string(def, k)?);
    }
    let soil = load_soil(src.soil_toml, None, None)?;

    let p = tbl(&cfg, "current_plan_default")?;
    let default_p0 = PlanSpec {
        j1: string(tbl(p, "J1")?, "crop")?,
        dec1: int(tbl(p, "J1")?, "decendio")? as i32,
        j2: string(tbl(p, "J2")?, "crop")?,
        j3: string(tbl(p, "J3")?, "crop")?,
        dec3: int(tbl(p, "J3")?, "decendio")? as i32,
        j4: string(tbl(p, "J4")?, "crop")?,
    };
    Ok(Catalog {
        crops,
        windows,
        emergence_days: int(cal, "emergence_days")? as i32,
        margin_days: int(cal, "operational_margin_days")? as i32,
        j2_search_end_t: md_to_t(4, 30)?,
        brachiaria_desiccation_days: int(succ, "brachiaria_desiccation_days_before_next_sowing")?
            as i32,
        zarc,
        default_p0,
        files_sha256: files,
        soil_defaults,
        cod_ad: soil.cod_ad,
        cod_texture: soil.cod_texture,
    })
}

// --------------------------------------------------------------------------- enumeração
fn risk_limit(zarc_risk: f64) -> Res<i32> {
    let pct = (zarc_risk * 100.0).round_ties_even() as i32;
    if pct > ZARC_RISK_LIMIT_PCT {
        return err("o limite ZARC não pode passar de 20% (decisão 1 do usuário)");
    }
    Ok(pct)
}

fn occupation(cat: &Catalog, window: usize, crop_idx: usize, sow_t: i32) -> Occupation {
    let crop = &cat.crops[crop_idx];
    let e = sow_t + cat.emergence_days;
    Occupation {
        window,
        crop: crop_idx,
        sow_t: Some(sow_t),
        emerge_t: Some(e),
        harvest_t: Some(e + crop.cycle_days),
        dec: Some(t_to_dec(sow_t)),
    }
}

fn fallow(cat: &Catalog, window: usize) -> Occupation {
    Occupation {
        window,
        crop: cat.fallow_index(),
        sow_t: None,
        emerge_t: None,
        harvest_t: None,
        dec: None,
    }
}

fn allowed(risk: i32, limit: i32) -> bool {
    0 < risk && risk <= limit
}

fn first_options(
    cat: &Catalog,
    soil: &Soil,
    limit: i32,
    wname: &str,
    rej: &mut Vec<Rejection>,
) -> Res<Vec<Occupation>> {
    let widx = WINDOWS.iter().position(|w| *w == wname).unwrap_or(0);
    let win = &cat.windows[wname];
    let mut out = Vec::new();
    for key in &win.crops {
        let idx = cat.index_of(key)?;
        let crop = &cat.crops[idx];
        for &dec in &win.decendios {
            let risk = cat.zarc_risk(crop, soil, dec)?;
            if risk == 0 {
                rej.push(Rejection::new(
                    wname,
                    key,
                    Some(dec),
                    "ZARC_NAO_INDICADO",
                    "risco 0 = não recomendado",
                ));
                continue;
            }
            if risk > limit {
                rej.push(Rejection::new(
                    wname,
                    key,
                    Some(dec),
                    "ZARC_RISCO_ACIMA",
                    &format!("risco {risk}% > {limit}%"),
                ));
                continue;
            }
            let s = dec_start_t(dec)?;
            if let Some((lo, hi)) = crop.legal_window {
                if !(dec_start_t(dec)? >= lo && dec_end_t(dec)? <= hi) {
                    rej.push(Rejection::new(
                        wname,
                        key,
                        Some(dec),
                        "JANELA_LEGAL",
                        "decêndio fora da janela legal de semeadura",
                    ));
                    continue;
                }
            }
            let occ = occupation(cat, widx, idx, s);
            if occ.harvest_t.unwrap_or(0) > DAYS_PER_YEAR {
                rej.push(Rejection::new(
                    wname,
                    key,
                    Some(dec),
                    "FORA_DO_ANO",
                    "colheita depois de 31/ago",
                ));
                continue;
            }
            out.push(occ);
        }
    }
    Ok(out)
}

fn derived_second(
    cat: &Catalog,
    soil: &Soil,
    limit: i32,
    wname: &str,
    prev_harvest: i32,
    crop_key: &str,
    rej: &mut Vec<Rejection>,
) -> Res<Option<Occupation>> {
    let widx = WINDOWS.iter().position(|w| *w == wname).unwrap_or(1);
    let idx = cat.index_of(crop_key)?;
    let crop = &cat.crops[idx];
    let t0 = prev_harvest + cat.margin_days;
    for t in t0..=cat.j2_search_end_t {
        if allowed(cat.zarc_risk(crop, soil, t_to_dec(t))?, limit) {
            let occ = occupation(cat, widx, idx, t);
            if occ.harvest_t.unwrap_or(0) > DAYS_PER_YEAR {
                rej.push(Rejection::new(
                    wname,
                    crop_key,
                    occ.dec,
                    "FORA_DO_ANO",
                    "colheita depois de 31/ago",
                ));
                return Ok(None);
            }
            return Ok(Some(occ));
        }
    }
    rej.push(Rejection::new(
        wname,
        crop_key,
        Some(t_to_dec(t0.min(DAYS_PER_YEAR - 1))),
        "SEM_DECENDIO_ZARC_APOS_COLHEITA",
        &format!(
            "colheita anterior em {}: nenhum decêndio a <= {limit}% até 30/abr",
            t_label(prev_harvest)
        ),
    ));
    Ok(None)
}

type Block = (Occupation, Occupation);

fn year_blocks(
    cat: &Catalog,
    soil: &Soil,
    limit: i32,
    rej: &mut Vec<Rejection>,
) -> Res<Vec<Block>> {
    let mut blocks = Vec::new();
    let w2 = &cat.windows["J2"];
    for o1 in first_options(cat, soil, limit, "J1", rej)? {
        let mut viable = Vec::new();
        for key in &w2.crops {
            if cat.crops[cat.index_of(key)?].is_fallow {
                continue;
            }
            if let Some(occ) =
                derived_second(cat, soil, limit, "J2", o1.harvest_t.unwrap_or(0), key, rej)?
            {
                viable.push(occ);
            }
        }
        let mode = w2.fallow_allowed.as_str();
        if !viable.is_empty() {
            for o2 in &viable {
                blocks.push((o1, *o2));
            }
            if mode == "true" {
                blocks.push((o1, fallow(cat, 1)));
            }
        } else if mode == "only_if_no_feasible_crop" || mode == "true" {
            blocks.push((o1, fallow(cat, 1)));
        } else {
            rej.push(Rejection::new(
                "J2",
                "-",
                o1.dec,
                "SEM_SEGUNDA_SAFRA",
                "pousio não permitido",
            ));
        }
    }
    Ok(blocks)
}

fn shift(occ: &Occupation, window: usize, offset: i32) -> Occupation {
    if occ.is_fallow() {
        return Occupation { window, ..*occ };
    }
    Occupation {
        window,
        crop: occ.crop,
        sow_t: occ.sow_t.map(|v| v + offset),
        emerge_t: occ.emerge_t.map(|v| v + offset),
        harvest_t: occ.harvest_t.map(|v| v + offset),
        dec: occ.dec,
    }
}

fn plan_id_of(o: &[Occupation; 4]) -> PlanId {
    let ordn = |x: &Occupation| x.dec.map_or(-1, dec_ordinal);
    [
        o[0].crop as i32,
        ordn(&o[0]),
        o[1].crop as i32,
        o[2].crop as i32,
        ordn(&o[2]),
        o[3].crop as i32,
    ]
}

fn last_end(b: &Block) -> i32 {
    [b.0.harvest_t, b.1.harvest_t]
        .into_iter()
        .flatten()
        .max()
        .unwrap_or(0)
}

/// Todos os planos viáveis (ordenados por ID) e a lista de rejeições com motivo.
///
/// `zarc_risk` só aceita <= 0.20 (a decisão do usuário proíbe relaxar). Lista vazia => INVIÁVEL.
pub fn enumerate_plans(
    cat: &Catalog,
    soil: &Soil,
    zarc_risk: f64,
) -> Res<(Vec<Plan>, Vec<Rejection>)> {
    let limit = risk_limit(zarc_risk)?;
    let mut rej = Vec::new();
    let blocks = year_blocks(cat, soil, limit, &mut rej)?;
    let mut plans: Vec<Plan> = Vec::new();
    for b1 in &blocks {
        for b2 in &blocks {
            let occ3 = shift(&b2.0, 2, DAYS_PER_YEAR);
            let occ4 = shift(&b2.1, 3, DAYS_PER_YEAR);
            if last_end(b1) > occ3.sow_t.unwrap_or(0) {
                rej.push(Rejection::new(
                    "J3",
                    &cat.crops[occ3.crop].key,
                    occ3.dec,
                    "SOBREPOSICAO_J2_J3",
                    "",
                ));
                continue;
            }
            if occ4.harvest_t.is_some_and(|h| h > HORIZON_DAYS) {
                rej.push(Rejection::new(
                    "J4",
                    &cat.crops[occ4.crop].key,
                    occ4.dec,
                    "FORA_DO_HORIZONTE",
                    "",
                ));
                continue;
            }
            let occs = [b1.0, b1.1, occ3, occ4];
            plans.push(Plan {
                id: plan_id_of(&occs),
                occupations: occs,
            });
        }
    }
    plans.sort_by_key(|a| a.id);
    for w in plans.windows(2) {
        if w[0].id == w[1].id {
            return err("IDs de plano duplicados");
        }
    }
    Ok((plans, rej))
}

/// Plano por ID (os planos de `enumerate_plans` já vêm ordenados por ID).
pub fn find_plan<'a>(plans: &'a [Plan], id: &PlanId) -> Option<&'a Plan> {
    plans
        .binary_search_by(|p| p.id.cmp(id))
        .ok()
        .map(|i| &plans[i])
}

/// Posição do plano na lista ordenada.
pub fn plan_index(plans: &[Plan], id: &PlanId) -> Option<usize> {
    plans.binary_search_by(|p| p.id.cmp(id)).ok()
}

// --------------------------------------------------------------------------- verificador independente
/// Violações do plano (vazio = aprovado). Reimplementa as restrições a partir das datas.
pub fn verify_plan(cat: &Catalog, soil: &Soil, plan: &Plan, zarc_risk: f64) -> Res<Vec<String>> {
    let limit = risk_limit(zarc_risk)?;
    let mut errs: Vec<String> = Vec::new();
    let occs = &plan.occupations;
    for (i, o) in occs.iter().enumerate() {
        if o.window != i {
            return Ok(vec!["janelas fora de ordem".into()]);
        }
    }
    for o in occs {
        let crop = &cat.crops[o.crop];
        let wname = o.window_name();
        if crop.is_fallow {
            if cat.windows[wname].fallow_allowed == "false" {
                errs.push(format!("{wname}: pousio não permitido na janela"));
            }
            continue;
        }
        if !cat.windows[wname].crops.contains(&crop.key) {
            errs.push(format!(
                "{wname}: cultura {} não pertence à janela",
                crop.key
            ));
            continue;
        }
        let (Some(s), Some(e), Some(h)) = (o.sow_t, o.emerge_t, o.harvest_t) else {
            errs.push(format!("{wname}: ocupação sem datas"));
            continue;
        };
        if e != s + cat.emergence_days || h != e + crop.cycle_days {
            errs.push(format!("{wname}: datas inconsistentes com o ciclo"));
        }
        let dec = t_to_dec(s);
        let risk = cat.zarc_risk(crop, soil, dec)?;
        if !allowed(risk, limit) {
            errs.push(format!(
                "{wname}: ZARC {risk}% no decêndio {dec} (limite {limit}%)"
            ));
        }
        if let Some((lo, hi)) = crop.legal_window {
            let u = s.rem_euclid(DAYS_PER_YEAR);
            if !(dec_start_t(dec)? >= lo && dec_end_t(dec)? <= hi && lo <= u) {
                errs.push(format!("{wname}: fora da janela legal"));
            }
        }
        let yr0 = DAYS_PER_YEAR * cat.windows[wname].year_index;
        if !(yr0 <= s && h <= yr0 + DAYS_PER_YEAR) {
            errs.push(format!("{wname}: ocupação fora do próprio ano agrícola"));
        }
    }
    let [j1, j2, j3, j4] = occs;
    if let (Some(_), Some(h1)) = (j1.sow_t, j1.harvest_t) {
        if let Some(s2) = j2.sow_t {
            if s2 < h1 + cat.margin_days {
                errs.push("J2 começa antes da colheita de J1 (sobreposição)".into());
            }
        }
        if let Some(s3) = j3.sow_t {
            if s3 < h1 {
                errs.push("J3 começa antes da colheita de J1".into());
            }
        }
    }
    for (prev, nxt, label) in [(j2, j3, "J2/J3"), (j3, j4, "J3/J4")] {
        if let (Some(ph), Some(ns)) = (prev.harvest_t, nxt.sow_t) {
            let margin = if label == "J3/J4" { cat.margin_days } else { 0 };
            if ns < ph + margin {
                errs.push(format!("sobreposição {label}"));
            }
        }
    }
    if j1.is_fallow() || j3.is_fallow() {
        errs.push("J1/J3 devem ter cultura de grão".into());
    }
    Ok(errs)
}

// --------------------------------------------------------------------------- P0 e relatos
pub fn plan_id_from_spec(cat: &Catalog, spec: &PlanSpec) -> Res<PlanId> {
    Ok([
        cat.index_of(&spec.j1)? as i32,
        dec_ordinal(spec.dec1),
        cat.index_of(&spec.j2)? as i32,
        cat.index_of(&spec.j3)? as i32,
        dec_ordinal(spec.dec3),
        cat.index_of(&spec.j4)? as i32,
    ])
}

/// Plano do usuário dentro de X, ou `None` com os motivos ("inviável sob ZARC <= 20%").
pub fn resolve_spec(
    cat: &Catalog,
    soil: &Soil,
    spec: &PlanSpec,
    plans: &[Plan],
    zarc_risk: f64,
) -> Res<(Option<Plan>, Vec<String>)> {
    let wanted = match plan_id_from_spec(cat, spec) {
        Ok(w) => w,
        Err(e) => return Ok((None, vec![format!("plano inválido: {}", e.0)])),
    };
    if let Some(p) = find_plan(plans, &wanted) {
        return Ok((Some(p.clone()), vec![]));
    }
    let limit = risk_limit(zarc_risk)?;
    let mut reasons: Vec<String> = Vec::new();
    for (w, key, dec) in [("J1", &spec.j1, spec.dec1), ("J3", &spec.j3, spec.dec3)] {
        let idx = cat.index_of(key)?;
        let crop = &cat.crops[idx];
        if !crop.windows.iter().any(|x| x == w) {
            reasons.push(format!("{w}: {key} não pertence à janela"));
            continue;
        }
        let mut rej = Vec::new();
        let opts = first_options(cat, soil, limit, "J1", &mut rej)?;
        let ok = opts.iter().any(|o| o.crop == idx && o.dec == Some(dec));
        if !ok {
            let hit: Vec<&Rejection> = rej
                .iter()
                .filter(|r| &r.crop == key && r.dec == Some(dec))
                .collect();
            if let Some(h) = hit.first() {
                reasons.push(format!(
                    "{w}: {key} no decêndio {dec} não é permitido ({}: {})",
                    h.reason, h.detail
                ));
            } else {
                reasons.push(format!(
                    "{w}: {key} no decêndio {dec} fora dos decêndios candidatos"
                ));
            }
        }
    }
    if !reasons.is_empty() {
        return Ok((None, reasons));
    }
    for (w, k1, d1, k2) in [
        ("J2", &spec.j1, spec.dec1, &spec.j2),
        ("J4", &spec.j3, spec.dec3, &spec.j4),
    ] {
        let mut rej = Vec::new();
        let i1 = cat.index_of(k1)?;
        let Some(o1) = first_options(cat, soil, limit, "J1", &mut Vec::new())?
            .into_iter()
            .find(|o| o.crop == i1 && o.dec == Some(d1))
        else {
            continue;
        };
        let crop2 = &cat.crops[cat.index_of(k2)?];
        let h1 = o1.harvest_t.unwrap_or(0);
        if crop2.is_fallow {
            let mut viable: Vec<&str> = Vec::new();
            for k in &cat.windows["J2"].crops {
                if cat.crops[cat.index_of(k)?].is_fallow {
                    continue;
                }
                if derived_second(cat, soil, limit, "J2", h1, k, &mut rej)?.is_some() {
                    viable.push(k);
                }
            }
            if !viable.is_empty() {
                reasons.push(format!(
                    "{w}: pousio não permitido: há 2ª safra viável ({})",
                    viable.join(", ")
                ));
            }
        } else if derived_second(cat, soil, limit, "J2", h1, k2, &mut rej)?.is_none() {
            let detail = rej.last().map(|r| r.detail.clone()).unwrap_or_default();
            reasons.push(format!(
                "{w}: {k2} inviável sob ZARC <= {limit}% depois da colheita de {k1} ({detail})"
            ));
        }
    }
    if reasons.is_empty() {
        reasons.push("plano fora de X (sobreposição entre anos ou fora do horizonte)".into());
    }
    Ok((None, reasons))
}

/// Resumo legível e serializável do plano.
pub fn describe_plan(cat: &Catalog, plan: &Plan) -> Value {
    let occ: Vec<Value> = plan
        .occupations
        .iter()
        .map(|o| {
            let crop = &cat.crops[o.crop];
            json!({
                "window": o.window_name(),
                "crop": crop.key,
                "label": crop.label,
                "sow": o.sow_t.map(t_label),
                "sow_t": o.sow_t,
                "harvest_t": o.harvest_t,
                "zarc_dec": o.dec,
            })
        })
        .collect();
    json!({"id": plan.id.to_vec(), "occupations": occ})
}

/// Quantos planos de X usam pousio (J2, J4, ambos).
pub fn fallow_summary(plans: &[Plan]) -> BTreeMap<String, i64> {
    let mut c: BTreeMap<String, i64> = BTreeMap::new();
    for p in plans {
        let fw = p.fallow_windows();
        *c.entry("total".into()).or_insert(0) += 1;
        *c.entry("com_pousio".into()).or_insert(0) += i64::from(!fw.is_empty());
        *c.entry("pousio_J2".into()).or_insert(0) += i64::from(fw.contains(&"J2"));
        *c.entry("pousio_J4".into()).or_insert(0) += i64::from(fw.contains(&"J4"));
        *c.entry("pousio_J2_e_J4".into()).or_insert(0) += i64::from(fw.len() == 2);
    }
    c
}

/// `"janela:cultura:motivo" -> contagem`, ordenado.
pub fn summarize_rejections(rej: &[Rejection]) -> BTreeMap<String, i64> {
    let mut c: BTreeMap<String, i64> = BTreeMap::new();
    for r in rej {
        *c.entry(format!("{}:{}:{}", r.window, r.crop, r.reason))
            .or_insert(0) += 1;
    }
    c
}

/// Diagnóstico de X vazio (nada é relaxado).
pub fn diagnose_infeasible(rej: &[Rejection]) -> Value {
    json!({
        "status": "INVIAVEL",
        "message": "Nenhum plano viável com ZARC <= 20% para o solo informado. O limite, o grupo de ciclo e o solo não foram alterados.",
        "rejections_by_reason": summarize_rejections(rej),
        "n_rejections": rej.len(),
    })
}
