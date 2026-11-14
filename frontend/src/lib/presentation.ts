// AI-assisted (OpenBot agent)
// Presentation-only helpers: they read what the Rust API already returns and never change it.
import {
  cropDisplayName,
  dateLabelFromT,
  decStartT,
  parseObjectiveRows,
  type Catalog,
  type EvalReport,
  type RunResult
} from './api/client';

type ReadonlyPlanId = readonly number[];
type Alternative = RunResult['alternatives'][number];

/** Same rule as `Inputs::validate` in backend/crates/fieldshift-api/src/engine.rs. */
export const WEIGHT_SUM_TOLERANCE = 1e-12;

export type WeightCheck =
  | { readonly kind: 'ok'; readonly sum: number }
  | { readonly kind: 'invalid'; readonly sum: number; readonly message: string };

export function checkWeights(weights: readonly number[]): WeightCheck {
  // Left-to-right sum, like Rust's `iter().sum::<f64>()`.
  const sum = weights.reduce((acc, w) => acc + w, 0);
  if (weights.length !== 3 || weights.some((w) => !Number.isFinite(w) || w < 0 || w > 1)) {
    return { kind: 'invalid', sum, message: 'Each weight must be a number between 0 and 1.' };
  }
  if (Math.abs(sum - 1) > WEIGHT_SUM_TOLERANCE) {
    return {
      kind: 'invalid',
      sum,
      message: `Weights add up to ${sum.toFixed(2)}; they must add up to exactly 1 (for example 0.50 / 0.25 / 0.25).`
    };
  }
  return { kind: 'ok', sum };
}

// ----------------------------------------------------------------------------- plan IDs

/** Plan IDs store sowing as `(dec - 25) mod 36` (backend catalog.rs `dec_ordinal`). */
export function decendioFromOrdinal(ordinal: number): number {
  return ((((ordinal + 24) % 36) + 36) % 36) + 1;
}

export type PlanStep = {
  readonly window: 'J1' | 'J2' | 'J3' | 'J4';
  readonly year: 1 | 2;
  readonly cropKey: string;
  readonly cropName: string;
  /** ZARC sowing decendio for main crops (J1, J3); second crops are sown after the harvest. */
  readonly decendio: number | null;
  readonly sowing: string | null;
};

/** Plan ID layout `(c1, d1, c2, c3, d3, c4)`: crop indices into `catalog.crops`, d = decendio ordinal. */
export function describePlan(planId: ReadonlyPlanId, catalog: Catalog | null): PlanStep[] {
  const crop = (idx: number): { key: string; name: string } => {
    const key = catalog?.crops[idx]?.key ?? `crop ${idx}`;
    return { key, name: cropDisplayName(key) };
  };
  const main = (cropIdx: number, ordinal: number) => {
    const dec = ordinal >= 0 ? decendioFromOrdinal(ordinal) : null;
    return { dec, sowing: dec === null ? null : dateLabelFromT(decStartT(dec)) };
  };
  const c1 = crop(planId[0]);
  const c2 = crop(planId[2]);
  const c3 = crop(planId[3]);
  const c4 = crop(planId[5]);
  const s1 = main(planId[0], planId[1]);
  const s3 = main(planId[3], planId[4]);
  return [
    { window: 'J1', year: 1, cropKey: c1.key, cropName: c1.name, decendio: s1.dec, sowing: s1.sowing },
    { window: 'J2', year: 1, cropKey: c2.key, cropName: c2.name, decendio: null, sowing: null },
    { window: 'J3', year: 2, cropKey: c3.key, cropName: c3.name, decendio: s3.dec, sowing: s3.sowing },
    { window: 'J4', year: 2, cropKey: c4.key, cropName: c4.name, decendio: null, sowing: null }
  ];
}

export function planSequenceLabel(planId: ReadonlyPlanId, catalog: Catalog | null): string {
  const s = describePlan(planId, catalog);
  const fmt = (p: PlanStep) => (p.sowing ? `${p.cropName} (sown ${p.sowing})` : p.cropName);
  return `Year 1: ${fmt(s[0])} → ${fmt(s[1])} · Year 2: ${fmt(s[2])} → ${fmt(s[3])}`;
}

export function planKey(planId: ReadonlyPlanId): string {
  return planId.join(',');
}

// ----------------------------------------------------------------------------- alternatives

/** `policy_plan_ids` is in the /v1/run JSON but not in the generated OpenAPI type. */
export function policyPlanIds(alt: Alternative): Record<string, number[]> {
  const raw = (alt as unknown as Record<string, unknown>).policy_plan_ids;
  const out: Record<string, number[]> = {};
  if (typeof raw !== 'object' || raw === null || Array.isArray(raw)) return out;
  for (const [label, id] of Object.entries(raw as Record<string, unknown>)) {
    if (Array.isArray(id) && id.length === 6 && id.every((v) => typeof v === 'number' && Number.isFinite(v))) {
      out[label] = id as number[];
    }
  }
  return out;
}

export type AlternativeRow = {
  /** 1-based crop-sequence group (one per API alternative). */
  readonly group: number;
  readonly groupSize: number;
  readonly planId: readonly number[];
  readonly policies: readonly string[];
  readonly reason: string;
  readonly onFront: boolean;
  readonly minHammingToOthers: number;
  readonly hammingRelaxed: boolean;
  readonly objectives: readonly [number, number, number, number] | null;
};

export function frontKeySet(run: RunResult): Set<string> {
  const keys = new Set<string>();
  for (const id of run.pareto?.front_ids ?? []) keys.add(planKey(id));
  for (const ids of Object.values(run.pareto?.equivalents ?? {})) {
    for (const id of ids) keys.add(planKey(id));
  }
  return keys;
}

/**
 * The API groups alternatives by crop sequence (one crop per window) and ignores sowing dates
 * (backend pareto.rs `select_alternatives`). This splits each group into one row per distinct plan.
 */
export function expandAlternatives(run: RunResult): AlternativeRow[] {
  const objectives = new Map<string, [number, number, number, number]>();
  for (const r of parseObjectiveRows(run.objectives)) {
    objectives.set(planKey(r.planId), [r.meanExposure, r.worstExposure, r.coverageLoss, r.diversityLoss]);
  }
  const front = frontKeySet(run);
  const rows: AlternativeRow[] = [];
  run.alternatives.forEach((alt, idx) => {
    const ids = policyPlanIds(alt);
    const members: Array<{ id: readonly number[]; policies: string[] }> = [];
    const labels = alt.labels.length > 0 ? alt.labels : [''];
    for (const label of labels) {
      const id = (label && ids[label]) || alt.plan_id;
      const existing = members.find((m) => planKey(m.id) === planKey(id));
      if (existing) {
        if (label) existing.policies.push(label);
      } else {
        members.push({ id, policies: label ? [label] : [] });
      }
    }
    for (const m of members) {
      rows.push({
        group: idx + 1,
        groupSize: members.length,
        planId: m.id,
        policies: m.policies,
        reason: alt.reason,
        onFront: front.has(planKey(m.id)),
        minHammingToOthers: alt.min_hamming_to_others,
        hammingRelaxed: alt.hamming_relaxed,
        objectives: objectives.get(planKey(m.id)) ?? null
      });
    }
  });
  return rows;
}

export const ALTERNATIVE_REASON_LABELS: Readonly<Record<string, string>> = {
  policy: 'Policy plan',
  extreme_g1: 'Lowest mean exposure on the front',
  extreme_g2: 'Lowest coverage loss on the front',
  extreme_g3: 'Lowest diversity loss on the front',
  spread: 'Most different remaining front plan'
};

// ----------------------------------------------------------------------------- classification

/** English rendering of docs/EVALUATION_PROTOCOL.md v1.1, section 3 ("Comparação pareada", "Classificação"). */
export const CLASSIFICATION_RULE = {
  source: 'docs/EVALUATION_PROTOCOL.md, protocol v1.1, section 3 "Classificação"',
  d: 'For each held-out test pair s, D = R(P2, s) − R(Pk, s), where Pk is the comparator (P1b or P1). Positive D means P2 had more regret. |D| ≤ ε = 0.02 counts as a tie. The CI is the 95% block-bootstrap interval (blocks of 2 pairs, 10,000 replicates) of the median of D.',
  supported:
    'Supported: the 95% CI of D is entirely below 0, the M1 of P2 is lower than the comparator’s by at least 10% (relative) and at least ε, and M0 = 100%.',
  negative:
    'Negative: the 95% CI of D is entirely above 0, or the M1 of P2 is greater than or equal to the comparator’s.',
  inconclusive: 'Inconclusive: every other case, including when both policies choose identical plans.',
  whyNotM1:
    'A lower maximum regret (M1) alone is not enough. “Supported” also requires the whole CI of D below 0 and an M1 reduction of at least 10% and at least ε. If the CI is entirely above 0, P2 had more regret than the comparator in the paired test years, and the rule classifies it as Negative even when P2’s single worst year is better.'
} as const;

const COMPARISON_REASONS: Readonly<Record<string, string>> = {
  'IC 95% inteiramente > 0':
    'Negative because the 95% CI of D is entirely above 0 (P2 had higher regret in the paired test years).',
  'M1(P2) ≥ M1 do comparador':
    "Negative because P2's maximum test regret (M1) is greater than or equal to the comparator's.",
  'planos idênticos: a robustez não mudou a decisão':
    'Inconclusive because both policies chose identical plans (robustness did not change the decision).',
  'IC indisponível': 'Inconclusive because the confidence interval is unavailable.',
  'IC 95% < 0 e M1 cai ≥10% e ≥ε':
    'Supported because the 95% CI of D is entirely below 0 and M1 drops by at least 10% and at least ε.',
  'IC cruza 0 ou ganho de M1 abaixo do limiar':
    'Inconclusive because the 95% CI of D crosses 0 or the M1 reduction is below the threshold.'
};

/** Reasons emitted by `classify` in backend/crates/fieldshift-eval/src/stats.rs. */
export function comparisonReasonLabel(reason: string): string {
  return COMPARISON_REASONS[reason] ?? reason;
}

// ----------------------------------------------------------------------------- sensitivity

export type EvalSensitivityRow = {
  readonly gridIndex: number;
  readonly weights: readonly number[];
  readonly isDefault: boolean;
  readonly m1: Readonly<Record<'P1' | 'P1b' | 'P2', number | null>>;
  readonly p2VsP1b: string | null;
  readonly p2VsP1: string | null;
};

function rec(v: unknown): Record<string, unknown> {
  return typeof v === 'object' && v !== null && !Array.isArray(v) ? (v as Record<string, unknown>) : {};
}

function num(v: unknown): number | null {
  return typeof v === 'number' && Number.isFinite(v) ? v : null;
}

export function parseEvalSensitivity(raw: EvalReport['sensitivity'] | undefined): EvalSensitivityRow[] {
  const rows: EvalSensitivityRow[] = [];
  for (const item of raw ?? []) {
    const r = rec(item);
    const weights = Array.isArray(r.weights) ? r.weights.filter((w): w is number => typeof w === 'number') : [];
    if (weights.length !== 3) continue;
    const m1 = rec(r.M1);
    const cls = (k: string) => {
      const c = rec(r[k]).classification;
      return typeof c === 'string' ? c : null;
    };
    rows.push({
      gridIndex: num(r.grid_index) ?? rows.length + 1,
      weights,
      isDefault: r.is_default === true,
      m1: { P1: num(m1.P1), P1b: num(m1.P1b), P2: num(m1.P2) },
      p2VsP1b: cls('P2_vs_P1b'),
      p2VsP1: cls('P2_vs_P1')
    });
  }
  return rows;
}

export function sameWeights(a: readonly number[], b: readonly number[]): boolean {
  return a.length === b.length && a.every((v, i) => Math.abs(v - b[i]) <= 1e-12);
}

// ----------------------------------------------------------------------------- chart scales

export function paddedDomain(values: readonly number[], padFraction = 0.12): [number, number] {
  const finite = values.filter((v) => Number.isFinite(v));
  if (finite.length === 0) return [0, 1];
  const min = Math.min(...finite);
  const max = Math.max(...finite);
  const span = max - min > 0 ? max - min : Math.max(Math.abs(max) * 0.1, 0.01);
  const lo = Math.max(0, min - span * padFraction);
  const hi = Math.min(1, max + span * padFraction);
  return hi > lo ? [lo, hi] : [Math.max(0, lo - 0.01), Math.min(1, hi + 0.01)];
}

export function linearTicks(lo: number, hi: number, count = 5): number[] {
  return Array.from({ length: count }, (_, i) => lo + ((hi - lo) * i) / (count - 1));
}

// Viridis samples (colorblind-friendly), darkest = highest value.
const COLOR_STOPS = ['#b5de2b', '#35b779', '#26828e', '#3e4989', '#440154'] as const;

function hexToRgb(hex: string): [number, number, number] {
  const n = Number.parseInt(hex.slice(1), 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

/** Sequential color scale on t in [0, 1] (clamped). */
export function colorScale(t: number): string {
  const x = Math.min(1, Math.max(0, Number.isFinite(t) ? t : 0)) * (COLOR_STOPS.length - 1);
  const i = Math.min(COLOR_STOPS.length - 2, Math.floor(x));
  const f = x - i;
  const a = hexToRgb(COLOR_STOPS[i]);
  const b = hexToRgb(COLOR_STOPS[i + 1]);
  const c = a.map((v, k) => Math.round(v + (b[k] - v) * f));
  return `rgb(${c[0]}, ${c[1]}, ${c[2]})`;
}

// ----------------------------------------------------------------------------- glossary

export type GlossaryEntry = {
  readonly id: string;
  readonly term: string;
  readonly definition: string;
  readonly source: string;
};

/** Definitions restate the frozen protocol and project docs; `source` names the passage. */
export const GLOSSARY: readonly GlossaryEntry[] = [
  {
    id: 'policies',
    term: 'P0, P1, P1b, P2',
    definition:
      'The four strategies compared under identical data and constraints. P0: the current plan entered by the user, never tuned. P1 (mean climate): lowest loss in the average construction climate. P1b (expected loss): lowest average loss across the construction scenarios. P2 (minimax regret): see below.',
    source: 'docs/EVALUATION_PROTOCOL.md §2'
  },
  {
    id: 'regret',
    term: 'Regret',
    definition:
      'How much worse a plan is than the best feasible plan in the same two-year climate scenario: R(x, s) = L(x, s) − min L(x′, s). L is the weighted loss w₁·climate exposure + w₂·cover loss + w₃·diversity loss, each on a 0–1 scale. Regret is never negative.',
    source: 'docs/EVALUATION_PROTOCOL.md §1'
  },
  {
    id: 'minimax-regret',
    term: 'Minimax regret',
    definition:
      'The P2 rule: choose the plan whose largest regret over the 28 construction scenarios is the smallest. Ties are broken by lower mean regret, then by plan ID. It does not guarantee better results in the test years.',
    source: 'docs/EVALUATION_PROTOCOL.md §1'
  },
  {
    id: 'm1',
    term: 'Maximum regret (M1)',
    definition:
      'The largest regret of a policy across the held-out test scenarios. It is the primary metric; lower is better.',
    source: 'docs/EVALUATION_PROTOCOL.md §3'
  },
  {
    id: 'm0-m5',
    term: 'M0–M5',
    definition:
      'M0: share of P1, P1b and P2 plans that pass the feasibility check (must be 100%). M1: maximum test regret. M2: mean and median test regret. M3: loss and regret in dry test pairs (Oct–Mar rain below the construction 25th percentile; descriptive only if fewer than 3). M4: stability — (a) share of pairs with regret ≤ ε = 0.02, (b) interquartile range of loss, (c) LOO Hamming. M5: price of robustness, L(P2) − L(P1) in the mean construction climate.',
    source: 'docs/EVALUATION_PROTOCOL.md §3'
  },
  {
    id: 'm4c',
    term: 'LOO Hamming (M4c)',
    definition:
      'Stability of a policy’s choice: re-select the plan leaving one construction pair out at a time and count how many of the four windows change crop or sowing date versus the plan chosen with all pairs; report the mean. Uses construction data only; lower means more stable.',
    source: 'docs/EVALUATION_PROTOCOL.md §3 (M4c); backend/crates/fieldshift-eval/src/evaluation.rs'
  },
  {
    id: 'held-out',
    term: 'Held-out test',
    definition:
      'Agricultural years 2010/11–2024/25 (14 overlapping two-year pairs; 7 non-overlapping for the Wilcoxon test). They are never used to choose plans or weights, only to measure them. Plans are chosen with the construction years 1981/82–2009/10 (28 pairs).',
    source: 'docs/EVALUATION_PROTOCOL.md §1'
  },
  {
    id: 'pareto',
    term: 'Pareto front',
    definition:
      'Plans that no other plan matches or beats on all four construction objectives with at least one strictly better (mean exposure, worst exposure, coverage loss, diversity loss; lower is better). Plans with identical objective values are counted once.',
    source: 'docs/CONTRACT.md; backend/crates/fieldshift-model/src/pareto.rs'
  },
  {
    id: 'zarc',
    term: 'ZARC',
    definition:
      'Brazil’s Agricultural Climate Risk Zoning (Zoneamento Agrícola de Risco Climático, MAPA). The tool uses the 2026/27 risk table for Rio Verde: allowed sowing periods (decendios) with at most 20% risk per crop, cycle group and soil. It is a fixed constraint, never relaxed, and not a compliance check for insurance or credit.',
    source: 'docs/DATA_SOURCES.md; docs/EVALUATION_PROTOCOL.md §2, §5'
  },
  {
    id: 'decendio',
    term: 'Decendio',
    definition:
      'A ten-day sowing period (1–36 in the year: days 1–10, 11–20 and 21 to month end). Main crops (J1, J3) are sown in a ZARC decendio; second crops (J2, J4) are sown after the main crop’s harvest.',
    source: 'docs/DATA_SOURCES.md; backend/crates/fieldshift-model/src/catalog.rs'
  }
];
