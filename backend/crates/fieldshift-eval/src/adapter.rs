// AI-assisted (OpenBot agent)
//! Adaptador real: recalcula (g1, g2, g3) com `fieldshift_model::indicators::loss_components`
//! a partir do plano e do clima. A referência de e3 (`ConstructionClimatology`) é montada AQUI,
//! só com os anos agrícolas dos pares de construção que o avaliador recebeu; nada do otimizador
//! é reaproveitado. Porte de `IndicatorLossModel` (com a correção VSDD M-1).

use std::collections::{BTreeMap, HashMap};

use fieldshift_model::catalog::{verify_plan, Catalog, Plan, Soil};
use fieldshift_model::indicators::{
    loss_components, ClimatePair, ConstructionClimatology, IndicatorParams,
};

use crate::evaluation::{EvalError, LossModel, PlanInfo, Scenario, DAYS_PER_YEAR};

struct CachedPair {
    precip: Vec<f64>,
    tmax: Vec<f64>,
    tmin: Vec<f64>,
    pair: ClimatePair,
}

/// `LossModel` sobre os indicadores Rust.
pub struct IndicatorLossModel<'a> {
    plans: &'a [Plan],
    base: &'a IndicatorParams,
    params: Option<IndicatorParams>,
    /// VSDD M-1: o cache é por rótulo, mas cada acerto CONFERE o conteúdo completo do cenário;
    /// conteúdo diferente com o mesmo rótulo (ex.: s̄ do leave-one-out) recria o par.
    pairs: HashMap<String, CachedPair>,
    pub fingerprint: Option<String>,
}

impl<'a> IndicatorLossModel<'a> {
    pub fn new(plans: &'a [Plan], base: &'a IndicatorParams) -> Self {
        IndicatorLossModel {
            plans,
            base,
            params: None,
            pairs: HashMap::new(),
            fingerprint: None,
        }
    }

    fn pair_for(&mut self, sc: &Scenario) -> Result<&ClimatePair, EvalError> {
        let key = format!("{}|{}", sc.label, sc.start);
        let fresh = match self.pairs.get(&key) {
            Some(c) => c.precip != sc.precip || c.tmax != sc.tmax || c.tmin != sc.tmin,
            None => true,
        };
        if fresh {
            let pair = ClimatePair::new(
                sc.precip.clone(),
                sc.tmax.clone(),
                sc.tmin.clone(),
                &sc.label,
            )
            .map_err(|e| EvalError(e.to_string()))?;
            self.pairs.insert(
                key.clone(),
                CachedPair {
                    precip: sc.precip.clone(),
                    tmax: sc.tmax.clone(),
                    tmin: sc.tmin.clone(),
                    pair,
                },
            );
        }
        Ok(&self.pairs[&key].pair)
    }
}

impl LossModel for IndicatorLossModel<'_> {
    fn fit(&mut self, construction: &[Scenario]) -> Result<String, EvalError> {
        let mut years: BTreeMap<i32, Vec<f64>> = BTreeMap::new();
        for sc in construction {
            for (i, y) in sc.years.iter().enumerate() {
                years.insert(
                    *y,
                    sc.precip[i * DAYS_PER_YEAR..(i + 1) * DAYS_PER_YEAR].to_vec(),
                );
            }
        }
        let labels: Vec<i32> = years.keys().copied().collect();
        let rows: Vec<Vec<f64>> = years.into_values().collect();
        let reference =
            ConstructionClimatology::new(&rows, labels).map_err(|e| EvalError(e.to_string()))?;
        let fp = reference.sha256();
        self.params = Some(
            self.base
                .clone_without_reference()
                .with_reference(reference),
        );
        self.fingerprint = Some(fp.clone());
        Ok(fp)
    }

    fn components(&mut self, plan: usize, sc: &Scenario) -> Result<[f64; 3], EvalError> {
        let plan = self
            .plans
            .get(plan)
            .ok_or_else(|| EvalError("plano fora de X".into()))?;
        // pair_for empresta &mut self; separa os campos para o borrow checker
        let pair = {
            self.pair_for(sc)?;
            let key = format!("{}|{}", sc.label, sc.start);
            &self.pairs[&key].pair
        };
        let params = self
            .params
            .as_ref()
            .ok_or_else(|| EvalError("fit não chamado".into()))?;
        loss_components(plan, pair, params).map_err(|e| EvalError(e.to_string()))
    }
}

/// Plano visto pelo avaliador (ID e genes por janela = (cultura, semeadura)).
pub fn plan_infos(plans: &[Plan]) -> Vec<PlanInfo> {
    plans
        .iter()
        .map(|p| PlanInfo {
            id: p.id.iter().map(|x| i64::from(*x)).collect(),
            genes: p
                .occupations
                .iter()
                .map(|o| (o.crop as i64, o.sow_t.map(i64::from)))
                .collect(),
        })
        .collect()
}

/// Verificador independente de restrições (`verify_plan`) para o gate M0.
pub fn plan_checker<'a>(
    catalog: &'a Catalog,
    soil: &'a Soil,
    plans: &'a [Plan],
) -> impl Fn(usize) -> Vec<String> + 'a {
    move |i| match verify_plan(catalog, soil, &plans[i], 0.20) {
        Ok(v) => v,
        Err(e) => vec![format!("verificador falhou: {e}")],
    }
}
