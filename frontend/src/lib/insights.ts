// AI-assisted (OpenBot agent)
// Five "insight" pieces built only from fields the Rust API already returns (/v1/run, /v1/evaluate,
// /v1/catalog). Nothing here changes a result; it re-reads it. Every number shown in the UI comes from
// these functions, which read the API JSON directly.
import {
  cropDisplayName,
  dateLabelFromT,
  decStartT,
  formatOutcomeLabel,
  parseObjectiveRows,
  type Catalog,
  type EvalReport,
  type RunResult
} from './api/client';
import { comparisonReasonLabel, sameWeights } from './presentation';

type Comparison = EvalReport['comparisons'][number];
export type PolicyKey = 'P0' | 'P1' | 'P1b' | 'P2';
export const POLICIES: readonly PolicyKey[] = ['P0', 'P1', 'P1b', 'P2'];

const f4 = (v: number) => v.toFixed(4);

// ----------------------------------------------------------------------------- paired rule

/**
 * Win/tie/loss rule of the evaluator (backend/crates/fieldshift-eval/src/evaluation.rs, `Comparison`):
 * D = R(P2, s) − R(Pk, s) per held-out pair; win if D < −ε, tie if |D| ≤ ε, loss if D > ε, with
 * ε = 0.02 (docs/EVALUATION_PROTOCOL.md §3, "Comparação pareada").
 */
export const PAIRED_RULE = {
  epsilon: 0.02,
  text: 'Per test pair, D = regret(P2) − regret(comparator). Win if D < −0.02, tie if |D| ≤ 0.02, loss if D > 0.02 (protocol v1.1 §3; same rule as the evaluator).'
} as const;

export function pairedOutcome(a: readonly number[], b: readonly number[], eps: number = PAIRED_RULE.epsilon) {
  const d = a.map((v, i) => v - b[i]);
  return {
    wins: d.filter((v) => v < -eps).length,
    ties: d.filter((v) => Math.abs(v) <= eps).length,
    losses: d.filter((v) => v > eps).length
  };
}

// ----------------------------------------------------------------------------- piece 1

/** "Negative because X." -> "X." so it reads after "is also NEGATIVE because". */
function reasonClause(reason: string): string {
  const label = comparisonReasonLabel(reason);
  const m = /^(?:Negative|Supported|Inconclusive) because (.*)$/.exec(label);
  return m ? m[1] : label;
}

type PolicyNumbers = { readonly m1: number; readonly lossMeanClimate: number };

function policyNumbers(report: EvalReport, key: PolicyKey): PolicyNumbers | null {
  const p = report.policies[key];
  if (!p?.feasible || p.M1_max_regret === null || p.loss_mean_climate === null) return null;
  return { m1: p.M1_max_regret, lossMeanClimate: p.loss_mean_climate };
}

function comparison(report: EvalReport, b: string): Comparison | null {
  return report.comparisons.find((c) => c.a === 'P2' && c.b === b) ?? null;
}

export function protectionStory(report: EvalReport) {
  const p1 = policyNumbers(report, 'P1');
  const p1b = policyNumbers(report, 'P1b');
  const p2 = policyNumbers(report, 'P2');
  const primary = comparison(report, 'P1b');
  const secondary = comparison(report, 'P1');
  const m5 = report.M5_price_of_robustness;
  if (!p1 || !p1b || !p2 || !primary || !secondary || m5 === null) return null;

  const vsP1 = pairedOutcome(report.policies.P2.per_scenario_regret, report.policies.P1.per_scenario_regret);
  const vsP1Consistent = vsP1.wins === secondary.wins && vsP1.ties === secondary.ties && vsP1.losses === secondary.losses;
  const worstYearReductionVsP1Pct = ((p1.m1 - p2.m1) / p1.m1) * 100;
  const p2BeatsP1bWorstYear = p2.m1 < p1b.m1;
  const nPairs = report.policies.P2.per_scenario_regret.length;

  const primaryLabel = formatOutcomeLabel(primary.classification);
  const secondaryLabel = formatOutcomeLabel(secondary.classification);
  const text = {
    primary:
      `Primary comparison of the protocol: P2 vs P1b. P2's worst test year (M1) was ${f4(p2.m1)} against ${f4(p1b.m1)} for P1b, ` +
      `so minimax regret ${p2BeatsP1bWorstYear ? 'reduced' : 'did not reduce'} the maximum regret. Classification: ${primaryLabel}.`,
    secondary:
      `Secondary comparison: against the mean-climate plan P1, P2 ` +
      (worstYearReductionVsP1Pct > 0
        ? `lowered the worst test year from ${f4(p1.m1)} to ${f4(p2.m1)} (${Math.round(worstYearReductionVsP1Pct)}% less).`
        : `did not lower the worst test year (${f4(p2.m1)} vs ${f4(p1.m1)}).`),
    tradeoff:
      `What it cost: ${f4(m5)} more loss in the average climate (M5), and across the ${nPairs} test pairs P2 had ` +
      `${vsP1.wins} wins, ${vsP1.ties} ties and ${vsP1.losses} losses against P1. ` +
      `That comparison is also ${secondaryLabel} because ${reasonClause(secondary.reason)} ` +
      (p2BeatsP1bWorstYear
        ? `P1b's worst test year was ${f4(p1b.m1)}.`
        : `P1b, not P2, had the smaller worst test year (${f4(p1b.m1)}).`)
  };
  return {
    p1,
    p1b,
    p2,
    m5,
    primary,
    secondary,
    vsP1,
    vsP1Consistent,
    worstYearReductionVsP1Pct,
    p2BeatsP1bWorstYear,
    text
  };
}

// ----------------------------------------------------------------------------- piece 2

/**
 * Premium = extra loss in the construction mean climate versus P1 (for P2 this is exactly M5,
 * L(x_MMR, s̄) − L(x_CONV, s̄); for P1b the same difference is computed from `loss_mean_climate`).
 * Protection = reduction of the worst held-out year (M1) versus P1.
 */
export function premiumVsProtection(report: EvalReport) {
  const p1 = policyNumbers(report, 'P1');
  if (!p1) return [];
  return (['P2', 'P1b'] as const).flatMap((key) => {
    const p = policyNumbers(report, key);
    if (!p) return [];
    const premium = key === 'P2' && report.M5_price_of_robustness !== null ? report.M5_price_of_robustness : p.lossMeanClimate - p1.lossMeanClimate;
    return [{ policy: key, premium, protection: p1.m1 - p.m1, m1: p.m1, p1M1: p1.m1, isM5: key === 'P2' }];
  });
}

// ----------------------------------------------------------------------------- piece 3

function decOrdinal(dec: number): number {
  return (((dec - 25) % 36) + 36) % 36;
}

function policyIds(run: RunResult): Array<{ policy: PolicyKey; id: number[] }> {
  return POLICIES.flatMap((k) => {
    const id = run.policies[k]?.plan?.id;
    return id ? [{ policy: k, id: [...id] }] : [];
  });
}

const sigKey = (s: readonly number[]) => s.join(',');
const signatureOf = (id: readonly number[]) => [id[0], id[2], id[3], id[5]];

export function cropSequences(run: RunResult, catalog: Catalog | null) {
  const counts = new Map<string, { signature: number[]; nPlans: number }>();
  for (const r of parseObjectiveRows(run.objectives)) {
    const s = signatureOf(r.planId);
    const k = sigKey(s);
    const e = counts.get(k) ?? { signature: s, nPlans: 0 };
    e.nPlans += 1;
    counts.set(k, e);
  }
  const pol = policyIds(run);
  const crop = (i: number) => cropDisplayName(catalog?.crops[i]?.key ?? `crop ${i}`);
  return [...counts.values()]
    .map((e) => ({
      ...e,
      key: sigKey(e.signature),
      label: `${crop(e.signature[0])} → ${crop(e.signature[1])} · ${crop(e.signature[2])} → ${crop(e.signature[3])}`,
      policies: pol.filter((p) => sigKey(signatureOf(p.id)) === sigKey(e.signature)).map((p) => p.policy)
    }))
    .sort((a, b) => a.key.localeCompare(b.key, 'en', { numeric: true }));
}

export type GridCell = {
  readonly d1: number;
  readonly d3: number;
  readonly planId: number[] | null;
  /** mean construction climate exposure g1 (objectives column `mean_g1`); null = no feasible plan */
  readonly value: number | null;
  readonly worst: number | null;
  readonly policies: PolicyKey[];
};

/**
 * Grid "year-1 sowing (d1) × year-2 sowing (d3)" for one crop sequence (c1, c2, c3, c4). Each cell is the
 * plan ID (c1, d1, c2, c3, d3, c4); its colour is that plan's mean construction exposure (objectives[1]).
 * A missing ID is not in the feasible set (ZARC ≤ 20% / calendar) and stays null, never zero.
 */
export function sowingGrid(run: RunResult, catalog: Catalog | null, signature: readonly number[]) {
  const axis = (window: string) =>
    [...(catalog?.windows.find((w) => w.name === window)?.decendios ?? [25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36])]
      .map((dec) => ({ ordinal: decOrdinal(dec), decendio: dec, label: `${dec}: ${dateLabelFromT(decStartT(dec))}` }))
      .sort((a, b) => a.ordinal - b.ordinal);
  const d1 = axis('J1');
  const d3 = axis('J3');
  const byId = new Map(parseObjectiveRows(run.objectives).map((r) => [r.planId.join(','), r]));
  const pol = policyIds(run);
  const cells: GridCell[][] = d3.map((y) =>
    d1.map((x) => {
      const id = [signature[0], x.ordinal, signature[1], signature[2], y.ordinal, signature[3]];
      const row = byId.get(id.join(','));
      return {
        d1: x.ordinal,
        d3: y.ordinal,
        planId: row ? id : null,
        value: row ? row.meanExposure : null,
        worst: row ? row.worstExposure : null,
        policies: pol.filter((p) => p.id.join(',') === id.join(',')).map((p) => p.policy)
      };
    })
  );
  const values = cells.flat().flatMap((c) => (c.value === null ? [] : [c.value]));
  return {
    d1,
    d3,
    cells,
    min: values.length ? Math.min(...values) : 0,
    max: values.length ? Math.max(...values) : 0,
    feasibleCount: values.length,
    markers: cells.flat().flatMap((c) => c.policies.map((p) => ({ policy: p, d1: c.d1, d3: c.d3 })))
  };
}

// ----------------------------------------------------------------------------- piece 4

export type Gap = { readonly id: string; readonly text: string; readonly source: string };

/** docs/LIMITATIONS.md §2 restates the [N] hypotheses of docs/AGRONOMY.md; `§2.k` = item k of that list. */
const LIMITS = 'docs/LIMITATIONS.md §2';

/** Crop-specific [N] hypotheses; `crops` = catalog keys the hypothesis is applied to. */
const CROP_GAPS: ReadonlyArray<Gap & { readonly crops: readonly string[] }> = [
  { id: 'soy-cycle', crops: ['soja'], text: 'Soybean cycle assumed to be 100 days (Group I; inferred).', source: `${LIMITS}.1` },
  {
    id: 'phase-fractions',
    crops: ['soja', 'milho_2a', 'milho_2a_braquiaria', 'sorgo_2a'],
    text: 'Crop-stage fractions taken from first-season maize and applied to this crop.',
    source: `${LIMITS}.2`
  },
  { id: 'legal-window', crops: ['soja'], text: 'Sowing periods only partly inside the legal soybean window are excluded.', source: `${LIMITS}.4` },
  {
    id: 'harvest-rain-soy-sorghum',
    crops: ['soja', 'sorgo_2a'],
    text: 'Harvest-rain threshold documented for maize is also applied to soybean and sorghum.',
    source: `${LIMITS}.5`
  },
  { id: 'brachiaria-desiccation', crops: ['milho_2a_braquiaria'], text: 'Fixed 15-day brachiaria desiccation window before J3.', source: `${LIMITS}.5` }
];

/** [N] hypotheses that apply to every plan alike. */
export const GLOBAL_GAPS: readonly Gap[] = [
  { id: 'emergence', text: 'Emergence 7 days after sowing; no gap between main-crop harvest and second-crop sowing.', source: `${LIMITS}.3` },
  { id: 'thresholds', text: 'Indicator thresholds (20 mm in 10 days, 1 mm rainy day) and equal sub-indicator weights in g1.', source: `${LIMITS}.5` },
  { id: 'soil', text: 'No measured farm soil: AD4 + clay is a default assumption.', source: `${LIMITS}.6` },
  { id: 'static-zarc', text: 'The 2026/27 ZARC table is applied to every historical year.', source: `${LIMITS}.7` },
  { id: 'prectotcorr', text: 'NASA POWER corrected rainfall may use a climatology that overlaps the test years.', source: 'docs/LIMITATIONS.md §4' }
];

const DEMO_P0_GAP: Gap = {
  id: 'demo-p0',
  text: 'The demo current plan (soybean in decendio 29 + second-season maize, both years) is itself an assumption.',
  source: 'docs/EVALUATION_PROTOCOL.md §2'
};

export function evidenceGapsRisks(run: RunResult, report: EvalReport, catalog: Catalog | null, policy: PolicyKey) {
  const entry = run.policies[policy];
  const metrics = report.policies[policy];
  const planId = entry?.plan?.id ?? null;
  const crops = new Set((planId ? [planId[0], planId[2], planId[3], planId[5]] : []).map((i) => catalog?.crops[i]?.key ?? ''));
  const planSpecific: Gap[] = CROP_GAPS.filter((g) => g.crops.some((c) => crops.has(c))).map(({ id, text, source }) => ({ id, text, source }));
  const isDemoP0 =
    policy === 'P0' &&
    planId !== null &&
    catalog !== null &&
    entry?.plan?.occupations.map((o) => o.crop).join(',') ===
      [catalog.default_p0.j1, catalog.default_p0.j2, catalog.default_p0.j3, catalog.default_p0.j4].join(',');
  if (isDemoP0) planSpecific.unshift(DEMO_P0_GAP);

  let weightChanges: { changed: number; total: number } | null = null;
  if (policy !== 'P0') {
    const rows = run.sensitivity ?? [];
    const def = rows.find((r) => sameWeights(r.weights, run.weights)) ?? null;
    if (def) {
      const base = def[policy].join(',');
      weightChanges = { changed: rows.filter((r) => r[policy].join(',') !== base).length, total: rows.length };
    }
  }

  return {
    policy,
    feasible: Boolean(metrics?.feasible && planId),
    evidence: {
      snapshotId: run.snapshot_id,
      protocolVersion: run.protocol_version,
      constructionPairs: report.construction.length,
      constructionRange: report.construction.length ? `${report.construction[0]} to ${report.construction[report.construction.length - 1]}` : '—',
      testPairs: report.test.length,
      testRange: report.test.length ? `${report.test[0]} to ${report.test[report.test.length - 1]}` : '—',
      excluded: report.excluded_scenarios.length,
      selection:
        policy === 'P0'
          ? 'Entered by the user; not selected from data.'
          : 'Selected on construction pairs only; test pairs used once, to measure.'
    },
    gaps: { planSpecific, global: GLOBAL_GAPS },
    risks: {
      worstYear: metrics?.M1_max_regret ?? null,
      dryLoss: metrics?.M3_dry_mean_loss ?? null,
      dryPairs: report.dry_test_scenarios.length,
      weightChanges
    }
  };
}

// ----------------------------------------------------------------------------- piece 5

export function replayPair(report: EvalReport, index: number) {
  const label = report.test[index];
  if (label === undefined) return null;
  const rows = POLICIES.flatMap((policy) => {
    const v = report.policies[policy]?.per_scenario_regret?.[index];
    return report.policies[policy]?.feasible && typeof v === 'number' ? [{ policy, regret: v }] : [];
  });
  const values = rows.map((r) => r.regret);
  const min = Math.min(...values);
  const max = Math.max(...values);
  return {
    index,
    label,
    dry: report.dry_test_scenarios.includes(label),
    rows: rows.map((r) => ({ ...r, lowest: r.regret === min, highest: r.regret === max }))
  };
}
