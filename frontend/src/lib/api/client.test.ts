// AI-assisted (OpenBot agent)
import { describe, expect, it } from 'vitest';
import goldenEvalJson from '../../../../fixtures/reference/golden_eval_report.json';
import goldenRunJson from '../../../../fixtures/reference/golden_run_result.json';

import {
  DEFAULT_INPUTS,
  DEFAULT_P0,
  OpenApiHttpError,
  buildEvaluationExportJson,
  countChangedWeightSettings,
  createOpenApiClient,
  cropDisplayName,
  dateLabelFromT,
  decStartT,
  decendioOptionLabel,
  formatOutcomeLabel,
  formatP0Diagnostics,
  isExploratoryInputs,
  isSha256Hex,
  isSnapshotId,
  parseObjectiveRows,
  tToMonthDay,
  validateInputs,
  type EvalReport,
  type RunResult
} from './client';

describe('FieldShift OpenAPI boundary and domain logic', () => {
  it('formats ZARC decendios and agricultural day indices identically to Python ui_app.py', () => {
    expect(decStartT(29)).toBe(40);
    expect(tToMonthDay(40)).toEqual({ month: 10, day: 11 });
    expect(decendioOptionLabel(29)).toBe('29: 11/Oct');
    expect(dateLabelFromT(0)).toBe('1/Sep');
    expect(dateLabelFromT(365)).toBe('1/Sep (year 2)');
    expect(dateLabelFromT(null)).toBe('—');
    expect(cropDisplayName('milho_2a_braquiaria')).toBe('Second-season maize + brachiaria');
    expect(formatOutcomeLabel('NEGATIVO')).toBe('NEGATIVE');
    expect(formatOutcomeLabel('INVIAVEL')).toBe('INFEASIBLE');
    expect(formatOutcomeLabel('DADOS_INSUFICIENTES')).toBe('INSUFFICIENT DATA');
  });

  it('validates farm inputs and rejects invalid weights or sowing periods before API call', () => {
    expect(validateInputs(DEFAULT_INPUTS)).toEqual({
      kind: 'ok',
      inputs: DEFAULT_INPUTS
    });

    expect(
      validateInputs({
        ...DEFAULT_INPUTS,
        weights: [0.9, 0.25, 0.25]
      })
    ).toEqual({
      kind: 'invalid',
      message: 'Weights must be finite, non-negative and sum to 1.'
    });

    expect(
      validateInputs({
        ...DEFAULT_INPUTS,
        weights: [-0.5, 1.0, 0.5]
      })
    ).toEqual({
      kind: 'invalid',
      message: 'Weights must be finite, non-negative and sum to 1.'
    });

    expect(
      validateInputs({
        ...DEFAULT_INPUTS,
        ad_class: 'AD9'
      })
    ).toEqual({
      kind: 'invalid',
      message: 'Select a soil water class from AD1 to AD6.'
    });

    expect(
      validateInputs({
        ...DEFAULT_INPUTS,
        p0: { ...DEFAULT_P0, dec1: 37 }
      })
    ).toEqual({
      kind: 'invalid',
      message: 'P0 sowing periods must be integers between 1 and 36.'
    });
  });

  it('distinguishes frozen default inputs from exploratory changed-input runs', () => {
    expect(isExploratoryInputs(DEFAULT_INPUTS)).toBe(false);
    expect(isExploratoryInputs({ ...DEFAULT_INPUTS, p0: null })).toBe(false);
    expect(isExploratoryInputs({ ...DEFAULT_INPUTS, ad_class: 'AD1' })).toBe(true);
    expect(isExploratoryInputs({ ...DEFAULT_INPUTS, texture: 'arenosa' })).toBe(true);
    expect(isExploratoryInputs({ ...DEFAULT_INPUTS, weights: [1.0, 0.0, 0.0] })).toBe(true);
    expect(
      isExploratoryInputs({
        ...DEFAULT_INPUTS,
        p0: { ...DEFAULT_P0, j4: 'milho_2a_braquiaria' }
      })
    ).toBe(true);
  });

  it('translates ZARC and fallow P0 rejection diagnostics into actionable English messages', () => {
    const messages = formatP0Diagnostics([
      'J1: soja no decêndio 29 tem risco 30% > 20% em AD1/argilosa',
      'J2: pousio não permitido quando há cultura viável',
      'J3: violação de calendário'
    ]);
    expect(messages).toEqual([
      'J1: ZARC risk 30% exceeds 20% for the selected crop and sowing period in this soil class.',
      'J2: fallow is not allowed while a feasible second crop exists.',
      'J3: the selected crop or sowing period violates the calendar, succession or fixed ZARC constraints. Change P0 explicitly to compare it.'
    ]);
  });

  it('parses golden RunResult and EvalReport through the generated OpenAPI client', async () => {
    const goldenRun = goldenRunJson as unknown as RunResult;
    const goldenEval = goldenEvalJson as unknown as EvalReport;

    expect(isSha256Hex(goldenRun.result_hash)).toBe(true);
    expect(isSha256Hex(goldenEval.result_hash)).toBe(true);
    expect(isSnapshotId(goldenRun.snapshot_id)).toBe(true);

    const calls: Array<{ url: string; method: string }> = [];
    const client = createOpenApiClient({
      baseUrl: 'http://127.0.0.1:3000',
      fetchFn: async (input, init) => {
        const url = String(input);
        const method = init?.method ?? 'GET';
        calls.push({ url, method });
        if (url.endsWith('/v1/run')) {
          return new Response(JSON.stringify(goldenRun), { status: 200 });
        }
        if (url.endsWith('/v1/evaluate')) {
          return new Response(JSON.stringify(goldenEval), { status: 200 });
        }
        return new Response(
          JSON.stringify({
            error: { code: 'INVALID_INPUT', message: 'weights must be finite, non-negative and sum to 1' }
          }),
          { status: 422 }
        );
      }
    });

    const run = await client.run(DEFAULT_INPUTS);
    const report = await client.evaluate(DEFAULT_INPUTS);

    expect(calls).toEqual([
      { url: 'http://127.0.0.1:3000/v1/run', method: 'POST' },
      { url: 'http://127.0.0.1:3000/v1/evaluate', method: 'POST' }
    ]);
    expect(run.result_hash).toBe('b43703326690e6d0ddd2be48641fbdb9ba46908810ebfca0c8c90a379c79fd66');
    expect(report.result_hash).toBe('504c28e85e54cbd4e9470e8a423499299dcbed643cb9b2ff8aa461873e134c18');
    expect(report.outcome).toBe('NEGATIVO');

    const objRows = parseObjectiveRows(run.objectives);
    expect(objRows.length).toBe(576);
    expect(objRows[0]).toEqual({
      planId: [0, 3, 2, 0, 3, 2],
      meanExposure: 0.190459957,
      worstExposure: 0.274056307,
      coverageLoss: 0.433390411,
      diversityLoss: 0.666666667
    });

    const changedP2 = countChangedWeightSettings(
      run.sensitivity,
      'P2',
      run.policies.P2?.plan?.id
    );
    expect(changedP2.length).toBe(12);

    const exported = JSON.parse(buildEvaluationExportJson(run, report)) as Record<string, unknown>;
    expect(exported.snapshot_id).toBe(run.snapshot_id);
    expect(exported.optimizer_result_hash).toBe(run.result_hash);
    expect(exported.evaluation_result_hash).toBe(report.result_hash);

    await expect(client.plans(DEFAULT_INPUTS)).rejects.toThrowError(
      new OpenApiHttpError({
        status: 422,
        code: 'INVALID_INPUT',
        message: 'weights must be finite, non-negative and sum to 1'
      })
    );
  });
});

describe('retry on busy API responses (item A1)', () => {
  const golden = goldenRunJson as unknown as RunResult;

  function busy(status: number, retryAfter?: string): Response {
    return new Response(JSON.stringify({ error: { code: 'BUSY', message: 'busy' } }), {
      status,
      headers: retryAfter ? { 'Retry-After': retryAfter } : {}
    });
  }

  it('retries 429 and 503 with backoff (honouring Retry-After) and then returns the result', async () => {
    const responses = [busy(429), busy(503, '2'), new Response(JSON.stringify(golden), { status: 200 })];
    const sleeps: number[] = [];
    let calls = 0;
    const client = createOpenApiClient({
      fetchFn: async () => {
        calls += 1;
        return responses.shift() as Response;
      },
      retry: { maxRetries: 3, baseDelayMs: 400, maxDelayMs: 5000, sleep: async (ms) => void sleeps.push(ms) }
    });
    const run = await client.run(DEFAULT_INPUTS);
    expect(run.result_hash).toBe(golden.result_hash);
    expect(calls).toBe(3);
    expect(sleeps).toEqual([400, 2000]);
  });

  it('gives up after maxRetries and surfaces the last busy error', async () => {
    const sleeps: number[] = [];
    let calls = 0;
    const client = createOpenApiClient({
      fetchFn: async () => {
        calls += 1;
        return busy(503, '60');
      },
      retry: { maxRetries: 2, baseDelayMs: 300, maxDelayMs: 5000, sleep: async (ms) => void sleeps.push(ms) }
    });
    await expect(client.evaluate(DEFAULT_INPUTS)).rejects.toMatchObject({ status: 503, code: 'BUSY' });
    expect(calls).toBe(3);
    expect(sleeps).toEqual([5000, 5000]);
  });

  it('does not retry validation errors', async () => {
    let calls = 0;
    const client = createOpenApiClient({
      fetchFn: async () => {
        calls += 1;
        return new Response(JSON.stringify({ error: { code: 'INVALID_INPUT', message: 'bad' } }), { status: 422 });
      },
      retry: { maxRetries: 3, baseDelayMs: 1, maxDelayMs: 1, sleep: async () => undefined }
    });
    await expect(client.run(DEFAULT_INPUTS)).rejects.toMatchObject({ status: 422 });
    expect(calls).toBe(1);
  });
});
