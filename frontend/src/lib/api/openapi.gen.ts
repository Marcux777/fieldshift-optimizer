// AI-assisted (OpenBot agent)
// AUTO-GENERATED FROM backend/crates/fieldshift-api/openapi.json — DO NOT EDIT MANUALLY.
// OpenAPI title: FieldShift computed scientific API (v1.0.0)

export type PlanId = Array<number>;

export type CurrentPlan = {
  readonly "j1": string;
  readonly "dec1": number;
  readonly "j2": string;
  readonly "j3": string;
  readonly "dec3": number;
  readonly "j4": string;
};

export type Inputs = {
  readonly "ad_class": "AD1" | "AD2" | "AD3" | "AD4" | "AD5" | "AD6";
  readonly "texture": "arenosa" | "media" | "argilosa";
  readonly "weights": Array<number>;
  readonly "p0"?: CurrentPlan | null;
};

export type ApiError = {
  readonly "error": {
    readonly "code": string;
    readonly "message": string;
  };
};

export type Health = {
  readonly "status": string;
  readonly "language": string;
  readonly "mode": string;
  readonly "recomputation_available": boolean;
};

export type Meta = {
  readonly "version": string;
  readonly "language": string;
  readonly "mode": string;
  readonly "protocol_version": string;
  readonly "snapshot_id": string;
  readonly "config_hash": string;
  readonly "protocol_sha256": string;
};

export type Crop = {
  readonly "key": string;
  readonly "label": string;
  readonly "windows": Array<string>;
  readonly "cycle_days": number;
};

export type Window = {
  readonly "name": string;
  readonly "crops": Array<string>;
  readonly "decendios": Array<number>;
};

export type Catalog = {
  readonly "crops": Array<Crop>;
  readonly "windows": Array<Window>;
  readonly "soils": Array<string>;
  readonly "textures": Array<string>;
  readonly "default_p0": CurrentPlan;
  readonly "weight_grid": Array<Array<number>>;
  readonly "risk_limit": number;
};

export type Occupation = {
  readonly "window": string;
  readonly "crop": string;
  readonly "label": string;
  readonly "sow": string | null;
  readonly "sow_t": number | null;
  readonly "harvest_t": number | null;
  readonly "zarc_dec": number | null;
};

export type Plan = {
  readonly "id": PlanId;
  readonly "occupations": Array<Occupation>;
};

export type Policy = {
  readonly "status": string;
  readonly "plan"?: Plan;
  readonly "reasons"?: Array<string>;
  readonly "metrics_construction"?: {
    readonly "L_sbar": number;
    readonly "mean_L": number;
    readonly "max_regret": number;
    readonly "mean_regret": number;
  };
};

export type Sensitivity = {
  readonly "weights": Array<number>;
  readonly "P1": PlanId;
  readonly "P1b": PlanId;
  readonly "P2": PlanId;
};

export type Alternative = {
  readonly "plan_id": PlanId;
  readonly "labels": Array<string>;
  readonly "on_front": boolean;
  readonly "min_hamming_to_others": number;
  readonly "reason": string;
  readonly "hamming_relaxed": boolean;
};

export type Pareto = {
  readonly "front_ids": Array<PlanId>;
  readonly "n_front": number;
  readonly "equivalents": Record<string, Array<PlanId>>;
  readonly "objectives": Array<string>;
};

export type RunResult = {
  readonly "status": "OK" | "INVIAVEL" | "DADOS_INSUFICIENTES";
  readonly "message": string;
  readonly "snapshot_id": string;
  readonly "config_hash": string;
  readonly "protocol_version": string;
  readonly "n_plans": number;
  readonly "weights": Array<number>;
  readonly "policies": Record<string, Policy>;
  readonly "objectives": Array<Array<Array<number> | number>>;
  readonly "pareto": Pareto;
  readonly "alternatives": Array<Alternative>;
  readonly "sensitivity": Array<Sensitivity>;
  readonly "result_hash": string;
};

export type PolicyMetrics = {
  readonly "policy": string;
  readonly "plan_id": PlanId | null;
  readonly "feasible": boolean;
  readonly "M1_max_regret": number | null;
  readonly "M2_mean_regret": number | null;
  readonly "M2_median_regret": number | null;
  readonly "M3_dry_mean_loss": number | null;
  readonly "M3_dry_mean_regret": number | null;
  readonly "M4a_frac_regret_le_eps": number | null;
  readonly "M4b_iqr_loss": number | null;
  readonly "M4c_loo_mean_hamming": number | null;
  readonly "mean_loss_test": number | null;
  readonly "loss_mean_climate": number | null;
  readonly "per_scenario_regret": Array<number>;
  readonly "per_scenario_loss": Array<number>;
};

export type Comparison = {
  readonly "a": string;
  readonly "b": string;
  readonly "classification": string;
  readonly "identical_plans": boolean;
  readonly "n": number;
  readonly "wins": number;
  readonly "ties": number;
  readonly "losses": number;
  readonly "median_D": number | null;
  readonly "mean_D": number | null;
  readonly "ci95_median_D": Array<number> | null;
  readonly "ci95_mean_D": Array<number> | null;
  readonly "m1_a": number | null;
  readonly "m1_b": number | null;
  readonly "wilcoxon_nonoverlap": Record<string, number> | null;
  readonly "reason": string;
};

export type EvalReport = {
  readonly "outcome": string;
  readonly "headline": string;
  readonly "n_plans": number;
  readonly "construction": Array<string>;
  readonly "test": Array<string>;
  readonly "excluded_scenarios": Array<Record<string, unknown>>;
  readonly "weights": Array<number>;
  readonly "M0_feasibility": number | null;
  readonly "M5_price_of_robustness": number | null;
  readonly "dry_threshold_mm": number | null;
  readonly "dry_test_scenarios": Array<string>;
  readonly "policies": Record<string, PolicyMetrics>;
  readonly "comparisons": Array<Comparison>;
  readonly "sensitivity": Array<Record<string, unknown>>;
  readonly "limitations": Array<string>;
  readonly "result_hash": string;
};

export type Plans = {
  readonly "status": string;
  readonly "n_plans": number;
  readonly "plans": Array<Plan>;
  readonly "rejections": Array<Record<string, unknown>>;
  readonly "diagnosis": Record<string, unknown>;
};

export type Provenance = {
  readonly "manifest": {
    readonly "snapshot_id": string;
    readonly "collected_utc": string;
    readonly "source": {
      readonly "api_version": string;
    };
    readonly "citation": {
      readonly "reference": string;
      readonly "data_reference": string;
      readonly "referencing_guide": string;
    };
    readonly "request": Record<string, unknown>;
    readonly "units": Record<string, string>;
  };
  readonly "quality": Record<string, unknown>;
  readonly "protocol_sha256": string;
  readonly "source": string;
};

export type Limitations = {
  readonly "items": Array<string>;
};

export type ParetoResponse = {
  readonly "pareto": Pareto;
  readonly "objectives": Array<Array<Array<number> | number>>;
  readonly "alternatives": Array<Alternative>;
};

export interface OpenApiClientOptions {
  readonly baseUrl?: string;
  readonly fetchFn?: typeof fetch;
}

export class OpenApiHttpError extends Error {
  readonly status: number;
  readonly code: string;

  constructor(params: { readonly status: number; readonly code: string; readonly message: string }) {
    super(params.message);
    this.name = 'OpenApiHttpError';
    this.status = params.status;
    this.code = params.code;
  }
}

export interface GeneratedOpenApiClient {
  health(): Promise<Health>;
  meta(): Promise<Meta>;
  catalog(): Promise<Catalog>;
  plans(body: Inputs): Promise<Plans>;
  run(body: Inputs): Promise<RunResult>;
  evaluate(body: Inputs): Promise<EvalReport>;
  pareto(body: Inputs): Promise<ParetoResponse>;
  provenance(): Promise<Provenance>;
  limitations(): Promise<Limitations>;
}
