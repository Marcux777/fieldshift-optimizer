// AI-assisted (OpenBot agent)
import { describe, expect, it } from 'vitest';
import goldenEvalJson from '../../../../fixtures/reference/golden_eval_report.json';
import goldenRunJson from '../../../../fixtures/reference/golden_run_result.json';

import { parseObjectiveRows, type EvalReport, type RunResult } from '../api/client';
import { lossShares, robustnessTradeoff, sowingGrid, yearByYear } from './insights';

const report = goldenEvalJson as unknown as EvalReport;
const run = goldenRunJson as unknown as RunResult;
const rows = parseObjectiveRows(run.objectives);
const planOf = (k: string) => run.policies[k]!.plan!.id;
const rowOf = (k: string) => rows.find((r) => r.planId.join(',') === planOf(k).join(','))!;

/** Deep copy with an override, to build infeasible / null cases from the real fixture. */
function withPolicy(key: string, patch: Record<string, unknown>): EvalReport {
  const copy = structuredClone(report) as unknown as { policies: Record<string, Record<string, unknown>> };
  copy.policies[key] = { ...copy.policies[key], ...patch };
  return copy as unknown as EvalReport;
}

describe('robustnessTradeoff', () => {
  it('reads P1 -> P2 worst-year regret, the M5 price and the P2 vs P1 tally', () => {
    const t = robustnessTradeoff(report)!;
    expect(t.p1MaxRegret.toFixed(4)).toBe('0.2811');
    expect(t.p2MaxRegret.toFixed(4)).toBe('0.1744');
    expect(t.reductionPct.toFixed(1)).toBe('38.0');
    expect(t.price.toFixed(4)).toBe('0.0080');
    expect(t.vsP1).toEqual({ wins: 2, ties: 4, losses: 8 });
    expect(t.classification).toBe('NEGATIVO');
    // values come straight from the JSON
    expect(t.p1MaxRegret).toBe(report.policies.P1.M1_max_regret);
    expect(t.price).toBe(report.M5_price_of_robustness);
  });

  it('returns null when P1 or P2 is infeasible or a value is null', () => {
    expect(robustnessTradeoff(withPolicy('P2', { feasible: false }))).toBeNull();
    expect(robustnessTradeoff(withPolicy('P1', { M1_max_regret: null }))).toBeNull();
    expect(robustnessTradeoff({ ...report, M5_price_of_robustness: null })).toBeNull();
    expect(robustnessTradeoff({ ...report, comparisons: [] })).toBeNull();
  });
});

describe('yearByYear', () => {
  it('lists the 14 test pairs with each policy regret and the worst pair per policy', () => {
    const y = yearByYear(report)!;
    expect(y.pairs).toHaveLength(14);
    expect(y.pairs[0].label).toBe(report.test[0]);
    expect(y.pairs.filter((p) => p.dry)).toHaveLength(report.dry_test_scenarios.length);
    expect(y.pairs[10].regret.P2).toBe(report.policies.P2.per_scenario_regret[10]);
    expect(y.worst.P1?.label).toBe('2020/21–2021/22');
    expect(y.worst.P2?.label).toBe('2020/21–2021/22');
    expect(y.worst.P1b?.label).toBe('2013/14–2014/15');
    expect(y.worst.P2?.regret).toBe(report.policies.P2.M1_max_regret);
  });

  it('gives null for an infeasible policy and for a report without test pairs', () => {
    const y = yearByYear(withPolicy('P0', { feasible: false, per_scenario_regret: [] }))!;
    expect(y.worst.P0).toBeNull();
    expect(y.pairs.every((p) => p.regret.P0 === null)).toBe(true);
    expect(y.worst.P2).not.toBeNull();
    expect(yearByYear({ ...report, test: [] })).toBeNull();
  });
});

describe('sowingGrid', () => {
  it('keeps the plans with the same crops (ID positions 0, 2, 3, 5) on a d1 x d3 grid', () => {
    const p2 = sowingGrid(rows, planOf('P2'))!;
    expect(p2.cells).toHaveLength(15);
    expect([p2.d1.length, p2.d3.length]).toEqual([3, 5]);
    expect(p2.cells.every((c) => [0, 2, 3, 5].every((i) => c.planId[i] === planOf('P2')[i]))).toBe(true);
    expect(p2.cells.some((c) => c.planId.join(',') === planOf('P1').join(','))).toBe(true);

    const p0 = sowingGrid(rows, planOf('P0'))!;
    expect(p0.cells).toHaveLength(9);
    expect([p0.d1.length, p0.d3.length]).toEqual([3, 3]);
    const cell = p0.cells.find((c) => c.planId.join(',') === planOf('P0').join(','))!;
    expect(cell.meanExposure).toBe(rowOf('P0').meanExposure);
  });

  it('returns null for a missing plan ID or a sequence with no feasible plan', () => {
    expect(sowingGrid(rows, null)).toBeNull();
    expect(sowingGrid(rows, [9, 0, 9, 9, 0, 9])).toBeNull();
    expect(sowingGrid([], planOf('P2'))).toBeNull();
  });
});

describe('lossShares', () => {
  it('splits the weighted construction loss into exposure, cover and diversity', () => {
    const p0 = lossShares(rowOf('P0'), run.weights)!;
    const p1 = lossShares(rowOf('P1'), run.weights)!;
    expect(p0.total.toFixed(4)).toBe('0.3690');
    expect(p1.total.toFixed(4)).toBe('0.1497');
    // equals the mean weighted loss the optimizer reports for the same plan
    expect(p0.total).toBeCloseTo(run.policies.P0!.metrics_construction!.mean_L, 9);
    expect(p1.total).toBeCloseTo(run.policies.P1!.metrics_construction!.mean_L, 9);
    expect(p0.exposure + p0.coverage + p0.diversity).toBeCloseTo(p0.total, 12);
    expect(p1.diversity).toBe(0);
    expect(p0.share.exposure + p0.share.coverage + p0.share.diversity).toBeCloseTo(1, 12);
  });

  it('returns null for a missing row or invalid weights', () => {
    expect(lossShares(null, run.weights)).toBeNull();
    expect(lossShares(rowOf('P0'), [0.5, 0.5])).toBeNull();
    expect(lossShares(rowOf('P0'), [0.5, Number.NaN, 0.5])).toBeNull();
  });
});
