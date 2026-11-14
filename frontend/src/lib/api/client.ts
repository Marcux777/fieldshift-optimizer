// AI-assisted (OpenBot agent)
import {
  OpenApiHttpError,
  type Catalog,
  type CurrentPlan,
  type EvalReport,
  type GeneratedOpenApiClient,
  type Health,
  type Inputs,
  type Limitations,
  type Meta,
  type OpenApiClientOptions,
  type ParetoResponse,
  type Plans,
  type Provenance,
  type RunResult
} from './openapi.gen';

export type {
  Catalog,
  CurrentPlan,
  EvalReport,
  GeneratedOpenApiClient,
  Health,
  Inputs,
  Limitations,
  Meta,
  ParetoResponse,
  Plans,
  Provenance,
  RunResult
};
export { OpenApiHttpError };

export type Sha256Hex = string & { readonly __brand: 'Sha256Hex' };
export type SnapshotId = string & { readonly __brand: 'SnapshotId' };

export type ObjectiveRow = {
  readonly planId: readonly [number, number, number, number, number, number];
  readonly meanExposure: number;
  readonly worstExposure: number;
  readonly coverageLoss: number;
  readonly diversityLoss: number;
};

export type ValidationResult =
  | { readonly kind: 'ok'; readonly inputs: Inputs }
  | { readonly kind: 'invalid'; readonly message: string };

export const DEFAULT_WEIGHTS: readonly [number, number, number] = [0.5, 0.25, 0.25];

export const DEFAULT_P0: CurrentPlan = {
  j1: 'soja',
  dec1: 29,
  j2: 'milho_2a',
  j3: 'soja',
  dec3: 29,
  j4: 'milho_2a'
};

export const DEFAULT_INPUTS: Inputs = {
  ad_class: 'AD4',
  texture: 'argilosa',
  weights: [...DEFAULT_WEIGHTS],
  p0: DEFAULT_P0
};

export const CROP_LABELS: Readonly<Record<string, string>> = {
  soja: 'Soybean',
  milho_1a: 'First-season maize',
  milho_2a: 'Second-season maize',
  sorgo_2a: 'Second-season sorghum',
  milho_2a_braquiaria: 'Second-season maize + brachiaria',
  pousio: 'Fallow'
};

export const TEXTURE_LABELS: Readonly<Record<Inputs['texture'], string>> = {
  argilosa: 'Clay (argilosa)',
  media: 'Medium (media)',
  arenosa: 'Sandy (arenosa)'
};

export const OUTCOME_LABELS: Readonly<Record<string, string>> = {
  NEGATIVO: 'NEGATIVE',
  INCONCLUSIVO: 'INCONCLUSIVE',
  SUSTENTADO: 'SUPPORTED',
  INVIAVEL: 'INFEASIBLE',
  DADOS_INSUFICIENTES: 'INSUFFICIENT DATA'
};

export const POLICY_LABELS: Readonly<Record<string, string>> = {
  P0: 'Current plan',
  P1: 'Mean climate',
  P1b: 'Expected loss',
  P2: 'Minimax regret'
};

export const POLICY_DESCRIPTIONS: Readonly<Record<string, string>> = {
  P0: 'A fixed user-specified baseline, not an optimized recommendation.',
  P1: 'Minimizes loss under the construction mean climate.',
  P1b: 'Minimizes average loss across actual construction scenarios.',
  P2: 'Minimizes the largest construction regret; this does not ensure better performance on the held-out years.'
};

export const DISCLAIMER =
  "Decision support only: this tool supports but does not replace the farmer's judgment. Indicators describe climate exposure and calendar proxies, not yield, profit, water savings or soil recovery.";

export const FROZEN_OUTCOME_BANNER =
  'Frozen evaluation outcome: NEGATIVE — under the default AD4/clay inputs, minimax regret did not reduce maximum test regret versus expected loss. Changed-input runs are exploratory and do not replace this finding.';

const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'] as const;
const AGRI_MONTH_ORDER = [9, 10, 11, 12, 1, 2, 3, 4, 5, 6, 7, 8] as const;
const MONTH_LENGTHS: Readonly<Record<number, number>> = {
  1: 31,
  2: 28,
  3: 31,
  4: 30,
  5: 31,
  6: 30,
  7: 31,
  8: 31,
  9: 30,
  10: 31,
  11: 30,
  12: 31
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function isString(value: unknown): value is string {
  return typeof value === 'string';
}

function isFiniteNumber(value: unknown): value is number {
  return typeof value === 'number' && Number.isFinite(value);
}

export function isSha256Hex(value: unknown): value is Sha256Hex {
  return typeof value === 'string' && /^[0-9a-f]{64}$/.test(value);
}

export function isSnapshotId(value: unknown): value is SnapshotId {
  return typeof value === 'string' && value.startsWith('power-daily-') && value.length > 12;
}

export function cropDisplayName(key: string): string {
  return CROP_LABELS[key] ?? key.replaceAll('_', ' ');
}

export function formatOutcomeLabel(outcome: string): string {
  return OUTCOME_LABELS[outcome] ?? outcome;
}

function monthStartT(month: number): number {
  let acc = 0;
  for (const m of AGRI_MONTH_ORDER) {
    if (m === month) return acc;
    acc += MONTH_LENGTHS[m] ?? 30;
  }
  return 0;
}

export function tToMonthDay(t: number): { readonly month: number; readonly day: number } {
  const u = ((t % 365) + 365) % 365;
  for (let i = AGRI_MONTH_ORDER.length - 1; i >= 0; i--) {
    const m = AGRI_MONTH_ORDER[i];
    const start = monthStartT(m);
    if (u >= start) {
      return { month: m, day: u - start + 1 };
    }
  }
  return { month: 9, day: 1 };
}

export function dateLabelFromT(t: number | null | undefined): string {
  if (t === null || t === undefined || !Number.isFinite(t)) return '—';
  const { month, day } = tToMonthDay(t);
  const suffix = t >= 365 ? ' (year 2)' : '';
  return `${day}/${MONTHS[month - 1]}${suffix}`;
}

export function decStartT(dec: number): number {
  const clamped = Math.min(36, Math.max(1, Math.trunc(dec)));
  const month = Math.floor((clamped - 1) / 3) + 1;
  const part = (clamped - 1) % 3;
  const day = 1 + 10 * part;
  return monthStartT(month) + day - 1;
}

export function decendioOptionLabel(dec: number): string {
  return `${dec}: ${dateLabelFromT(decStartT(dec))}`;
}

export function formatP0Diagnostics(reasons: readonly string[] | undefined): string[] {
  if (!reasons || reasons.length === 0) return [];
  return reasons.map((reason) => {
    const windowName = reason.split(':', 1)[0] ?? 'P0';
    const riskMatch = /risco (\d+)% > (\d+)%/.exec(reason);
    if (riskMatch) {
      return `${windowName}: ZARC risk ${riskMatch[1]}% exceeds ${riskMatch[2]}% for the selected crop and sowing period in this soil class.`;
    }
    if (reason.includes('pousio não permitido')) {
      return `${windowName}: fallow is not allowed while a feasible second crop exists.`;
    }
    return `${windowName}: the selected crop or sowing period violates the calendar, succession or fixed ZARC constraints. Change P0 explicitly to compare it.`;
  });
}

export function validateInputs(candidate: {
  readonly ad_class: string;
  readonly texture: string;
  readonly weights: readonly number[];
  readonly p0?: CurrentPlan | null;
}): ValidationResult {
  const validSoils: ReadonlyArray<Inputs['ad_class']> = ['AD1', 'AD2', 'AD3', 'AD4', 'AD5', 'AD6'];
  const validTextures: ReadonlyArray<Inputs['texture']> = ['arenosa', 'media', 'argilosa'];
  const soil = validSoils.find((s) => s === candidate.ad_class);
  if (!soil) {
    return { kind: 'invalid', message: 'Select a soil water class from AD1 to AD6.' };
  }
  const texture = validTextures.find((t) => t === candidate.texture);
  if (!texture) {
    return { kind: 'invalid', message: 'Select sandy, medium or clay soil texture.' };
  }
  if (
    candidate.weights.length !== 3 ||
    candidate.weights.some((w) => !Number.isFinite(w) || w < 0) ||
    // Same tolerance as `Inputs::validate` in backend/crates/fieldshift-api/src/engine.rs.
    Math.abs(candidate.weights[0] + candidate.weights[1] + candidate.weights[2] - 1.0) > 1e-12
  ) {
    return {
      kind: 'invalid',
      message: 'Weights must be finite, non-negative and sum to 1.'
    };
  }
  if (candidate.p0) {
    if (
      !Number.isInteger(candidate.p0.dec1) ||
      candidate.p0.dec1 < 1 ||
      candidate.p0.dec1 > 36 ||
      !Number.isInteger(candidate.p0.dec3) ||
      candidate.p0.dec3 < 1 ||
      candidate.p0.dec3 > 36
    ) {
      return {
        kind: 'invalid',
        message: 'P0 sowing periods must be integers between 1 and 36.'
      };
    }
  }
  return {
    kind: 'ok',
    inputs: {
      ad_class: soil,
      texture,
      weights: [candidate.weights[0], candidate.weights[1], candidate.weights[2]],
      p0: candidate.p0 ?? null
    }
  };
}

export function isExploratoryInputs(inputs: Inputs, defaultP0: CurrentPlan = DEFAULT_P0): boolean {
  if (inputs.ad_class !== 'AD4' || inputs.texture !== 'argilosa') {
    return true;
  }
  if (
    Math.abs(inputs.weights[0] - DEFAULT_WEIGHTS[0]) > 1e-9 ||
    Math.abs(inputs.weights[1] - DEFAULT_WEIGHTS[1]) > 1e-9 ||
    Math.abs(inputs.weights[2] - DEFAULT_WEIGHTS[2]) > 1e-9
  ) {
    return true;
  }
  if (!inputs.p0) {
    return false;
  }
  return (
    inputs.p0.j1 !== defaultP0.j1 ||
    inputs.p0.dec1 !== defaultP0.dec1 ||
    inputs.p0.j2 !== defaultP0.j2 ||
    inputs.p0.j3 !== defaultP0.j3 ||
    inputs.p0.dec3 !== defaultP0.dec3 ||
    inputs.p0.j4 !== defaultP0.j4
  );
}

export function parseObjectiveRows(raw: RunResult['objectives']): ObjectiveRow[] {
  const rows: ObjectiveRow[] = [];
  for (const item of raw) {
    if (!Array.isArray(item) || item.length !== 5) continue;
    const [idCandidate, g1Mean, g1Max, g2, g3] = item;
    if (
      Array.isArray(idCandidate) &&
      idCandidate.length === 6 &&
      idCandidate.every(isFiniteNumber) &&
      isFiniteNumber(g1Mean) &&
      isFiniteNumber(g1Max) &&
      isFiniteNumber(g2) &&
      isFiniteNumber(g3)
    ) {
      rows.push({
        planId: [
          idCandidate[0],
          idCandidate[1],
          idCandidate[2],
          idCandidate[3],
          idCandidate[4],
          idCandidate[5]
        ],
        meanExposure: g1Mean,
        worstExposure: g1Max,
        coverageLoss: g2,
        diversityLoss: g3
      });
    }
  }
  return rows;
}

export function countChangedWeightSettings(
  sensitivity: RunResult['sensitivity'],
  policy: 'P1' | 'P1b' | 'P2',
  selectedPlanId: readonly number[] | undefined
): Array<readonly [number, number, number]> {
  if (!selectedPlanId) return [];
  const key = selectedPlanId.join(',');
  const changed: Array<readonly [number, number, number]> = [];
  for (const row of sensitivity) {
    const rowId = row[policy];
    if (rowId.join(',') !== key && row.weights.length === 3) {
      changed.push([row.weights[0], row.weights[1], row.weights[2]]);
    }
  }
  return changed;
}

export function buildEvaluationExportJson(run: RunResult, report: EvalReport): string {
  return JSON.stringify(
    {
      snapshot_id: run.snapshot_id,
      config_hash: run.config_hash,
      optimizer_result_hash: run.result_hash,
      evaluation_result_hash: report.result_hash,
      run_result: run,
      evaluation: report
    },
    null,
    2
  );
}

function parseErrorPayload(status: number, payload: unknown): OpenApiHttpError {
  if (isRecord(payload) && isRecord(payload.error)) {
    const code = isString(payload.error.code) ? payload.error.code : `HTTP_${status}`;
    const message = isString(payload.error.message)
      ? payload.error.message
      : `Request failed with status ${status}`;
    return new OpenApiHttpError({ status, code, message });
  }
  return new OpenApiHttpError({
    status,
    code: `HTTP_${status}`,
    message: `Request failed with status ${status}`
  });
}

function assertHealth(data: unknown): Health {
  if (
    !isRecord(data) ||
    !isString(data.status) ||
    !isString(data.language) ||
    !isString(data.mode) ||
    typeof data.recomputation_available !== 'boolean'
  ) {
    throw new Error('Invalid Health payload at API boundary');
  }
  return {
    status: data.status,
    language: data.language,
    mode: data.mode,
    recomputation_available: data.recomputation_available
  };
}

function assertMeta(data: unknown): Meta {
  if (
    !isRecord(data) ||
    !isString(data.version) ||
    !isString(data.language) ||
    !isString(data.mode) ||
    !isString(data.protocol_version) ||
    !isSnapshotId(data.snapshot_id) ||
    !isSha256Hex(data.config_hash) ||
    !isSha256Hex(data.protocol_sha256)
  ) {
    throw new Error('Invalid Meta payload at API boundary');
  }
  return {
    version: data.version,
    language: data.language,
    mode: data.mode,
    protocol_version: data.protocol_version,
    snapshot_id: data.snapshot_id,
    config_hash: data.config_hash,
    protocol_sha256: data.protocol_sha256
  };
}

function assertCatalog(data: unknown): Catalog {
  if (
    !isRecord(data) ||
    !Array.isArray(data.crops) ||
    !Array.isArray(data.windows) ||
    !Array.isArray(data.soils) ||
    !Array.isArray(data.textures) ||
    !isRecord(data.default_p0) ||
    !Array.isArray(data.weight_grid) ||
    !isFiniteNumber(data.risk_limit)
  ) {
    throw new Error('Invalid Catalog payload at API boundary');
  }
  return data as unknown as Catalog;
}

function assertRunResult(data: unknown): RunResult {
  if (
    !isRecord(data) ||
    (data.status !== 'OK' && data.status !== 'INVIAVEL' && data.status !== 'DADOS_INSUFICIENTES') ||
    !isString(data.protocol_version) ||
    !isFiniteNumber(data.n_plans) ||
    !isSha256Hex(data.result_hash)
  ) {
    throw new Error('Invalid RunResult payload at API boundary');
  }
  return data as unknown as RunResult;
}

function assertEvalReport(data: unknown): EvalReport {
  if (
    !isRecord(data) ||
    !isString(data.outcome) ||
    !isString(data.headline) ||
    !isFiniteNumber(data.n_plans) ||
    !Array.isArray(data.comparisons) ||
    !isSha256Hex(data.result_hash)
  ) {
    throw new Error('Invalid EvalReport payload at API boundary');
  }
  return data as unknown as EvalReport;
}

function assertPlans(data: unknown): Plans {
  if (!isRecord(data) || !isString(data.status) || !isFiniteNumber(data.n_plans) || !Array.isArray(data.plans)) {
    throw new Error('Invalid Plans payload at API boundary');
  }
  return data as unknown as Plans;
}

function assertParetoResponse(data: unknown): ParetoResponse {
  if (!isRecord(data) || !isRecord(data.pareto) || !Array.isArray(data.objectives) || !Array.isArray(data.alternatives)) {
    throw new Error('Invalid ParetoResponse payload at API boundary');
  }
  return data as unknown as ParetoResponse;
}

function assertProvenance(data: unknown): Provenance {
  if (!isRecord(data) || !isRecord(data.manifest) || !isSha256Hex(data.protocol_sha256)) {
    throw new Error('Invalid Provenance payload at API boundary');
  }
  return data as unknown as Provenance;
}

function assertLimitations(data: unknown): Limitations {
  if (!isRecord(data) || !Array.isArray(data.items) || !data.items.every(isString)) {
    throw new Error('Invalid Limitations payload at API boundary');
  }
  return { items: data.items };
}

export function createOpenApiClient(options: OpenApiClientOptions = {}): GeneratedOpenApiClient {
  const base = (options.baseUrl ?? '').replace(/\/$/, '');
  const fetchImpl = options.fetchFn ?? fetch;

  async function requestJson<T>(
    path: string,
    init: RequestInit,
    parse: (raw: unknown) => T
  ): Promise<T> {
    const response = await fetchImpl(`${base}${path}`, init);
    const text = await response.text();
    let parsed: unknown = null;
    if (text.length > 0) {
      try {
        parsed = JSON.parse(text) as unknown;
      } catch {
        throw new OpenApiHttpError({
          status: response.status,
          code: 'INVALID_JSON_RESPONSE',
          message: `Server returned non-JSON response (${response.status})`
        });
      }
    }
    if (!response.ok) {
      throw parseErrorPayload(response.status, parsed);
    }
    return parse(parsed);
  }

  return {
    health() {
      return requestJson('/v1/health', { method: 'GET' }, assertHealth);
    },
    meta() {
      return requestJson('/v1/meta', { method: 'GET' }, assertMeta);
    },
    catalog() {
      return requestJson('/v1/catalog', { method: 'GET' }, assertCatalog);
    },
    plans(body: Inputs) {
      return requestJson(
        '/v1/plans',
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(body)
        },
        assertPlans
      );
    },
    run(body: Inputs) {
      return requestJson(
        '/v1/run',
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(body)
        },
        assertRunResult
      );
    },
    evaluate(body: Inputs) {
      return requestJson(
        '/v1/evaluate',
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(body)
        },
        assertEvalReport
      );
    },
    pareto(body: Inputs) {
      return requestJson(
        '/v1/pareto',
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(body)
        },
        assertParetoResponse
      );
    },
    provenance() {
      return requestJson('/v1/provenance', { method: 'GET' }, assertProvenance);
    },
    limitations() {
      return requestJson('/v1/limitations', { method: 'GET' }, assertLimitations);
    }
  };
}
