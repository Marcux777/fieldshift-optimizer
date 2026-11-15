// AI-assisted (OpenBot agent)
// Pure domain functions over the API JSON (/v1/evaluate, /v1/run). No UI, no I/O, no side effects.
// Every function returns null when a policy is infeasible or a value it needs is null.
import type { EvalReport, ObjectiveRow } from '../api/client';

export type PolicyKey = 'P0' | 'P1' | 'P1b' | 'P2';
export const POLICY_KEYS: readonly PolicyKey[] = ['P0', 'P1', 'P1b', 'P2'];

/** Paired-comparison tie margin ε (docs/EVALUATION_PROTOCOL.md §3). */
export const EPSILON = 0.02;

const isNum = (v: unknown): v is number => typeof v === 'number' && Number.isFinite(v);

function feasibleRegret(report: EvalReport, key: PolicyKey): number[] | null {
  const p = report.policies[key];
  if (!p?.feasible || !Array.isArray(p.per_scenario_regret)) return null;
  return p.per_scenario_regret.every(isNum) ? p.per_scenario_regret : null;
}

// ----------------------------------------------------------------------------- robustnessTradeoff

export type RobustnessTradeoff = {
  readonly p1MaxRegret: number;
  readonly p2MaxRegret: number;
  /** (M1(P1) − M1(P2)) / M1(P1) × 100 */
  readonly reductionPct: number;
  /** M5: L(P2, s̄) − L(P1, s̄), extra loss in the construction mean climate */
  readonly price: number;
  /** P2 vs P1 from the API comparison (D = R(P2) − R(P1); win D < −ε, tie |D| ≤ ε, loss D > ε) */
  readonly vsP1: { readonly wins: number; readonly ties: number; readonly losses: number };
  readonly classification: string;
};

export function robustnessTradeoff(report: EvalReport): RobustnessTradeoff | null {
  const p1 = report.policies.P1;
  const p2 = report.policies.P2;
  if (!p1?.feasible || !p2?.feasible) return null;
  const p1M1 = p1.M1_max_regret;
  const p2M1 = p2.M1_max_regret;
  const price = report.M5_price_of_robustness;
  const cmp = report.comparisons.find((c) => c.a === 'P2' && c.b === 'P1');
  if (!isNum(p1M1) || !isNum(p2M1) || !isNum(price) || !cmp || p1M1 === 0) return null;
  return {
    p1MaxRegret: p1M1,
    p2MaxRegret: p2M1,
    reductionPct: ((p1M1 - p2M1) / p1M1) * 100,
    price,
    vsP1: { wins: cmp.wins, ties: cmp.ties, losses: cmp.losses },
    classification: cmp.classification
  };
}

// ----------------------------------------------------------------------------- yearByYear

export type YearPair = {
  readonly index: number;
  readonly label: string;
  readonly dry: boolean;
  readonly regret: Readonly<Record<PolicyKey, number | null>>;
};

export type YearByYear = {
  readonly pairs: readonly YearPair[];
  /** Pair with the largest regret per policy (first one on ties); null if the policy is infeasible. */
  readonly worst: Readonly<Record<PolicyKey, { label: string; regret: number } | null>>;
};

export function yearByYear(report: EvalReport): YearByYear | null {
  const labels = report.test;
  if (!Array.isArray(labels) || labels.length === 0) return null;
  const dry = new Set(report.dry_test_scenarios ?? []);
  const series = Object.fromEntries(
    POLICY_KEYS.map((k) => {
      const r = feasibleRegret(report, k);
      return [k, r && r.length === labels.length ? r : null];
    })
  ) as Record<PolicyKey, number[] | null>;

  const pairs = labels.map((label, index) => ({
    index,
    label,
    dry: dry.has(label),
    regret: Object.fromEntries(POLICY_KEYS.map((k) => [k, series[k]?.[index] ?? null])) as Record<PolicyKey, number | null>
  }));
  const worst = Object.fromEntries(
    POLICY_KEYS.map((k) => {
      const r = series[k];
      if (!r) return [k, null];
      const i = r.indexOf(Math.max(...r));
      return [k, { label: labels[i], regret: r[i] }];
    })
  ) as Record<PolicyKey, { label: string; regret: number } | null>;
  return { pairs, worst };
}

// ----------------------------------------------------------------------------- sowingGrid

export type SowingCell = {
  readonly planId: readonly number[];
  /** decendio ordinals (dec − 25) mod 36 for J1 and J3 */
  readonly d1: number;
  readonly d3: number;
  readonly meanExposure: number;
  readonly worstExposure: number;
};

export type SowingGrid = {
  /** crop indices at ID positions 0, 2, 3, 5 */
  readonly signature: readonly [number, number, number, number];
  /** distinct year-1 / year-2 sowing ordinals that have at least one feasible plan, ascending */
  readonly d1: readonly number[];
  readonly d3: readonly number[];
  /** feasible plans only: a (d1, d3) pair without a cell is infeasible under ZARC ≤ 20% / calendar */
  readonly cells: readonly SowingCell[];
};

/**
 * Feasible plans that share the crops of `planId` (positions 0, 2, 3, 5 of `(c1, d1, c2, c3, d3, c4)`),
 * laid out by sowing period. `rows` are `parseObjectiveRows(run.objectives)`: every feasible plan.
 */
export function sowingGrid(rows: readonly ObjectiveRow[], planId: readonly number[] | null | undefined): SowingGrid | null {
  if (!planId || planId.length !== 6 || !planId.every(isNum)) return null;
  const sig: [number, number, number, number] = [planId[0], planId[2], planId[3], planId[5]];
  const cells = rows
    .filter((r) => r.planId[0] === sig[0] && r.planId[2] === sig[1] && r.planId[3] === sig[2] && r.planId[5] === sig[3])
    .map((r) => ({
      planId: r.planId,
      d1: r.planId[1],
      d3: r.planId[4],
      meanExposure: r.meanExposure,
      worstExposure: r.worstExposure
    }));
  if (cells.length === 0) return null;
  const uniq = (v: number[]) => [...new Set(v)].sort((a, b) => a - b);
  return { signature: sig, d1: uniq(cells.map((c) => c.d1)), d3: uniq(cells.map((c) => c.d3)), cells };
}

// ----------------------------------------------------------------------------- lossShares

export type LossShares = {
  /** w1 · mean exposure (g1), w2 · cover loss (g2), w3 · diversity loss (g3), over construction pairs */
  readonly exposure: number;
  readonly coverage: number;
  readonly diversity: number;
  /** = the mean weighted construction loss of the plan (metrics_construction.mean_L in /v1/run) */
  readonly total: number;
  /** fractions of `total` (all 0 when total is 0) */
  readonly share: { readonly exposure: number; readonly coverage: number; readonly diversity: number };
};

/**
 * Weighted parts of the loss L = w1·g1 + w2·g2 + w3·g3 averaged over the construction pairs. The loss is
 * linear, so mean(L) = w1·mean(g1) + w2·mean(g2) + w3·mean(g3); g1 uses the mean (not the worst) exposure.
 */
export function lossShares(row: ObjectiveRow | null | undefined, weights: readonly number[] | null | undefined): LossShares | null {
  if (!row || !weights || weights.length !== 3 || !weights.every(isNum)) return null;
  if (![row.meanExposure, row.coverageLoss, row.diversityLoss].every(isNum)) return null;
  const exposure = weights[0] * row.meanExposure;
  const coverage = weights[1] * row.coverageLoss;
  const diversity = weights[2] * row.diversityLoss;
  const total = exposure + coverage + diversity;
  const frac = (v: number) => (total > 0 ? v / total : 0);
  return {
    exposure,
    coverage,
    diversity,
    total,
    share: { exposure: frac(exposure), coverage: frac(coverage), diversity: frac(diversity) }
  };
}
