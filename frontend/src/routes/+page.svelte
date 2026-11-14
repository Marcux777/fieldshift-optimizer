<script lang="ts">
  // AI-assisted (OpenBot agent)
  import { onMount, tick } from 'svelte';
  import '../app.css';
  import ParetoChart from '$lib/components/ParetoChart.svelte';
  import PlanTimeline from '$lib/components/PlanTimeline.svelte';
  import {
    ALTERNATIVE_REASON_LABELS,
    CLASSIFICATION_RULE,
    GLOSSARY,
    checkWeights,
    comparisonReasonLabel,
    expandAlternatives,
    frontKeySet,
    parseEvalSensitivity,
    sameWeights
  } from '$lib/presentation';
  import {
    DEFAULT_INPUTS,
    DEFAULT_P0,
    DISCLAIMER,
    FROZEN_OUTCOME_BANNER,
    OpenApiHttpError,
    POLICY_DESCRIPTIONS,
    POLICY_LABELS,
    TEXTURE_LABELS,
    buildEvaluationExportJson,
    countChangedWeightSettings,
    createOpenApiClient,
    cropDisplayName,
    dateLabelFromT,
    decendioOptionLabel,
    formatOutcomeLabel,
    formatP0Diagnostics,
    isExploratoryInputs,
    parseObjectiveRows,
    validateInputs,
    type Catalog,
    type CurrentPlan,
    type EvalReport,
    type Inputs,
    type Limitations,
    type Meta,
    type Provenance,
    type RunResult
  } from '$lib/api/client';

  const TABS = [
    'Overview',
    'Policy comparison',
    'Pareto and alternatives',
    'Retrospective evaluation',
    'Weight sensitivity',
    'Sources and limitations'
  ] as const;
  type TabName = (typeof TABS)[number];

  const POLICY_KEYS = ['P0', 'P1', 'P1b', 'P2'] as const;

  const rawApiBase = (import.meta.env.VITE_API_BASE_URL ?? '').trim();
  const client = createOpenApiClient({ baseUrl: rawApiBase });

  let activeTab = $state<TabName>('Overview');
  let loading = $state<boolean>(true);
  let submitting = $state<boolean>(false);
  let formError = $state<string>('');
  let fatalError = $state<string>('');

  let catalog = $state<Catalog | null>(null);
  let meta = $state<Meta | null>(null);
  let provenance = $state<Provenance | null>(null);
  let limitations = $state<Limitations | null>(null);

  let activeInputs = $state<Inputs>({ ...DEFAULT_INPUTS, p0: { ...DEFAULT_P0 } });
  let runResult = $state<RunResult | null>(null);
  let evalReport = $state<EvalReport | null>(null);

  // Editable form state
  let adClass = $state<Inputs['ad_class']>('AD4');
  let texture = $state<Inputs['texture']>('argilosa');
  let wExposure = $state<number>(0.5);
  let wCoverage = $state<number>(0.25);
  let wDiversity = $state<number>(0.25);
  let p0J1 = $state<string>(DEFAULT_P0.j1);
  let p0Dec1 = $state<number>(DEFAULT_P0.dec1);
  let p0J2 = $state<string>(DEFAULT_P0.j2);
  let p0J3 = $state<string>(DEFAULT_P0.j3);
  let p0Dec3 = $state<number>(DEFAULT_P0.dec3);
  let p0J4 = $state<string>(DEFAULT_P0.j4);

  const defaultP0 = $derived<CurrentPlan>(catalog?.default_p0 ?? DEFAULT_P0);

  // An emptied number input binds to null; treat it as invalid, not as 0.
  function toNum(v: unknown): number {
    return v === null || v === undefined || v === '' ? Number.NaN : Number(v);
  }

  const candidateInputs = $derived({
    ad_class: adClass,
    texture,
    weights: [toNum(wExposure), toNum(wCoverage), toNum(wDiversity)],
    p0: {
      j1: p0J1,
      dec1: Number(p0Dec1),
      j2: p0J2,
      j3: p0J3,
      dec3: Number(p0Dec3),
      j4: p0J4
    }
  });

  const inputsDirty = $derived.by(() => {
    if (!runResult) return false;
    const p0Active = activeInputs.p0 ?? defaultP0;
    return (
      adClass !== activeInputs.ad_class ||
      texture !== activeInputs.texture ||
      !(Math.abs(toNum(wExposure) - activeInputs.weights[0]) <= 1e-12) ||
      !(Math.abs(toNum(wCoverage) - activeInputs.weights[1]) <= 1e-12) ||
      !(Math.abs(toNum(wDiversity) - activeInputs.weights[2]) <= 1e-12) ||
      p0J1 !== p0Active.j1 ||
      Number(p0Dec1) !== p0Active.dec1 ||
      p0J2 !== p0Active.j2 ||
      p0J3 !== p0Active.j3 ||
      Number(p0Dec3) !== p0Active.dec3 ||
      p0J4 !== p0Active.j4
    );
  });

  const weightCheck = $derived(checkWeights(candidateInputs.weights));
  const weightsInvalid = $derived(weightCheck.kind === 'invalid');

  // Results stay on screen after a rejected or failed submission, marked as stale.
  const resultsStale = $derived(Boolean(runResult) && (inputsDirty || formError !== ''));

  const activeInputsSummary = $derived(
    `${activeInputs.ad_class} / ${TEXTURE_LABELS[activeInputs.texture] ?? activeInputs.texture}, weights ${activeInputs.weights.map((w) => w.toFixed(2)).join(' / ')}`
  );

  let glossaryOpen = $state<boolean>(false);

  async function openGlossary(event: MouseEvent, id: string): Promise<void> {
    event.preventDefault();
    glossaryOpen = true;
    await tick();
    const el = document.getElementById(`glossary-${id}`);
    el?.scrollIntoView({ behavior: 'smooth', block: 'center' });
    el?.focus({ preventScroll: true });
  }

  function glossaryTitle(id: string): string {
    return GLOSSARY.find((g) => g.id === id)?.definition ?? '';
  }

  const exploratory = $derived<boolean>(isExploratoryInputs(activeInputs, defaultP0));

  const outcomeHeading = $derived.by(() => {
    if (!evalReport) return '';
    const prefix = exploratory ? 'Exploratory outcome' : 'Outcome';
    return `${prefix}: ${formatOutcomeLabel(evalReport.outcome)}`;
  });

  const primaryComparison = $derived(
    evalReport?.comparisons.find((c) => c.a === 'P2' && c.b === 'P1b') ?? null
  );

  const allObjectiveRows = $derived(runResult ? parseObjectiveRows(runResult.objectives) : []);

  const frontObjectiveRows = $derived.by(() => {
    if (!runResult || !runResult.pareto?.front_ids) return [];
    const frontKeys = new Set(runResult.pareto.front_ids.map((id) => id.join(',')));
    return allObjectiveRows.filter((row) => frontKeys.has(row.planId.join(',')));
  });

  const objectiveRowByPlanKey = $derived(new Map(allObjectiveRows.map((row) => [row.planId.join(','), row])));

  const policyChartPoints = $derived.by(() => {
    if (!runResult) return [];
    const front = frontKeySet(runResult);
    return POLICY_KEYS.flatMap((key) => {
      const id = runResult?.policies[key]?.plan?.id;
      const row = id ? objectiveRowByPlanKey.get(id.join(',')) : undefined;
      return row ? [{ ...row, policy: key, onFront: front.has(row.planId.join(',')) }] : [];
    });
  });

  const alternativeRows = $derived(runResult ? expandAlternatives(runResult) : []);
  const groupedPolicyNote = $derived.by(() => {
    const byGroup = new Map<number, string[]>();
    for (const r of alternativeRows) {
      if (r.groupSize > 1) byGroup.set(r.group, [...(byGroup.get(r.group) ?? []), ...r.policies]);
    }
    return [...byGroup.values()].map((labels) => labels.join('/'));
  });

  const evalSensitivityRows = $derived(parseEvalSensitivity(evalReport?.sensitivity));
  function evalSensitivityFor(weights: readonly number[]) {
    return evalSensitivityRows.find((r) => sameWeights(r.weights, weights)) ?? null;
  }

  function windowSpec(name: string) {
    return catalog?.windows.find((w) => w.name === name);
  }

  const j1Crops = $derived(windowSpec('J1')?.crops ?? ['soja', 'milho_1a']);
  const j1Decs = $derived(windowSpec('J1')?.decendios ?? [28, 29, 30, 31, 32, 33, 34, 35, 36]);
  const j2Crops = $derived([...(windowSpec('J2')?.crops ?? ['milho_2a', 'sorgo_2a', 'milho_2a_braquiaria']), 'pousio']);
  const j3Crops = $derived(windowSpec('J3')?.crops ?? ['soja', 'milho_1a']);
  const j3Decs = $derived(windowSpec('J3')?.decendios ?? [28, 29, 30, 31, 32, 33, 34, 35, 36]);
  const j4Crops = $derived([...(windowSpec('J4')?.crops ?? ['milho_2a', 'sorgo_2a', 'milho_2a_braquiaria']), 'pousio']);

  function fmtNum(value: number | null | undefined, digits = 4): string {
    if (value === null || value === undefined || !Number.isFinite(value)) return '—';
    return value.toFixed(digits);
  }

  function fmtSigned(value: number | null | undefined, digits = 4): string {
    if (value === null || value === undefined || !Number.isFinite(value)) return '—';
    const prefix = value >= 0 ? '+' : '';
    return `${prefix}${value.toFixed(digits)}`;
  }

  function fmtPct(value: number | null | undefined): string {
    if (value === null || value === undefined || !Number.isFinite(value)) return '—';
    return `${(value * 100).toFixed(1)}%`;
  }

  async function executeComparison(inputs: Inputs): Promise<void> {
    formError = '';
    submitting = true;
    try {
      const [nextRun, nextReport] = await Promise.all([
        client.run(inputs),
        client.evaluate(inputs)
      ]);
      activeInputs = {
        ad_class: inputs.ad_class,
        texture: inputs.texture,
        weights: [...inputs.weights],
        p0: inputs.p0 ? { ...inputs.p0 } : null
      };
      runResult = nextRun;
      evalReport = nextReport;
    } catch (err) {
      // Keep the previous results visible; the stale banner says they belong to the previous inputs.
      if (err instanceof OpenApiHttpError) {
        formError = err.message;
      } else {
        formError = err instanceof Error ? err.message : String(err);
      }
    } finally {
      submitting = false;
    }
  }

  async function handleFormSubmit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (weightCheck.kind === 'invalid') {
      // The inline message next to the weight fields already explains the problem.
      document.getElementById('weight-exposure')?.focus();
      return;
    }
    const check = validateInputs(candidateInputs);
    if (check.kind === 'invalid') {
      formError = check.message;
      return;
    }
    await executeComparison(check.inputs);
  }

  async function handleResetDefaults(): Promise<void> {
    const def = catalog?.default_p0 ?? DEFAULT_P0;
    adClass = 'AD4';
    texture = 'argilosa';
    wExposure = 0.5;
    wCoverage = 0.25;
    wDiversity = 0.25;
    p0J1 = def.j1;
    p0Dec1 = def.dec1;
    p0J2 = def.j2;
    p0J3 = def.j3;
    p0Dec3 = def.dec3;
    p0J4 = def.j4;
    await executeComparison({
      ad_class: 'AD4',
      texture: 'argilosa',
      weights: [0.5, 0.25, 0.25],
      p0: { ...def }
    });
  }

  function downloadEvaluationJson(): void {
    if (!runResult || !evalReport) return;
    const blob = new Blob([buildEvaluationExportJson(runResult, evalReport)], {
      type: 'application/json'
    });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement('a');
    anchor.href = url;
    anchor.download = 'evaluation.json';
    anchor.click();
    URL.revokeObjectURL(url);
  }

  onMount(async () => {
    try {
      const [cat, metaResp, provResp, limResp] = await Promise.all([
        client.catalog(),
        client.meta(),
        client.provenance(),
        client.limitations()
      ]);
      catalog = cat;
      meta = metaResp;
      provenance = provResp;
      limitations = limResp;

      p0J1 = cat.default_p0.j1;
      p0Dec1 = cat.default_p0.dec1;
      p0J2 = cat.default_p0.j2;
      p0J3 = cat.default_p0.j3;
      p0Dec3 = cat.default_p0.dec3;
      p0J4 = cat.default_p0.j4;

      await executeComparison({
        ad_class: 'AD4',
        texture: 'argilosa',
        weights: [0.5, 0.25, 0.25],
        p0: { ...cat.default_p0 }
      });
    } catch (err) {
      fatalError = err instanceof Error ? err.message : String(err);
    } finally {
      loading = false;
    }
  });
</script>

<svelte:head>
  <title>FieldShift Optimizer | Climate-aware crop rotation decision support</title>
  <meta
    name="description"
    content="Support rainfed crop rotation decisions in Rio Verde, Goiás with verified NASA POWER data, ZARC constraints and transparent retrospective evaluation."
  />
</svelte:head>

<div class="app-shell">
  <aside class="sidebar">
    <div class="brand">
      <span class="brand-icon">F<span class="brand-dot">.</span></span>
      <span class="brand-word">FieldShift<small>OPTIMIZER</small></span>
    </div>
    <div class="section-label">Workspace</div>
    <nav class="navigation" aria-label="Primary navigation">
      {#each TABS as item}
        <button
          type="button"
          class:active={activeTab === item}
          onclick={() => (activeTab = item)}
        >
          <span class="nav-mark">
            {item === 'Overview'
              ? '◈'
              : item === 'Policy comparison'
                ? '≋'
                : item === 'Pareto and alternatives'
                  ? '◇'
                  : item === 'Retrospective evaluation'
                    ? '▣'
                    : item === 'Weight sensitivity'
                      ? '⊞'
                      : '▤'}
          </span>
          {item}
        </button>
      {/each}
    </nav>
    <div class="sidebar-bottom">
      <div class="sidebar-tag"><span class="status-dot"></span> Verified NASA snapshot</div>
      <p>Rio Verde · Goiás · Brazil</p>
      <p>Rainfed · 2 agricultural years · 4 windows</p>
      <p>NASA POWER / ZARC 2026–27</p>
      <a
        href="https://github.com/Marcux777/fieldshift-optimizer"
        target="_blank"
        rel="noopener noreferrer"
      >
        View source on GitHub ↗
      </a>
    </div>
  </aside>

  <main class="main-content">
    <header class="topbar">
      <div class="breadcrumb">
        FieldShift Optimizer <span>/</span> <strong>{activeTab}</strong>
      </div>
      <div class="topbar-right">
        {#if exploratory}
          <span class="mode-pill exploratory" data-testid="mode-pill">EXPLORATORY RUN</span>
        {:else}
          <span class="mode-pill" data-testid="mode-pill">COMPUTED IN RUST · PROTOCOL v1.1</span>
        {/if}
      </div>
    </header>

    <div class="content-wrap">
      <section class="intro" aria-labelledby="intro-heading" data-testid="tool-intro">
        <h1 id="intro-heading" class="intro-title">Which crop rotation holds up best when the weather varies?</h1>
        <p class="intro-lead">
          FieldShift Optimizer compares a farm's current two-year crop rotation with three optimized alternatives for rainfed fields in Rio Verde, Goiás, using NASA POWER daily climate history and the ZARC sowing-risk limits.
          It is a decision-support tool for farmers and their advisers choosing a rotation under climate uncertainty: it shows how each plan would have fared in past climate years and does not predict yield, profit or future weather.
        </p>
        <details class="glossary" bind:open={glossaryOpen} data-testid="glossary">
          <summary>Glossary: P0–P2, regret, minimax regret, M1–M5, LOO Hamming, ZARC, Pareto, held-out</summary>
          <dl>
            {#each GLOSSARY as g}
              <div class="glossary-entry" id={`glossary-${g.id}`} tabindex="-1">
                <dt>{g.term}</dt>
                <dd>
                  {g.definition}
                  <span class="glossary-source">Source: {g.source}</span>
                </dd>
              </div>
            {/each}
          </dl>
        </details>
      </section>

      <div class="banner-stack">
        <div class="info-banner" role="note" data-testid="disclaimer-banner">
          {DISCLAIMER}
        </div>
        <div class="info-banner" role="note" data-testid="frozen-outcome-banner">
          {FROZEN_OUTCOME_BANNER}
        </div>
      </div>

      <!-- Farm and priorities interactive form -->
      <section class="panel" aria-labelledby="inputs-heading">
        <div class="panel-header">
          <div>
            <span class="overline">Interactive Parameters</span>
            <h2 id="inputs-heading" class="text-lg font-bold">Farm and priorities &amp; editable current plan (P0)</h2>
          </div>
          <span class="text-xs text-slate-600">
            Rio Verde, Goiás · Rainfed farming · Two agricultural years · Four crop windows
          </span>
        </div>

        <form onsubmit={handleFormSubmit} data-testid="inputs-form" novalidate>
          <div class="form-grid-top">
            <div class="field-group">
              <label for="soil-ad-select">Soil water class</label>
              <select id="soil-ad-select" bind:value={adClass}>
                {#each catalog?.soils ?? ['AD1', 'AD2', 'AD3', 'AD4', 'AD5', 'AD6'] as s}
                  <option value={s}>{s}</option>
                {/each}
              </select>
            </div>

            <div class="field-group">
              <label for="soil-texture-select">Soil texture</label>
              <select id="soil-texture-select" bind:value={texture}>
                {#each (catalog?.textures ?? ['argilosa', 'media', 'arenosa']) as tex}
                  <option value={tex}>{TEXTURE_LABELS[tex as Inputs['texture']] ?? tex}</option>
                {/each}
              </select>
            </div>

            <div class="field-group">
              <label for="weight-exposure">Climate exposure weight</label>
              <input
                id="weight-exposure"
                type="number"
                min="0"
                max="1"
                step="0.05"
                aria-invalid={weightsInvalid}
                aria-describedby="weights-status"
                bind:value={wExposure}
              />
            </div>

            <div class="field-group">
              <label for="weight-coverage">Coverage weight</label>
              <input
                id="weight-coverage"
                type="number"
                min="0"
                max="1"
                step="0.05"
                aria-invalid={weightsInvalid}
                aria-describedby="weights-status"
                bind:value={wCoverage}
              />
            </div>

            <div class="field-group">
              <label for="weight-diversity">Diversity weight</label>
              <input
                id="weight-diversity"
                type="number"
                min="0"
                max="1"
                step="0.05"
                aria-invalid={weightsInvalid}
                aria-describedby="weights-status"
                bind:value={wDiversity}
              />
            </div>
          </div>
          <p
            id="weights-status"
            class="weights-status"
            class:invalid={weightsInvalid}
            aria-live="polite"
            data-testid="weights-status"
          >
            {#if weightCheck.kind === 'invalid'}
              {weightCheck.message}
            {:else}
              Weights add up to 1.
            {/if}
          </p>

          <div class="form-grid-p0">
            <div class="field-group">
              <label for="p0-j1">Year 1: main crop (J1)</label>
              <select id="p0-j1" bind:value={p0J1}>
                {#each j1Crops as c}
                  <option value={c}>{cropDisplayName(c)}</option>
                {/each}
              </select>
            </div>

            <div class="field-group">
              <label for="p0-dec1">Year 1: sowing period</label>
              <select id="p0-dec1" bind:value={p0Dec1}>
                {#each j1Decs as d}
                  <option value={d}>{decendioOptionLabel(d)}</option>
                {/each}
              </select>
            </div>

            <div class="field-group">
              <label for="p0-j2">Year 1: second crop (J2)</label>
              <select id="p0-j2" bind:value={p0J2}>
                {#each j2Crops as c}
                  <option value={c}>{cropDisplayName(c)}</option>
                {/each}
              </select>
            </div>

            <div class="field-group">
              <label for="p0-j3">Year 2: main crop (J3)</label>
              <select id="p0-j3" bind:value={p0J3}>
                {#each j3Crops as c}
                  <option value={c}>{cropDisplayName(c)}</option>
                {/each}
              </select>
            </div>

            <div class="field-group">
              <label for="p0-dec3">Year 2: sowing period</label>
              <select id="p0-dec3" bind:value={p0Dec3}>
                {#each j3Decs as d}
                  <option value={d}>{decendioOptionLabel(d)}</option>
                {/each}
              </select>
            </div>

            <div class="field-group">
              <label for="p0-j4">Year 2: second crop (J4)</label>
              <select id="p0-j4" bind:value={p0J4}>
                {#each j4Crops as c}
                  <option value={c}>{cropDisplayName(c)}</option>
                {/each}
              </select>
            </div>
          </div>

          <div class="form-actions">
            <p class="help-text m-0">
              AD4 + clay is a demo assumption, not a measured soil classification. Weights must sum to 1.
              Second-crop sowing follows harvest. ZARC risk is fixed at 20%; an infeasible P0 is excluded from regret comparisons, never repaired silently.
              Do not tune inputs to improve historical test results.
            </p>
            <div class="flex gap-2 items-center">
              <button
                type="button"
                class="btn-secondary"
                onclick={handleResetDefaults}
                disabled={submitting}
              >
                Reset defaults
              </button>
              <button
                type="submit"
                class="btn-primary"
                data-testid="compare-rotations-btn"
                aria-describedby="weights-status"
                disabled={submitting || weightsInvalid}
                title={weightsInvalid ? 'Fix the weights first: they must add up to 1.' : undefined}
              >
                {submitting ? 'Computing in Rust…' : 'Compare rotations'}
              </button>
            </div>
          </div>
        </form>
      </section>

      {#if formError}
        <div class="error-banner mb-4" role="alert" data-testid="form-error">
          {formError}
        </div>
      {/if}

      {#if resultsStale}
        <div class="warning-banner stale-banner mb-4" role="status" data-testid="stale-results-warning">
          Results below are from the previous inputs ({activeInputsSummary}), not from the values now in the form.
          {weightsInvalid ? 'Fix the weights, then select "Compare rotations" to update them.' : 'Select "Compare rotations" to update them.'}
        </div>
      {/if}

      {#if loading}
        <div class="panel" role="status">
          <p class="m-0 font-semibold">Comparing rotations with the verified offline NASA snapshot…</p>
        </div>
      {:else if fatalError}
        <div class="error-banner" role="alert">
          <strong>Research evidence is unavailable:</strong> {fatalError}. The application does not substitute simulated or invented data.
        </div>
      {:else if runResult && evalReport}
        {#if exploratory}
          <div class="warning-banner mb-4" role="status" data-testid="exploratory-warning">
            EXPLORATORY RUN: changed soil, weights or current plan. This result does not replace the frozen AD4/clay evaluation, whose primary outcome was NEGATIVE.
          </div>
        {/if}

        <div class="flex items-baseline justify-between flex-wrap gap-3 mb-4">
          <div>
            <h2 class="text-2xl font-extrabold m-0" data-testid="outcome-heading">
              {outcomeHeading}
            </h2>
            <p class="help-text m-0 mt-1">
              Primary retrospective comparison: P2 vs P1b. All outcomes use the same format.
            </p>
          </div>
          <div class="text-xs text-slate-600 font-mono min-w-0 max-w-full break-all" data-testid="snapshot-config-caption">
            Snapshot: {runResult.snapshot_id} · Configuration: {runResult.config_hash}
          </div>
        </div>

        {#if runResult.status === 'INVIAVEL'}
          <div class="error-banner mb-4" role="alert" data-testid="infeasible-error">
            INFEASIBLE: no rotation satisfies the fixed ZARC and calendar constraints.
          </div>
        {:else if runResult.status === 'DADOS_INSUFICIENTES'}
          <div class="error-banner mb-4" role="alert" data-testid="insufficient-error">
            INSUFFICIENT DATA: too few complete historical pairs; no recommendation.
          </div>
        {:else}
          <!-- Active Tab Content -->
          {#if activeTab === 'Overview'}
            <section class="verdict">
              <div class="verdict-top">
                <span class="verdict-indicator">
                  {exploratory ? 'EXPLORATORY EVALUATION' : 'SCIENTIFIC RESULT'}
                </span>
                <span>PROTOCOL v{runResult.protocol_version} · HELD-OUT TEST (2010/11–2024/25)</span>
              </div>
              <div class="verdict-flex">
                <div>
                  <h2>{evalReport.headline}</h2>
                  <p>
                    Out-of-sample worst-case regret for P2 ({fmtNum(primaryComparison?.m1_a)}) vs P1b ({fmtNum(primaryComparison?.m1_b)}).
                    95% block-bootstrap CI of median difference spans [{fmtSigned(primaryComparison?.ci95_median_D?.[0])}, {fmtSigned(primaryComparison?.ci95_median_D?.[1])}].
                    The contribution is a reproducible comparison tool, not method superiority.
                  </p>
                </div>
                <div class="verdict-number">
                  <span>{fmtNum(primaryComparison?.m1_a)}</span>
                  <small>P2 worst test regret</small>
                </div>
              </div>
            </section>

            <div class="kpi-grid">
              <div class="kpi">
                <span>Feasible rotations</span>
                <strong data-testid="kpi-n-plans">{runResult.n_plans}</strong>
                <small>ZARC ≤20% risk constraint</small>
              </div>
              <div class="kpi">
                <span>Pareto objective vectors</span>
                <strong data-testid="kpi-n-front">{runResult.pareto.n_front}</strong>
                <small>Four-dimensional frontier</small>
              </div>
              <div class="kpi">
                <span>Historical test pairs</span>
                <strong>{evalReport.test.length}</strong>
                <small>7 non-overlapping pairs</small>
              </div>
              <div class="kpi">
                <span>Robustness price (M5)</span>
                <strong>{fmtNum(evalReport.M5_price_of_robustness)}</strong>
                <small>Mean-climate loss gap</small>
              </div>
            </div>

            <div class="two-column">
              <section class="panel">
                <div class="panel-header">
                  <div>
                    <span class="overline">Test Performance</span>
                    <h3 class="text-base font-bold">
                      Worst-case regret by strategy
                      (<a class="term" href="#glossary-m1" title={glossaryTitle('m1')} onclick={(e) => openGlossary(e, 'm1')}>M1</a>)
                    </h3>
                  </div>
                  <span class="text-xs text-slate-600">Lower is better</span>
                </div>
                <div class="bar-chart">
                  {#each POLICY_KEYS as key}
                    {@const pm = evalReport.policies[key]}
                    {@const m1 = pm?.M1_max_regret ?? null}
                    <div>
                      <div class="bar-label">
                        <span>{key} <em>{POLICY_LABELS[key]}</em></span>
                        <strong>{pm?.feasible ? fmtNum(m1) : 'Infeasible (ZARC > 20%)'}</strong>
                      </div>
                      <div class="track">
                        <div
                          class:highlight={key === 'P2'}
                          class:baseline={key === 'P1b'}
                          class="bar-fill"
                          style:width={`${m1 !== null ? Math.min(100, (m1 / 0.35) * 100) : 0}%`}
                        ></div>
                      </div>
                    </div>
                  {/each}
                </div>
              </section>

              <section class="panel">
                <div class="panel-header">
                  <div>
                    <span class="overline">Primary Comparison</span>
                    <h3 class="text-base font-bold">
                      P2 (<a class="term" href="#glossary-minimax-regret" title={glossaryTitle('minimax-regret')} onclick={(e) => openGlossary(e, 'minimax-regret')}>Minimax regret</a>) vs P1b (Expected loss)
                    </h3>
                  </div>
                </div>
                <div class="comparison-metrics">
                  <div>
                    <span>P1b worst regret</span>
                    <strong>{fmtNum(primaryComparison?.m1_b)}</strong>
                  </div>
                  <div>
                    <span>P2 worst regret</span>
                    <strong>{fmtNum(primaryComparison?.m1_a)}</strong>
                  </div>
                  <div>
                    <span>Median paired difference</span>
                    <strong>{fmtSigned(primaryComparison?.median_D)}</strong>
                  </div>
                </div>
                <div class="ci-card">
                  <span>95% block-bootstrap CI of median difference</span>
                  <strong>
                    [{fmtSigned(primaryComparison?.ci95_median_D?.[0])}, {fmtSigned(primaryComparison?.ci95_median_D?.[1])}]
                  </strong>
                  <small>
                    Wins / ties / losses: {primaryComparison?.wins ?? 0} / {primaryComparison?.ties ?? 0} / {primaryComparison?.losses ?? 0} · Classification: {formatOutcomeLabel(primaryComparison?.classification ?? '')}
                  </small>
                  {#if primaryComparison}
                    <small data-testid="primary-classification-reason">
                      {comparisonReasonLabel(primaryComparison.reason)}
                      <a href="#classification-rule">See the classification rule.</a>
                    </small>
                  {/if}
                </div>
              </section>
            </div>
          {/if}

          {#if activeTab === 'Policy comparison' || activeTab === 'Overview'}
            <section class="panel" data-testid="policy-comparison-section">
              <div class="panel-header">
                <div>
                  <span class="overline">Policy Comparison</span>
                  <h3 class="text-base font-bold">
                    {runResult.n_plans} feasible plans · Plans are selected using construction data only
                  </h3>
                </div>
              </div>

              <div class="table-scroll mb-4">
                <table class="data-table">
                  <thead>
                    <tr>
                      <th>Policy</th>
                      <th>Feasible</th>
                      <th>Plan (crop and sowing per window)</th>
                      <th>Max test regret (M1)</th>
                      <th>Mean test regret (M2)</th>
                      <th>Mean test loss</th>
                    </tr>
                  </thead>
                  <tbody>
                    {#each POLICY_KEYS as key}
                      {@const pm = evalReport.policies[key]}
                      <tr>
                        <td><strong>{key}: {POLICY_LABELS[key]}</strong></td>
                        <td>{pm?.feasible ? 'Yes' : 'No'}</td>
                        <td class="plan-cell">
                          {#if pm?.plan_id}
                            <PlanTimeline planId={pm.plan_id} {catalog} compact />
                          {:else}
                            —
                          {/if}
                        </td>
                        <td>{fmtNum(pm?.M1_max_regret)}</td>
                        <td>{fmtNum(pm?.M2_mean_regret)}</td>
                        <td>{fmtNum(pm?.mean_loss_test)}</td>
                      </tr>
                    {/each}
                  </tbody>
                </table>
              </div>

              <div class="policy-grid">
                {#each POLICY_KEYS as key}
                  {@const entry = runResult.policies[key]}
                  <div class="policy-card" data-testid={`policy-card-${key}`}>
                    <div class="flex items-center justify-between mb-2">
                      <strong>{key}: {POLICY_LABELS[key]}</strong>
                      <span class="text-xs font-mono px-2 py-0.5 rounded bg-slate-100">
                        {entry?.status ?? '—'}
                      </span>
                    </div>
                    <p class="help-text m-0 mb-3">{POLICY_DESCRIPTIONS[key]}</p>

                    {#if entry && entry.status !== 'OK'}
                      <div class="warning-banner mb-2" data-testid={`p0-infeasible-warning-${key}`}>
                        INFEASIBLE (INVIAVEL_SOB_ZARC): P0 violates ZARC 20% / calendar rules and is excluded from regret. Feasible alternatives remain available.
                      </div>
                      {#each formatP0Diagnostics(entry.reasons) as diag}
                        <p class="text-xs text-amber-900 m-0 mb-1" data-testid="p0-diagnostic-line">
                          {diag}
                        </p>
                      {/each}
                    {:else if entry?.plan}
                      <div class="table-scroll mb-3">
                        <table class="data-table">
                          <thead>
                            <tr>
                              <th>Window</th>
                              <th>Crop</th>
                              <th>Sowing</th>
                              <th>Harvest</th>
                            </tr>
                          </thead>
                          <tbody>
                            {#each entry.plan.occupations as occ}
                              <tr>
                                <td>{occ.window}</td>
                                <td>{cropDisplayName(occ.crop)}</td>
                                <td>{dateLabelFromT(occ.sow_t)}</td>
                                <td>{dateLabelFromT(occ.harvest_t)}</td>
                              </tr>
                            {/each}
                          </tbody>
                        </table>
                      </div>
                      {#if entry.metrics_construction}
                        <p class="text-xs text-slate-700 m-0 mb-1">
                          Construction mean loss: <strong>{fmtNum(entry.metrics_construction.mean_L)}</strong> ·
                          worst regret: <strong>{fmtNum(entry.metrics_construction.max_regret)}</strong> ·
                          mean-climate loss: <strong>{fmtNum(entry.metrics_construction.L_sbar)}</strong>
                        </p>
                      {/if}
                      {#if key === 'P1' || key === 'P1b' || key === 'P2'}
                        {@const changed = countChangedWeightSettings(runResult.sensitivity, key, entry.plan.id)}
                        <p class="text-xs text-slate-600 m-0">
                          Selection changes at {changed.length} of 15 fixed weight settings.
                        </p>
                      {/if}
                    {/if}
                  </div>
                {/each}
              </div>
            </section>
          {/if}

          {#if activeTab === 'Pareto and alternatives' || activeTab === 'Overview'}
            <section class="panel" data-testid="pareto-section">
              <div class="panel-header">
                <div>
                  <span class="overline">Multi-Objective Trade-Offs</span>
                  <h3 class="text-base font-bold">
                    {runResult.pareto.n_front} non-dominated objective vectors across {runResult.n_plans} feasible rotations
                  </h3>
                </div>
                <span class="text-xs text-slate-600">Construction data only · Lower is better</span>
              </div>
              <p class="help-text">
                Two-dimensional view of the four-objective
                <a class="term" href="#glossary-pareto" title={glossaryTitle('pareto')} onclick={(e) => openGlossary(e, 'pareto')}>Pareto front</a>,
                computed on construction years only. Dominance uses all four objectives; worst exposure is in each point's tooltip and in the table.
              </p>

              <div class="mb-4">
                <ParetoChart front={frontObjectiveRows} policies={policyChartPoints} {catalog} />
              </div>

              <div class="info-banner mb-3" role="note" data-testid="pareto-grouping-note">
                <strong>How alternatives are grouped:</strong> the optimizer keeps one alternative per crop sequence (the crop chosen for each window J1–J4) and ignores sowing dates when grouping.
                {#if groupedPolicyNote.length > 0}
                  {groupedPolicyNote.join(', ')} share a crop sequence but differ in sowing dates, so each policy's own plan, ID and objective values are listed on its own row (same group number).
                {/if}
                "Min. crop difference" counts windows whose crop differs from the nearest other group.
              </div>

              <div class="table-scroll">
                <table class="data-table" data-testid="alternatives-table">
                  <thead>
                    <tr>
                      <th>Group</th>
                      <th>Policy / reason</th>
                      <th>Plan (crop and sowing per window)</th>
                      <th>On front</th>
                      <th>Objectives (mean exp / worst exp / cov / div)</th>
                      <th>Min. crop difference</th>
                    </tr>
                  </thead>
                  <tbody>
                    {#each alternativeRows as row}
                      <tr class:group-member={row.groupSize > 1}>
                        <td>{row.group}{row.groupSize > 1 ? ` (${row.groupSize} plans)` : ''}</td>
                        <td>
                          <strong>{row.policies.join(', ') || 'Alternative'}</strong>
                          <span class="cell-note">{ALTERNATIVE_REASON_LABELS[row.reason] ?? row.reason}</span>
                        </td>
                        <td class="plan-cell"><PlanTimeline planId={row.planId} {catalog} compact /></td>
                        <td>{row.onFront ? 'Yes' : 'No'}</td>
                        <td>
                          {row.objectives ? row.objectives.map((v) => fmtNum(v)).join(' / ') : '—'}
                        </td>
                        <td>{row.minHammingToOthers}{row.hammingRelaxed ? ' (relaxed)' : ''}</td>
                      </tr>
                    {/each}
                  </tbody>
                </table>
              </div>
            </section>
          {/if}

          {#if activeTab === 'Retrospective evaluation' || activeTab === 'Overview'}
            <section class="panel" data-testid="retrospective-evaluation-section">
              <div class="panel-header">
                <div>
                  <span class="overline">Retrospective Evaluation</span>
                  <h3 class="text-base font-bold">
                    Construction: 1981/82–2009/10 ({evalReport.construction.length} valid pairs) · Test: 2010/11–2024/25 ({evalReport.test.length} valid pairs) · Excluded: {evalReport.excluded_scenarios.length}
                  </h3>
                </div>
                <button type="button" class="btn-secondary" onclick={downloadEvaluationJson}>
                  Download evaluation JSON
                </button>
              </div>

              <div class="table-scroll mb-3">
                <table class="data-table" data-testid="comparisons-table">
                  <thead>
                    <tr>
                      <th>Comparison</th>
                      <th>Classification</th>
                      <th>Why (protocol rule applied)</th>
                      <th>Max regret (A vs B)</th>
                      <th>Median Δ regret</th>
                      <th>95% CI (median Δ)</th>
                      <th>Wins / Ties / Losses</th>
                      <th>Wilcoxon non-overlap (p)</th>
                    </tr>
                  </thead>
                  <tbody>
                    {#each evalReport.comparisons as c}
                      <tr>
                        <td><strong>{c.a} vs {c.b}</strong></td>
                        <td><strong>{formatOutcomeLabel(c.classification)}</strong></td>
                        <td class="reason-cell">{comparisonReasonLabel(c.reason)}</td>
                        <td>{fmtNum(c.m1_a)} vs {fmtNum(c.m1_b)}</td>
                        <td>{fmtSigned(c.median_D)}</td>
                        <td>
                          {c.ci95_median_D
                            ? `[${fmtSigned(c.ci95_median_D[0])}, ${fmtSigned(c.ci95_median_D[1])}]`
                            : '—'}
                        </td>
                        <td>{c.wins} / {c.ties} / {c.losses}</td>
                        <td>
                          {c.wilcoxon_nonoverlap
                            ? `${fmtNum(c.wilcoxon_nonoverlap.p_two_sided)} (n≠0=${c.wilcoxon_nonoverlap.n_nonzero ?? 0})`
                            : '—'}
                        </td>
                      </tr>
                    {/each}
                  </tbody>
                </table>
              </div>

              <aside class="rule-box mb-4" id="classification-rule" aria-labelledby="classification-rule-heading" data-testid="classification-rule">
                <h4 id="classification-rule-heading">How each comparison is classified (frozen before any test result)</h4>
                <p>{CLASSIFICATION_RULE.d}</p>
                <ul>
                  <li>{CLASSIFICATION_RULE.supported}</li>
                  <li>{CLASSIFICATION_RULE.negative}</li>
                  <li>{CLASSIFICATION_RULE.inconclusive}</li>
                </ul>
                <p><strong>Why a lower maximum regret is not enough:</strong> {CLASSIFICATION_RULE.whyNotM1}</p>
                <p class="rule-source">Source: {CLASSIFICATION_RULE.source}.</p>
              </aside>

              <p class="help-text">
                Dry test pairs: {evalReport.dry_test_scenarios.length} ({evalReport.dry_test_scenarios.join(', ') || 'none'});
                construction dry threshold (P25 Oct–Mar): {fmtNum(evalReport.dry_threshold_mm, 1)} mm.
                Price of robustness in mean climate (M5): {fmtNum(evalReport.M5_price_of_robustness)}.
                M0 feasibility: {fmtPct(evalReport.M0_feasibility)}.
              </p>

              <div class="table-scroll">
                <table class="data-table">
                  <thead>
                    <tr>
                      <th>Policy</th>
                      <th>Feasible</th>
                      <th>
                        <a class="term" href="#glossary-m0-m5" title={glossaryTitle('m0-m5')} onclick={(e) => openGlossary(e, 'm0-m5')}>M1 max regret</a>
                      </th>
                      <th>M2 mean regret</th>
                      <th>M2 median regret</th>
                      <th>M3 dry loss</th>
                      <th>M3 dry regret</th>
                      <th>M4a regret ≤ ε</th>
                      <th>M4b IQR loss</th>
                      <th>
                        <a class="term" href="#glossary-m4c" title={glossaryTitle('m4c')} onclick={(e) => openGlossary(e, 'm4c')}>M4c LOO Hamming</a>
                      </th>
                    </tr>
                  </thead>
                  <tbody>
                    {#each POLICY_KEYS as key}
                      {@const pm = evalReport.policies[key]}
                      <tr>
                        <td><strong>{key}</strong></td>
                        <td>{pm?.feasible ? 'Yes' : 'No'}</td>
                        <td>{fmtNum(pm?.M1_max_regret)}</td>
                        <td>{fmtNum(pm?.M2_mean_regret)}</td>
                        <td>{fmtNum(pm?.M2_median_regret)}</td>
                        <td>{fmtNum(pm?.M3_dry_mean_loss)}</td>
                        <td>{fmtNum(pm?.M3_dry_mean_regret)}</td>
                        <td>{fmtPct(pm?.M4a_frac_regret_le_eps)}</td>
                        <td>{fmtNum(pm?.M4b_iqr_loss)}</td>
                        <td>{fmtNum(pm?.M4c_loo_mean_hamming, 2)}</td>
                      </tr>
                    {/each}
                  </tbody>
                </table>
              </div>
            </section>
          {/if}

          {#if activeTab === 'Weight sensitivity' || activeTab === 'Overview'}
            <section class="panel" data-testid="weight-sensitivity-section">
              <div class="panel-header">
                <div>
                  <span class="overline">Weight Sensitivity</span>
                  <h3 class="text-base font-bold">
                    All {runResult.sensitivity.length} fixed weight combinations (construction selection &amp; held-out classification)
                  </h3>
                </div>
              </div>
              <p class="help-text">
                For each fixed weight setting, P1, P1b and P2 are re-selected on construction years and classified on the
                <a class="term" href="#glossary-held-out" title={glossaryTitle('held-out')} onclick={(e) => openGlossary(e, 'held-out')}>held-out</a>
                test years with the same rule as the main result. This grid is reported only: it never chooses the weights and never replaces the verdict at the default weights 0.50 / 0.25 / 0.25 (protocol v1.1 §3, §5). Test results must not be used to choose weights; changed inputs are exploratory.
              </p>
              <div class="table-scroll">
                <table class="data-table" data-testid="sensitivity-table">
                  <thead>
                    <tr>
                      <th>#</th>
                      <th>Weights (Exposure / Coverage / Diversity)</th>
                      <th>P2 vs P1b (held-out)</th>
                      <th>P2 vs P1 (held-out)</th>
                      <th>M1 test: P1 / P1b / P2</th>
                      <th>P1 plan</th>
                      <th>P1b plan</th>
                      <th>P2 plan</th>
                    </tr>
                  </thead>
                  <tbody>
                    {#each runResult.sensitivity as row, idx}
                      {@const ev = evalSensitivityFor(row.weights)}
                      <tr class:default-row={ev?.isDefault}>
                        <td>{idx + 1}</td>
                        <td>
                          {row.weights.map((w) => w.toFixed(2)).join(' / ')}
                          {#if ev?.isDefault}<span class="cell-note">default weights</span>{/if}
                        </td>
                        <td data-testid="sensitivity-class-p1b"><strong>{ev?.p2VsP1b ? formatOutcomeLabel(ev.p2VsP1b) : '—'}</strong></td>
                        <td data-testid="sensitivity-class-p1"><strong>{ev?.p2VsP1 ? formatOutcomeLabel(ev.p2VsP1) : '—'}</strong></td>
                        <td>{ev ? `${fmtNum(ev.m1.P1)} / ${fmtNum(ev.m1.P1b)} / ${fmtNum(ev.m1.P2)}` : '—'}</td>
                        <td class="plan-cell-inline"><PlanTimeline planId={row.P1} {catalog} inline /></td>
                        <td class="plan-cell-inline"><PlanTimeline planId={row.P1b} {catalog} inline /></td>
                        <td class="plan-cell-inline"><PlanTimeline planId={row.P2} {catalog} inline /></td>
                      </tr>
                    {/each}
                  </tbody>
                </table>
              </div>
            </section>
          {/if}
        {/if}

        {#if activeTab === 'Sources and limitations' || activeTab === 'Overview'}
          <section class="panel" data-testid="sources-section">
            <div class="panel-header">
              <div>
                <span class="overline">Traceable Provenance</span>
                <h3 class="text-base font-bold">Sources, assumptions, uncertainties and limitations</h3>
              </div>
            </div>

            <div class="evidence-grid mb-4">
              <div>
                <small>NASA POWER snapshot ID</small>
                <strong>{provenance?.manifest.snapshot_id ?? runResult.snapshot_id}</strong>
              </div>
              <div>
                <small>NASA POWER API version &amp; access time (UTC)</small>
                <strong>
                  NASA POWER API version: {provenance?.manifest.source.api_version ?? 'v2.10.3'} · {provenance?.manifest.collected_utc ?? '—'}
                </strong>
              </div>
              <div>
                <small>Optimizer RunResult SHA-256</small>
                <code data-testid="run-result-hash">{runResult.result_hash}</code>
              </div>
              <div>
                <small>Evaluator EvalReport SHA-256</small>
                <code data-testid="eval-result-hash">{evalReport.result_hash}</code>
              </div>
              <div>
                <small>Frozen Protocol v{meta?.protocol_version ?? runResult.protocol_version} SHA-256</small>
                <code>{provenance?.protocol_sha256 ?? meta?.protocol_sha256 ?? '—'}</code>
              </div>
              <div>
                <small>Daily grid cell &amp; units</small>
                <strong>−18.0°, −50.625° (Rio Verde, GO) · PRECTOTCORR (mm/day), T2M/MAX/MIN (°C)</strong>
              </div>
            </div>

            {#if provenance?.manifest.citation}
              <p class="help-text">
                <strong>Citation:</strong> {provenance.manifest.citation.reference}
                {provenance.manifest.citation.data_reference}
              </p>
            {/if}

            <h4 class="text-sm font-bold mt-4 mb-2">Known limitations &amp; modeling assumptions</h4>
            <ul class="limitations">
              {#each limitations?.items ?? evalReport.limitations as item}
                <li>{item}</li>
              {/each}
            </ul>

            <div class="evidence-links">
              <a
                href={provenance?.manifest.citation.referencing_guide ?? 'https://power.larc.nasa.gov/'}
                target="_blank"
                rel="noopener noreferrer"
              >
                NASA POWER referencing guide ↗
              </a>
              <a
                href="https://dados.agricultura.gov.br/"
                target="_blank"
                rel="noopener noreferrer"
              >
                ZARC / MAPA open data ↗
              </a>
              <a
                href="https://github.com/Marcux777/fieldshift-optimizer/blob/main/docs/EVALUATION_PROTOCOL.md"
                target="_blank"
                rel="noopener noreferrer"
              >
                Frozen evaluation protocol v1.1 ↗
              </a>
            </div>
          </section>
        {/if}

        <footer>
          <span>Decision-support only. No claims of yield, profit, water savings, soil recovery or method superiority.</span>
          <span>FieldShift Optimizer · NASA Space Apps 2026</span>
        </footer>
      {/if}
    </div>
  </main>
</div>
