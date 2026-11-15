<script lang="ts">
  // AI-assisted (OpenBot agent)
  import '$lib/insights.css';
  // Pieces 1 and 2: P2 read as insurance ("what it traded for what"), never as the winner.
  import { formatOutcomeLabel, type EvalReport } from '$lib/api/client';
  import { PAIRED_RULE, premiumVsProtection, protectionStory } from '$lib/insights';

  let { report }: { report: EvalReport } = $props();

  const story = $derived(protectionStory(report));
  const rows = $derived(premiumVsProtection(report));
  const p1bRow = $derived(rows.find((r) => r.policy === 'P1b') ?? null);

  // Year-by-year chart: regret of P1, P1b and P2 in each held-out pair (per_scenario_regret).
  const SERIES = [
    { key: 'P1', color: '#8a5a00', dash: '6 4' },
    { key: 'P1b', color: '#2c7fb8', dash: '' },
    { key: 'P2', color: '#1a7f4b', dash: '' }
  ] as const;
  const W = 560;
  const H = 230;
  const ML = 46;
  const MR = 12;
  const MT = 12;
  const MB = 34;
  const n = $derived(report.test.length);
  const maxY = $derived(
    Math.max(0.01, ...SERIES.flatMap((s) => report.policies[s.key]?.per_scenario_regret ?? []))
  );
  const x = (i: number) => ML + (n > 1 ? (i / (n - 1)) * (W - ML - MR) : 0);
  const y = (v: number) => MT + (1 - v / maxY) * (H - MT - MB);
  const yTicks = $derived([0, maxY / 2, maxY]);
  const shortLabel = (label: string) => label.split('–')[0];
</script>

{#if story}
  <div class="insight-block" data-testid="protection-story">
    <div class="insight-head">
      <h3 class="text-base font-bold">What P2 traded, and for what</h3>
      <span class="outcome-badge" data-testid="protection-story-outcome">
        Primary outcome: {formatOutcomeLabel(story.primary.classification)}
      </span>
    </div>
    <p class="insight-text" data-testid="story-primary">{story.text.primary}</p>
    <p class="insight-text" data-testid="story-secondary">{story.text.secondary}</p>
    <p class="insight-text" data-testid="story-tradeoff">{story.text.tradeoff}</p>
    <p class="insight-note">Read P2 as insurance bought against P1's worst year, not as the plan to prefer. {PAIRED_RULE.text}</p>

    <div class="insight-two">
      <figure class="year-chart">
        <svg viewBox={`0 0 ${W} ${H}`} role="img" aria-labelledby="year-chart-title" data-testid="year-chart">
          <title id="year-chart-title">Regret of P1, P1b and P2 in each of the {n} held-out test pairs; lower is better</title>
          {#each yTicks as t}
            <line x1={ML} x2={W - MR} y1={y(t)} y2={y(t)} stroke="#e2eae4" stroke-dasharray="3 5" />
            <text x={ML - 6} y={y(t) + 4} text-anchor="end">{t.toFixed(2)}</text>
          {/each}
          {#each report.test as label, i}
            {#if i % 2 === 0}
              <text x={x(i)} y={H - MB + 16} text-anchor="middle">{shortLabel(label)}</text>
            {/if}
          {/each}
          {#each SERIES as s}
            {@const vals = report.policies[s.key]?.per_scenario_regret ?? []}
            {#if vals.length === n}
              <polyline
                points={vals.map((v, i) => `${x(i)},${y(v)}`).join(' ')}
                fill="none"
                stroke={s.color}
                stroke-width="2.5"
                stroke-dasharray={s.dash}
              />
              {#each vals as v, i}
                <circle cx={x(i)} cy={y(v)} r="3.5" fill={s.color}>
                  <title>{s.key} · {report.test[i]} · regret {v.toFixed(4)}</title>
                </circle>
              {/each}
            {/if}
          {/each}
          <text x={(ML + W - MR) / 2} y={H - 4} text-anchor="middle" class="axis-title">Held-out test pair (first year shown)</text>
        </svg>
        <div class="chart-legend">
          {#each SERIES as s}
            <span class="legend-item"><span class="legend-line" style:background={s.color}></span>{s.key}</span>
          {/each}
          <span class="legend-item">Regret per pair (lower is better)</span>
        </div>
        <details class="chart-data">
          <summary>Show the numbers as a table</summary>
          <div class="table-scroll">
            <table class="data-table">
              <caption class="sr-only">Regret per held-out test pair for P1, P1b and P2</caption>
              <thead>
                <tr>
                  <th scope="col">Test pair</th>
                  {#each SERIES as s}<th scope="col">{s.key}</th>{/each}
                </tr>
              </thead>
              <tbody>
                {#each report.test as label, i}
                  <tr>
                    <th scope="row">{label}</th>
                    {#each SERIES as s}
                      <td>{report.policies[s.key]?.per_scenario_regret?.[i]?.toFixed(4) ?? '—'}</td>
                    {/each}
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        </details>
      </figure>

      <div class="premium-card" data-testid="premium-card">
        <h4>Premium paid × protection received</h4>
        <p class="insight-note">Both measured against the mean-climate plan P1.</p>
        <!-- One stacked block per plan: no wide table, so nothing is clipped at any width. -->
        <div class="premium-plans">
          {#each rows as r}
            <section class="premium-plan" data-testid={`premium-row-${r.policy}`} aria-label={`${r.policy}: premium and protection versus P1`}>
              <h5>{r.policy}</h5>
              <dl>
                <div>
                  <dt>Premium paid (extra loss, average climate)</dt>
                  <dd>+{r.premium.toFixed(4)}{r.isM5 ? ' (M5)' : ''}</dd>
                </div>
                <div>
                  <dt>Protection received (worst test year, vs P1)</dt>
                  <dd>{r.p1M1.toFixed(4)} → {r.m1.toFixed(4)} ({r.protection >= 0 ? '−' : '+'}{Math.abs((r.protection / r.p1M1) * 100).toFixed(0)}%)</dd>
                </div>
              </dl>
            </section>
          {/each}
        </div>
        <p class="insight-caveat" data-testid="premium-caveat">
          Caveat: the protocol's primary test is P2 vs P1b, and there P2's worst year ({story.p2.m1.toFixed(4)}) was
          {story.p2BeatsP1bWorstYear ? 'lower' : 'higher'} than P1b's ({story.p1b.m1.toFixed(4)}):
          {formatOutcomeLabel(story.primary.classification)}.
          {#if !story.p2BeatsP1bWorstYear && p1bRow}
            P1b paid a premium of {p1bRow.premium.toFixed(4)} and protected the worst year more.
          {/if}
        </p>
      </div>
    </div>
  </div>
{/if}
