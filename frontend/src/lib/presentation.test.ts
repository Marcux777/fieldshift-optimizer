// AI-assisted (OpenBot agent)
import { describe, expect, it } from 'vitest';
import goldenEvalJson from '../../../fixtures/reference/golden_eval_report.json';
import goldenRunJson from '../../../fixtures/reference/golden_run_result.json';

import type { Catalog, EvalReport, RunResult } from './api/client';
import {
  CLASSIFICATION_RULE,
  GLOSSARY,
  WEIGHT_SUM_TOLERANCE,
  checkWeights,
  colorScale,
  comparisonReasonLabel,
  decendioFromOrdinal,
  describePlan,
  expandAlternatives,
  paddedDomain,
  parseEvalSensitivity,
  planSequenceLabel,
  policyPlanIds
} from './presentation';

const goldenRun = goldenRunJson as unknown as RunResult;
const goldenEval = goldenEvalJson as unknown as EvalReport;

// Same crop order and window decendios as GET /v1/catalog (AD4/argilosa, commit 98e3964).
const CATALOG: Catalog = {
  crops: [
    { key: 'soja', label: 'Soja (Grupo I)', windows: ['J1', 'J3'], cycle_days: 100 },
    { key: 'milho_1a', label: 'Milho 1ª safra (Grupo I)', windows: ['J1', 'J3'], cycle_days: 100 },
    { key: 'milho_2a', label: 'Milho 2ª safra (Grupo I)', windows: ['J2', 'J4'], cycle_days: 110 },
    {
      key: 'milho_2a_braquiaria',
      label: 'Milho 2ª safra + braquiária (Grupo I)',
      windows: ['J2', 'J4'],
      cycle_days: 115
    },
    { key: 'sorgo_2a', label: 'Sorgo granífero 2ª safra (Grupo I)', windows: ['J2', 'J4'], cycle_days: 110 },
    { key: 'pousio', label: 'Pousio', windows: ['J2', 'J4'], cycle_days: 0 }
  ],
  windows: [],
  soils: ['AD1', 'AD2', 'AD3', 'AD4', 'AD5', 'AD6'],
  textures: ['arenosa', 'media', 'argilosa'],
  default_p0: { j1: 'soja', dec1: 29, j2: 'milho_2a', j3: 'soja', dec3: 29, j4: 'milho_2a' },
  weight_grid: [],
  risk_limit: 0.2
};

describe('weight validation mirrors the Rust API rule', () => {
  it('uses the backend tolerance |sum - 1| <= 1e-12 with left-to-right summation', () => {
    expect(WEIGHT_SUM_TOLERANCE).toBe(1e-12);
    expect(checkWeights([0.5, 0.25, 0.25])).toEqual({ kind: 'ok', sum: 1 });
    expect(checkWeights([0.6, 0.3, 0.1]).kind).toBe('ok');
    expect(checkWeights([1, 0, 0]).kind).toBe('ok');
    // passes the old 1e-9 frontend check but the API rejects it
    expect(checkWeights([0.5, 0.25, 0.25 + 1e-10]).kind).toBe('invalid');
  });

  it('explains the actual sum inline', () => {
    expect(checkWeights([0.9, 0.25, 0.25])).toEqual({
      kind: 'invalid',
      sum: 1.4,
      message: 'Weights add up to 1.40; they must add up to exactly 1 (for example 0.50 / 0.25 / 0.25).'
    });
    expect(checkWeights([-0.5, 1, 0.5])).toEqual({
      kind: 'invalid',
      sum: 1,
      message: 'Each weight must be a number between 0 and 1.'
    });
    expect(checkWeights([Number.NaN, 0.5, 0.5]).kind).toBe('invalid');
  });
});

describe('plan IDs are decoded into crops and sowing dates', () => {
  it('maps the decendio ordinal used in plan IDs (dec - 25 mod 36) back to the decendio', () => {
    expect(decendioFromOrdinal(0)).toBe(25);
    expect(decendioFromOrdinal(4)).toBe(29);
    expect(decendioFromOrdinal(11)).toBe(36);
    expect(decendioFromOrdinal(12)).toBe(1);
  });

  it('decodes the default P0 [0,4,2,0,4,2]', () => {
    expect(describePlan([0, 4, 2, 0, 4, 2], CATALOG)).toEqual([
      { window: 'J1', year: 1, cropKey: 'soja', cropName: 'Soybean', decendio: 29, sowing: '11/Oct' },
      { window: 'J2', year: 1, cropKey: 'milho_2a', cropName: 'Second-season maize', decendio: null, sowing: null },
      { window: 'J3', year: 2, cropKey: 'soja', cropName: 'Soybean', decendio: 29, sowing: '11/Oct' },
      { window: 'J4', year: 2, cropKey: 'milho_2a', cropName: 'Second-season maize', decendio: null, sowing: null }
    ]);
    expect(planSequenceLabel([0, 4, 2, 0, 4, 2], CATALOG)).toBe(
      'Year 1: Soybean (sown 11/Oct) → Second-season maize · Year 2: Soybean (sown 11/Oct) → Second-season maize'
    );
  });

  it('agrees with the occupations the API returns for every policy plan', () => {
    for (const key of ['P0', 'P1', 'P1b', 'P2']) {
      const plan = goldenRun.policies[key]?.plan;
      if (!plan) continue;
      const steps = describePlan(plan.id, CATALOG);
      plan.occupations.forEach((occ, i) => {
        expect(steps[i].window).toBe(occ.window);
        expect(steps[i].cropKey).toBe(occ.crop);
        if (occ.window === 'J1' || occ.window === 'J3') {
          expect(steps[i].decendio).toBe(occ.zarc_dec);
        }
      });
    }
  });
});

describe('Pareto alternatives are expanded to one row per plan', () => {
  it('reads policy_plan_ids that the API sends but the generated type omits', () => {
    const grouped = goldenRun.alternatives[1];
    expect(grouped.labels).toEqual(['P1', 'P1b', 'P2']);
    expect(policyPlanIds(grouped)).toEqual({
      P1: [0, 3, 3, 1, 9, 4],
      P1b: [0, 5, 3, 1, 7, 4],
      P2: [0, 4, 3, 1, 7, 4]
    });
    expect(policyPlanIds(goldenRun.alternatives[2])).toEqual({});
  });

  it('splits the crop-sequence group P1/P1b/P2 into three plan rows with their own objectives', () => {
    const rows = expandAlternatives(goldenRun);
    expect(rows.map((r) => [r.group, r.policies.join('+'), r.planId.join(',')])).toEqual([
      [1, 'P0', '0,4,2,0,4,2'],
      [2, 'P1', '0,3,3,1,9,4'],
      [2, 'P1b', '0,5,3,1,7,4'],
      [2, 'P2', '0,4,3,1,7,4'],
      [3, '', '0,6,4,1,11,5'],
      [4, '', '0,4,3,0,4,3'],
      [5, '', '1,7,4,0,4,3'],
      [6, '', '0,6,4,1,6,4']
    ]);
    const p1 = rows[1];
    const p2 = rows[3];
    expect(p1.objectives).not.toBeNull();
    expect(p2.objectives).not.toBeNull();
    expect(p1.objectives).not.toEqual(p2.objectives);
    expect(rows[0].onFront).toBe(false);
    expect(rows.slice(1).every((r) => r.onFront)).toBe(true);
    expect(p2.groupSize).toBe(3);
  });
});

describe('classification is explained with the frozen protocol rule', () => {
  it('quotes protocol v1.1 section 3 and translates every reason the evaluator can emit', () => {
    expect(CLASSIFICATION_RULE.source).toBe('docs/EVALUATION_PROTOCOL.md, protocol v1.1, section 3 "Classificação"');
    expect(CLASSIFICATION_RULE.negative).toContain('entirely above 0');
    expect(CLASSIFICATION_RULE.negative).toContain('M1 of P2 is greater than or equal');
    expect(comparisonReasonLabel('IC 95% inteiramente > 0')).toBe(
      'Negative because the 95% CI of D is entirely above 0 (P2 had higher regret in the paired test years).'
    );
    expect(comparisonReasonLabel('M1(P2) ≥ M1 do comparador')).toBe(
      "Negative because P2's maximum test regret (M1) is greater than or equal to the comparator's."
    );
    for (const raw of [
      'planos idênticos: a robustez não mudou a decisão',
      'IC indisponível',
      'IC 95% < 0 e M1 cai ≥10% e ≥ε',
      'IC cruza 0 ou ganho de M1 abaixo do limiar'
    ]) {
      expect(comparisonReasonLabel(raw)).not.toBe(raw);
    }
    expect(comparisonReasonLabel('unknown reason')).toBe('unknown reason');
  });

  it('parses the held-out classification of all 15 weight settings', () => {
    const rows = parseEvalSensitivity(goldenEval.sensitivity);
    expect(rows).toHaveLength(15);
    const def = rows.find((r) => r.isDefault);
    expect(def?.weights).toEqual([0.5, 0.25, 0.25]);
    expect(def?.p2VsP1b).toBe('NEGATIVO');
    expect(rows[0].weights).toEqual([1, 0, 0]);
    expect(rows[0].m1.P2).toBeCloseTo(0.271784688575, 12);
  });
});

describe('Pareto chart scales fit the data', () => {
  it('pads the domain instead of using the full 0-1 range', () => {
    const [lo, hi] = paddedDomain([0.18, 0.2, 0.22]);
    expect(lo).toBeGreaterThan(0.1);
    expect(hi).toBeLessThan(0.3);
    expect(lo).toBeLessThan(0.18);
    expect(hi).toBeGreaterThan(0.22);
    const [a, b] = paddedDomain([0.5, 0.5]);
    expect(b - a).toBeGreaterThan(0);
  });

  it('maps the color scale endpoints to distinct colors', () => {
    expect(colorScale(0)).not.toBe(colorScale(1));
    expect(colorScale(-1)).toBe(colorScale(0));
    expect(colorScale(2)).toBe(colorScale(1));
  });
});

describe('glossary', () => {
  it('covers every requested term and cites a project document', () => {
    const terms = GLOSSARY.map((g) => g.id);
    for (const id of ['minimax-regret', 'regret', 'm1', 'm0-m5', 'm4c', 'zarc', 'pareto', 'policies', 'held-out']) {
      expect(terms).toContain(id);
    }
    for (const g of GLOSSARY) {
      expect(g.source).toMatch(/^docs\/|^backend\//);
      expect(g.definition.length).toBeGreaterThan(20);
    }
  });
});
