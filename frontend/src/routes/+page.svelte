<script lang="ts">
  // AI-assisted migration UI. All scientific values come from frozen, hash-verified reports.
  import { onMount } from 'svelte';
  import '../app.css';

  type Comparison = { a: string; b: string; classification: string; m1_a: number;
    m1_b: number; median_D: number; ci95_median_D: number[]; wins: number;
    ties: number; losses: number; n: number; };
  type TestPolicy = { M1_max_regret: number; M2_mean_regret: number; mean_loss_test: number;
    M4a_frac_regret_le_eps: number; plan_id: number[] };
  type Report = { outcome: string; headline: string; n_plans: number; test: string[];
    comparisons: Comparison[]; M5_price_of_robustness: number;
    policies: Record<string, TestPolicy>; limitations: string[]; result_hash: string };
  type Occupation = { window: string; label: string; sow: string; zarc_dec: number };
  type Policy = { status: string; plan?: { occupations: Occupation[] };
    metrics_construction?: { max_regret: number; mean_L: number; L_sbar: number } };
  type Sensitivity = { weights: number[]; P1: number[]; P1b: number[]; P2: number[] };
  type Run = { status: string; n_plans: number; snapshot_id: string; protocol_version: string;
    policies: Record<string, Policy>;
    objectives: [number[], number, number, number, number][];
    pareto: { n_front: number; front_ids: number[][] };
    sensitivity: Sensitivity[]; result_hash: string; weights: number[] };

  const api = (import.meta.env.VITE_API_BASE_URL || 'http://localhost:3000').replace(/\/$/, '');
  const names: Record<string, string> = {
    P0: 'Current practice', P1: 'Mean climate', P1b: 'Expected loss', P2: 'Minimax regret'
  };
  const order = ['P0', 'P1', 'P1b', 'P2'];
  const tabs = ['Overview', 'Strategies', 'Pareto frontier', 'Evidence'];
  let tab = 'Overview';
  let report: Report | null = null;
  let run: Run | null = null;
  let selected = 'P2';
  let sensitivityIndex = 4;
  let error = '';
  let loading = true;
  $: primary = report?.comparisons.find(c => c.a === 'P2' && c.b === 'P1b');
  $: currentTest = report?.policies[selected];
  $: currentPlan = run?.policies[selected];
  $: sensitivity = run?.sensitivity[sensitivityIndex];
  $: frontSet = new Set((run?.pareto.front_ids ?? []).map(id => id.join(',')));
  $: front = (run?.objectives ?? []).filter(row => frontSet.has(row[0].join(',')));
  const n = (value: number | undefined, digits = 4) =>
    value === undefined ? '—' : value.toLocaleString('en-US', {
      minimumFractionDigits: digits, maximumFractionDigits: digits
    });
  const percent = (value: number) => `${(value * 100).toFixed(1)}%`;
  onMount(async () => {
    try {
      const responses = await Promise.all([
        fetch(`${api}/api/v1/report`),
        fetch(`${api}/api/v1/run`)
      ]);
      if (responses.some(r => !r.ok)) throw new Error('Rust API returned an error');
      const [resultReport, resultRun] = await Promise.all(responses.map(r => r.json()));
      if (resultReport.outcome !== 'NEGATIVO' || resultRun.protocol_version !== '1.1')
        throw new Error('Frozen evidence version is unexpected');
      report = resultReport as Report;
      run = resultRun as Run;
      const defaultIndex = run.sensitivity.findIndex(row =>
        row.weights.join(',') === '0.5,0.25,0.25');
      sensitivityIndex = defaultIndex < 0 ? 0 : defaultIndex;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  });
</script>

<svelte:head>
  <title>FieldShift | Climate-aware rotation explorer</title>
  <meta name="description" content="Explore rigorously evaluated rotation trade-offs with NASA POWER data. Negative scientific findings are reported transparently." />
</svelte:head>

<div class="app-shell">
  <aside class="sidebar">
    <div class="brand"><span class="brand-icon">F<span class="brand-dot">.</span></span>
      <span class="brand-word">FieldShift<small>OPTIMIZER</small></span></div>
    <div class="section-label">WORKSPACE</div>
    <nav class="navigation" aria-label="Primary navigation">
      {#each tabs as item}
        <button class:active={tab === item} onclick={() => tab = item}>
          <span class="nav-mark">{item === 'Overview' ? '◈' : item === 'Strategies' ? '≋' : item === 'Pareto frontier' ? '◇' : '▤'}</span>
          {item}
        </button>
      {/each}
    </nav>
    <div class="sidebar-bottom">
      <div class="sidebar-tag"><span class="status-dot"></span> Historical dataset</div>
      <p>Rio Verde · Goiás · Brazil</p>
      <p>NASA POWER / ZARC 2026–27</p>
      <a href="https://github.com/Marcux777/fieldshift-optimizer" target="_blank" rel="noopener noreferrer">View source on GitHub ↗</a>
    </div>
  </aside>

  <main class="main-content">
    <header class="topbar">
      <div class="breadcrumb">Research workspace <span>/</span> <strong>{tab}</strong></div>
      <div class="topbar-right"><span class="mode-pill">FROZEN EVIDENCE</span><span class="round-avatar">FS</span></div>
    </header>
    <div class="content-wrap">
      {#if loading}
        <div class="empty-state"><div class="spinner"></div>Loading verified research artifacts…</div>
      {:else if error}
        <div class="error-panel" role="alert">
          <h2>Research evidence is unavailable</h2>
          <p>{error}. The application does not substitute simulated or invented data.</p>
          <p>Start the Rust backend or configure <code>VITE_API_BASE_URL</code> for the deployed API.</p>
        </div>
      {:else if report && run}
        <div class="eyebrow">NASA SPACE APPS 2026 <span>·</span> FIELD SHIFT CHALLENGE</div>
        <div class="heading">
          <div><h1>{tab === 'Overview' ? 'Climate decisions, without guesswork.' : tab}</h1>
            <p class="subtitle">Explore crop rotation alternatives under climate uncertainty, with honest out-of-sample evidence.</p></div>
          <div class="dataset-label"><span class="tiny-dot"></span> Historical test: 2010/11—2024/25</div>
        </div>

        {#if tab === 'Overview'}
          <section class="verdict">
            <div class="verdict-top"><span class="verdict-indicator">SCIENTIFIC RESULT</span><span>PROTOCOL v{run.protocol_version} · HELD-OUT TEST</span></div>
            <div class="verdict-flex">
              <div><h2>Minimax regret did not outperform the stronger baseline.</h2>
                <p>Out-of-sample worst-case regret was higher for P2 than P1b. The confidence interval spans zero; statistical power is limited. The contribution is a reproducible comparison tool, not method superiority.</p></div>
              <div class="verdict-number"><span>{n(primary?.m1_a)}</span><small>P2 worst regret</small></div>
            </div>
          </section>
          <div class="kpi-grid">
            <div class="kpi"><span>Feasible rotations</span><strong>{run.n_plans}</strong><small>ZARC ≤20% risk constraint</small></div>
            <div class="kpi"><span>Pareto objective vectors</span><strong>{run.pareto.n_front}</strong><small>Four-dimensional frontier</small></div>
            <div class="kpi"><span>Historical test pairs</span><strong>{report.test.length}</strong><small>7 non-overlapping pairs</small></div>
            <div class="kpi"><span>Robustness price (M5)</span><strong>{n(report.M5_price_of_robustness)}</strong><small>Mean-climate loss gap</small></div>
          </div>
          <div class="two-column">
            <section class="panel">
              <div class="panel-header"><div><span class="overline">TEST PERFORMANCE</span><h3>Worst-case regret by strategy</h3></div><span class="muted">Lower is better</span></div>
              <div class="bar-chart">
                {#each order as key}
                  <div class="bar-group">
                    <div class="bar-label"><span>{key} <em>{names[key]}</em></span><strong>{n(report.policies[key]?.M1_max_regret)}</strong></div>
                    <div class="track"><div class:highlight={key === 'P2'} class:baseline={key === 'P1b'} class="bar-fill" style:width={`${(report.policies[key].M1_max_regret / 0.35) * 100}%`}></div></div>
                  </div>
                {/each}
              </div>
              <div class="chart-footer"><span><i class="legend-p1b"></i> Stronger baseline (P1b)</span><span><i class="legend-p2"></i> Minimax regret (P2)</span></div>
            </section>
            <section class="panel">
              <div class="panel-header"><div><span class="overline">PRIMARY COMPARISON</span><h3>P2 versus expected loss</h3></div></div>
              <div class="comparison-metrics">
                <div><span>P1b worst regret</span><strong>{n(primary?.m1_b)}</strong></div>
                <div><span>P2 worst regret</span><strong>{n(primary?.m1_a)}</strong></div>
                <div><span>Median paired difference</span><strong>{n(primary?.median_D, 6)}</strong></div>
              </div>
              <div class="ci-card"><span>95% CI of median difference</span><strong>[{n(primary?.ci95_median_D[0], 4)}, +{n(primary?.ci95_median_D[1], 4)}]</strong><small>Includes zero · low inferential power</small></div>
              <p class="help-text">The retrospective outcome is negative under the pre-registered classification. It does not establish that minimax regret is universally worse.</p>
            </section>
          </div>
          <section class="strip"><span>REPRODUCIBILITY</span><strong>Read-only exploration of verified, frozen evaluation artifacts.</strong><button onclick={() => tab='Evidence'}>Inspect evidence →</button></section>
        {:else if tab === 'Strategies'}
          <section class="panel">
            <div class="panel-header"><div><span class="overline">POLICY EXPLORER</span><h3>Compare the four decision rules</h3></div><span class="mode-label">Frozen test results</span></div>
            <div class="policy-grid">
              {#each order as key}
                <button class:chosen={selected === key} class="policy-choice" onclick={() => selected = key}>
                  <span>{key}</span><strong>{names[key]}</strong><small>Worst regret · {n(report.policies[key]?.M1_max_regret)}</small>
                </button>
              {/each}
            </div>
            <div class="policy-body">
              <div><span class="overline">ROTATION PLAN · {selected}</span><h3>{names[selected]}</h3><p class="help-text">Feasible calendar generated by the historical Python baseline; no new optimization is being performed.</p></div>
              <div class="metrics-row"><div><small>Test mean loss</small><strong>{n(currentTest?.mean_loss_test)}</strong></div><div><small>Test mean regret</small><strong>{n(currentTest?.M2_mean_regret)}</strong></div><div><small>Regret ≤ 0.02</small><strong>{currentTest ? percent(currentTest.M4a_frac_regret_le_eps) : '—'}</strong></div></div>
            </div>
            <div class="calendar-grid">
              {#each currentPlan?.plan?.occupations ?? [] as crop}
                <div class="calendar-cell"><span>{crop.window} <small>DEC {crop.zarc_dec}</small></span><strong>{crop.label}</strong><small>Sowing · {crop.sow}</small></div>
              {/each}
            </div>
          </section>
          <section class="panel sensitivity-panel">
            <div><span class="overline">CONSTRUCTION-ONLY SENSITIVITY</span><h3>Explore the 15 precomputed weights</h3><p class="help-text">These selections are taken from the frozen grid; they are not new test evaluations or live reoptimizations.</p></div>
            <label for="weights" class="input-label">Exposure / cover / diversity</label>
            <select id="weights" bind:value={sensitivityIndex}>
              {#each run.sensitivity as point, i}
                <option value={i}>{point.weights.map(w => n(w, 2)).join(' / ')}</option>
              {/each}
            </select>
            {#if sensitivity}
              <div class="sensitivity-grid">
                {#each ['P1','P1b','P2'] as key}
                  <div><small>{key} · {names[key]}</small><strong>({sensitivity[key as keyof Sensitivity] instanceof Array ? (sensitivity[key as keyof Sensitivity] as number[]).join(', ') : '—'})</strong></div>
                {/each}
              </div>
            {/if}
          </section>
        {:else if tab === 'Pareto frontier'}
          <section class="panel">
            <div class="panel-header"><div><span class="overline">MULTI-OBJECTIVE TRADE-OFFS</span><h3>Non-dominated crop rotation plans</h3></div><span class="mode-label">{run.pareto.n_front} objective vectors</span></div>
            <p class="help-text">2D projection: mean climate exposure versus vegetation cover discontinuity. Dominance is calculated in all four objective dimensions, not solely the visible axes. Lower is better.</p>
            <div class="plot-shell">
              <svg viewBox="0 0 700 420" role="img" aria-label="Scatter plot of non-dominated rotation plans by mean exposure and cover discontinuity">
                {#each [0,0.25,0.5,0.75,1] as value}
                  <line x1={56+value*595} y1="24" x2={56+value*595} y2="366" stroke="#e2eae4" stroke-dasharray="3 5" />
                  <line x1="56" y1={366-value*342} x2="651" y2={366-value*342} stroke="#e2eae4" stroke-dasharray="3 5" />
                  <text x={56+value*595} y="387" text-anchor="middle">{value.toFixed(2)}</text>
                  <text x="46" y={370-value*342} text-anchor="end">{value.toFixed(2)}</text>
                {/each}
                {#each front as point}
                  <circle cx={56+point[1]*595} cy={366-point[3]*342} r="6.8" fill={`hsl(${145 - point[4]*95} 37% 43%)`} fill-opacity="0.82" stroke="white" stroke-width="1.7">
                    <title>Plan {point[0].join(',')} · exposure {n(point[1])} · coverage {n(point[3])} · diversity {n(point[4])}</title>
                  </circle>
                {/each}
                <text x="352" y="414" text-anchor="middle" class="axis-title">Mean climate exposure (g₁)</text>
                <text x="-200" y="15" transform="rotate(-90)" text-anchor="middle" class="axis-title">Cover discontinuity (g₂)</text>
              </svg>
            </div>
            <div class="plot-bottom"><span>Color progression encodes diversity deficit (g₃).</span><span>Construction only · {run.n_plans} feasible candidates</span></div>
          </section>
        {:else if tab === 'Evidence'}
          <section class="panel">
            <div class="panel-header"><div><span class="overline">TRACEABLE RESEARCH</span><h3>Data, method, and limitations</h3></div></div>
            <div class="evidence-grid">
              <div><small>NASA POWER snapshot</small><strong>{run.snapshot_id}</strong></div>
              <div><small>Evaluation protocol</small><strong>v{run.protocol_version} · fixed 20% ZARC</strong></div>
              <div><small>Run result hash</small><code>{run.result_hash}</code></div>
              <div><small>Evaluation result hash</small><code>{report.result_hash}</code></div>
            </div>
            <h3 class="limitations-header">Known limitations</h3>
            <ul class="limitations">{#each report.limitations as item}<li>{item}</li>{/each}</ul>
            <div class="evidence-links">
              <a href="https://github.com/Marcux777/fieldshift-optimizer/blob/main/docs/EVALUATION_PROTOCOL.md" target="_blank" rel="noopener noreferrer">Frozen protocol ↗</a>
              <a href="https://github.com/Marcux777/fieldshift-optimizer/tree/main/reports/20261114T181122Z" target="_blank" rel="noopener noreferrer">Original report ↗</a>
              <a href="https://power.larc.nasa.gov/" target="_blank" rel="noopener noreferrer">NASA POWER ↗</a>
            </div>
          </section>
        {/if}
        <footer><span>Decision-support only. No claims of yield, profit or method superiority.</span><span>FieldShift · NASA Space Apps 2026</span></footer>
      {/if}
    </div>
  </main>
</div>
