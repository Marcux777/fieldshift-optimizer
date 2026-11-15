<script lang="ts">
  // AI-assisted (OpenBot agent)
  import '$lib/insights.css';
  // Piece 5: replay one held-out pair of agricultural years (per_scenario_regret of /v1/evaluate).
  import type { EvalReport } from '$lib/api/client';
  import { replayPair } from '$lib/insights';

  let { report, collapsed = false }: { report: EvalReport; collapsed?: boolean } = $props();

  let index = $state<number>(0);
  const replay = $derived(replayPair(report, Number(index)));
</script>

<details class="insight-block insight-details" data-testid="season-replay" open={!collapsed}>
  <summary><h3 class="text-base font-bold">Replay one test pair</h3></summary>
  <p class="insight-note">
    If you had followed each plan in the chosen pair of agricultural years, this is the regret it would have had (0 = the best
    feasible plan for that pair). This is a retrospective check on held-out years, not a forecast.
  </p>
  <div class="field-group replay-select">
    <label for="replay-pair">Held-out test pair</label>
    <select id="replay-pair" bind:value={index} data-testid="replay-select">
      {#each report.test as label, i}
        <option value={i}>{label}{report.dry_test_scenarios.includes(label) ? ' (dry pair)' : ''}</option>
      {/each}
    </select>
  </div>
  <div aria-live="polite" data-testid="replay-result">
    {#if replay}
      <div class="table-scroll">
        <table class="data-table">
          <caption class="sr-only">Regret of each policy in the test pair {replay.label}</caption>
          <thead>
            <tr>
              <th scope="col">Plan</th>
              <th scope="col">Regret in {replay.label}</th>
              <th scope="col">In this pair</th>
            </tr>
          </thead>
          <tbody>
            {#each replay.rows as r}
              <tr data-testid={`replay-row-${r.policy}`} class:replay-low={r.lowest} class:replay-high={r.highest}>
                <th scope="row">{r.policy}</th>
                <td data-testid={`replay-value-${r.policy}`}>{r.regret.toFixed(4)}</td>
                <td>{r.lowest ? 'Lowest regret' : r.highest ? 'Highest regret' : '—'}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
      {#if replay.rows.length < 4}
        <p class="insight-note">P0 is not listed: it is infeasible under ZARC ≤ 20% for these inputs and has no test regret.</p>
      {/if}
      <p class="insight-note">
        One pair is one draw of weather. The verdict uses all {report.test.length} pairs; see the classification rule.
      </p>
    {/if}
  </div>
</details>
