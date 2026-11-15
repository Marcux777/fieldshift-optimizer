// AI-assisted (OpenBot agent)
// Tests for the five "insight" pieces. Expected numbers are read from the golden API JSON
// (fixtures/reference, same canonical hashes as the live /v1/run and /v1/evaluate), never typed by hand
// unless the test is explicitly checking the user's quoted figures against the API.
import { describe, expect, it } from 'vitest';
import goldenEvalJson from '../../../fixtures/reference/golden_eval_report.json';
import goldenRunJson from '../../../fixtures/reference/golden_run_result.json';

import type { Catalog, EvalReport, RunResult } from './api/client';
import {
  PAIRED_RULE,
  cropSequences,
  evidenceGapsRisks,
  pairedOutcome,
  premiumVsProtection,
  protectionStory,
  replayPair,
  sowingGrid
} from './insights';

const run = goldenRunJson as unknown as RunResult;
const report = goldenEvalJson as unknown as EvalReport;

const CATALOG: Catalog = {
  crops: [
    { key: 'soja', label: 'Soja (Grupo I)', windows: ['J1', 'J3'], cycle_days: 100 },
    { key: 'milho_1a', label: 'Milho 1ª safra (Grupo I)', windows: ['J1', 'J3'], cycle_days: 100 },
    { key: 'milho_2a', label: 'Milho 2ª safra (Grupo I)', windows: ['J2', 'J4'], cycle_days: 110 },
    { key: 'milho_2a_braquiaria', label: 'Milho 2ª safra + braquiária (Grupo I)', windows: ['J2', 'J4'], cycle_days: 115 },
    { key: 'sorgo_2a', label: 'Sorgo granífero 2ª safra (Grupo I)', windows: ['J2', 'J4'], cycle_days: 110 },
    { key: 'pousio', label: 'Pousio', windows: ['J2', 'J4'], cycle_days: 0 }
  ],
  windows: [
    { name: 'J1', crops: ['soja', 'milho_1a'], decendios: [25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36] },
    { name: 'J2', crops: ['milho_2a', 'milho_2a_braquiaria', 'sorgo_2a'], decendios: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12] },
    { name: 'J3', crops: ['soja', 'milho_1a'], decendios: [25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36] },
    { name: 'J4', crops: ['milho_2a', 'milho_2a_braquiaria', 'sorgo_2a'], decendios: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12] }
  ],
  soils: ['AD1', 'AD2', 'AD3', 'AD4', 'AD5', 'AD6'],
  textures: ['arenosa', 'media', 'argilosa'],
  default_p0: { j1: 'soja', dec1: 29, j2: 'milho_2a', j3: 'soja', dec3: 29, j4: 'milho_2a' },
  weight_grid: [],
  risk_limit: 0.2
};

const cmp = (b: string) => report.comparisons.find((c) => c.a === 'P2' && c.b === b)!;

describe('paired win/tie/loss rule (same as backend evaluation.rs: D < -eps win, |D| <= eps tie, D > eps loss)', () => {
  it('reproduces the API counts from per_scenario_regret', () => {
    expect(PAIRED_RULE.epsilon).toBe(0.02);
    const p2 = report.policies.P2.per_scenario_regret;
    for (const b of ['P1', 'P1b']) {
      const derived = pairedOutcome(p2, report.policies[b].per_scenario_regret, 0.02);
      expect(derived).toEqual({ wins: cmp(b).wins, ties: cmp(b).ties, losses: cmp(b).losses });
    }
  });
});

describe('piece 1: P2 as protection, not winner', () => {
  it('matches the API JSON and the figures quoted in the request', () => {
    const s = protectionStory(report)!;
    expect(s.p2.m1).toBe(report.policies.P2.M1_max_regret);
    expect(s.p1.m1).toBe(report.policies.P1.M1_max_regret);
    expect(s.p1b.m1).toBe(report.policies.P1b.M1_max_regret);
    expect(s.m5).toBe(report.M5_price_of_robustness);
    // Request: 0.2811 -> 0.1744, "about 38%", M5 0.0080, 2 wins / 4 ties / 8 losses.
    expect(s.p1.m1.toFixed(4)).toBe('0.2811');
    expect(s.p2.m1.toFixed(4)).toBe('0.1744');
    expect(Math.round(s.worstYearReductionVsP1Pct)).toBe(38);
    expect(s.m5.toFixed(4)).toBe('0.0080');
    expect(s.vsP1).toEqual({ wins: 2, ties: 4, losses: 8 });
    expect(s.vsP1Consistent).toBe(true);
    expect(s.primary.classification).toBe('NEGATIVO');
    expect(s.secondary.classification).toBe('NEGATIVO');
    expect(s.p2BeatsP1bWorstYear).toBe(false);
    expect(s.text.primary).toContain('P2 vs P1b');
    expect(s.text.primary).toContain('did not reduce');
    expect(s.text.primary).toContain('NEGATIVE');
    expect(s.text.secondary).toMatch(/secondary/i);
    expect(s.text.secondary).toContain('38%');
    expect(s.text.tradeoff).toContain('2 wins, 4 ties and 8 losses');
    expect(`${s.text.primary} ${s.text.secondary} ${s.text.tradeoff}`).not.toMatch(/\bbetter\b|\bwins overall\b|\bbest\b/);
  });
});

describe('piece 2: premium paid vs protection received', () => {
  it('uses M5 for P2 and the same mean-climate difference for P1b, both against P1', () => {
    const rows = premiumVsProtection(report);
    const p2 = rows.find((r) => r.policy === 'P2')!;
    const p1b = rows.find((r) => r.policy === 'P1b')!;
    expect(p2.premium).toBeCloseTo(report.M5_price_of_robustness!, 12);
    expect(p2.premium).toBeCloseTo(report.policies.P2.loss_mean_climate! - report.policies.P1.loss_mean_climate!, 12);
    expect(p2.protection).toBeCloseTo(report.policies.P1.M1_max_regret! - report.policies.P2.M1_max_regret!, 12);
    expect(p1b.premium).toBeCloseTo(report.policies.P1b.loss_mean_climate! - report.policies.P1.loss_mean_climate!, 12);
    expect(p1b.protection).toBeCloseTo(report.policies.P1.M1_max_regret! - report.policies.P1b.M1_max_regret!, 12);
    // P1b protected the worst year more than P2 did.
    expect(p1b.protection).toBeGreaterThan(p2.protection);
  });
});

describe('piece 3: sowing-date risk map', () => {
  it('lists crop sequences and builds a 12 x 12 grid with infeasible cells left empty', () => {
    const seqs = cropSequences(run, CATALOG);
    expect(seqs.reduce((n, s) => n + s.nPlans, 0)).toBe(576);
    const p2 = run.policies.P2.plan!.id;
    const grid = sowingGrid(run, CATALOG, [p2[0], p2[2], p2[3], p2[5]]);
    expect(grid.d1).toHaveLength(12);
    expect(grid.d3).toHaveLength(12);
    expect(grid.d1[0]).toEqual({ ordinal: 0, decendio: 25, label: '25: 1/Sep' });
    const feasible = grid.cells.flat().filter((c) => c.value !== null);
    const infeasible = grid.cells.flat().filter((c) => c.value === null);
    expect(feasible.length + infeasible.length).toBe(144);
    expect(infeasible.length).toBeGreaterThan(0);
    // Every feasible cell equals the mean construction exposure (objectives[1]) of that plan ID.
    const rows = (run.objectives as unknown as Array<[number[], number, number, number, number]>);
    for (const cell of feasible) {
      const row = rows.find((r) => r[0].join(',') === cell.planId!.join(','))!;
      expect(cell.value).toBe(row[1]);
    }
    // P1, P1b and P2 share this crop sequence, so all three are marked; P0 (soy/maize) is not.
    expect(grid.markers.map((m) => m.policy).sort()).toEqual(['P1', 'P1b', 'P2']);
    const p2cell = grid.cells.flat().find((c) => c.policies.includes('P2'))!;
    expect(p2cell.planId).toEqual(p2);
    expect(grid.min).toBeLessThanOrEqual(grid.max);
  });
});

describe('piece 4: Evidence, Gaps and Risks per policy', () => {
  it('derives evidence, crop-specific [N] gaps and risks from the API', () => {
    const p2 = evidenceGapsRisks(run, report, CATALOG, 'P2')!;
    expect(p2.evidence.snapshotId).toBe(run.snapshot_id);
    expect(p2.evidence.constructionPairs).toBe(report.construction.length);
    expect(p2.evidence.testPairs).toBe(report.test.length);
    expect(p2.evidence.constructionPairs).toBe(28);
    expect(p2.evidence.testPairs).toBe(14);
    expect(p2.evidence.constructionRange).toBe(`${report.construction[0]} to ${report.construction[27]}`);
    expect(p2.risks.worstYear).toBe(report.policies.P2.M1_max_regret);
    expect(p2.risks.dryLoss).toBe(report.policies.P2.M3_dry_mean_loss);
    expect(p2.risks.dryPairs).toBe(report.dry_test_scenarios.length);
    expect(p2.risks.weightChanges).toEqual({ changed: 12, total: 15 });
    // P2 grows maize + brachiaria and sorghum: brachiaria and sorghum-specific assumptions apply.
    const ids = p2.gaps.planSpecific.map((g) => g.id);
    expect(ids).toContain('brachiaria-desiccation');
    expect(ids).toContain('harvest-rain-soy-sorghum');
    expect(p2.gaps.planSpecific.every((g) => g.source.startsWith('docs/LIMITATIONS.md'))).toBe(true);
    // P0 is the user-given plan: it is never re-selected, and the demo P0 itself is marked [N].
    const p0 = evidenceGapsRisks(run, report, CATALOG, 'P0')!;
    expect(p0.risks.weightChanges).toBeNull();
    expect(p0.gaps.planSpecific.map((g) => g.id)).toContain('demo-p0');
    expect(p0.gaps.planSpecific.map((g) => g.id)).not.toContain('brachiaria-desiccation');
    expect(p0.gaps.global.length).toBeGreaterThan(0);
  });
});

describe('piece 5: replay one held-out pair', () => {
  it('reads the regret of each policy for the chosen pair and flags lowest/highest', () => {
    expect(report.test[10]).toBe('2020/21–2021/22');
    const r = replayPair(report, 10)!;
    expect(r.label).toBe('2020/21–2021/22');
    for (const row of r.rows) {
      expect(row.regret).toBe(report.policies[row.policy].per_scenario_regret[10]);
    }
    const lowest = r.rows.filter((x) => x.lowest).map((x) => x.policy);
    const highest = r.rows.filter((x) => x.highest).map((x) => x.policy);
    const values = r.rows.map((x) => x.regret);
    expect(r.rows.find((x) => x.policy === lowest[0])!.regret).toBe(Math.min(...values));
    expect(r.rows.find((x) => x.policy === highest[0])!.regret).toBe(Math.max(...values));
    // Different pairs give different numbers.
    const first = replayPair(report, 0)!;
    expect(first.rows.map((x) => x.regret)).not.toEqual(r.rows.map((x) => x.regret));
    expect(replayPair(report, 99)).toBeNull();
  });
});
