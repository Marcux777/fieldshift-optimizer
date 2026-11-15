<script lang="ts">
  // AI-assisted (OpenBot agent)
  import '$lib/insights.css';
  // Piece 3: sowing-date risk map. Rows = year-2 main-crop sowing (d3), columns = year-1 sowing (d1);
  // colour = mean construction climate exposure g1 of that plan (objectives column `mean_g1` of /v1/run).
  import type { Catalog, RunResult } from '$lib/api/client';
  import { cropSequences, sowingGrid, type GridCell, type PolicyKey } from '$lib/insights';
  import { colorScale } from '$lib/presentation';

  let { run, catalog, focus = 'P2', collapsed = false }: { run: RunResult; catalog: Catalog | null; focus?: PolicyKey; collapsed?: boolean } = $props();

  const sequences = $derived(cropSequences(run, catalog));
  const initialKey = $derived(
    sequences.find((s) => s.policies.includes(focus))?.key ?? sequences.find((s) => s.policies.length > 0)?.key ?? sequences[0]?.key ?? ''
  );
  let chosen = $state<string>('');
  const selectedKey = $derived(chosen && sequences.some((s) => s.key === chosen) ? chosen : initialKey);
  const selected = $derived(sequences.find((s) => s.key === selectedKey) ?? null);
  const grid = $derived(selected ? sowingGrid(run, catalog, selected.signature) : null);

  function t(v: number): number {
    return grid && grid.max > grid.min ? (v - grid.min) / (grid.max - grid.min) : 0;
  }
  function textColor(v: number): string {
    return t(v) > 0.45 ? '#ffffff' : '#10201a';
  }
  function short(label: string): string {
    return label.split(': ')[1] ?? label;
  }
  function cellTitle(c: GridCell, d1Label: string, d3Label: string): string {
    const where = `Year-1 sowing ${short(d1Label)}, year-2 sowing ${short(d3Label)}`;
    if (c.value === null) return `${where}: infeasible (no plan under ZARC ≤ 20% and the calendar)`;
    return `${where}: mean exposure ${c.value.toFixed(4)}, worst-scenario exposure ${c.worst?.toFixed(4)}${c.policies.length ? ` · ${c.policies.join(', ')}` : ''} · ID [${c.planId?.join(', ')}]`;
  }
</script>

<details class="insight-block insight-details" data-testid="sowing-risk-map" open={!collapsed}>
  <summary><h3 class="text-base font-bold">Sowing-date risk map</h3></summary>
  <p class="insight-note">
    Same crops, different sowing dates: each cell is one plan, coloured by its mean climate exposure over the 28 construction
    pairs (NASA POWER rainfall and temperature; lower is better). Empty cells are not feasible under ZARC ≤ 20% or the calendar.
  </p>
  <div class="field-group replay-select">
    <label for="risk-map-sequence">Crop sequence (year 1 → second crop · year 2 → second crop)</label>
    <select id="risk-map-sequence" value={selectedKey} onchange={(e) => (chosen = e.currentTarget.value)} data-testid="risk-map-select">
      {#each sequences as s}
        <option value={s.key}>{s.label} ({s.nPlans} plans){s.policies.length ? ` — ${s.policies.join(', ')}` : ''}</option>
      {/each}
    </select>
  </div>

  {#if grid && selected}
    <p class="insight-note risk-scroll-hint">Scroll the grid sideways to see every year-1 sowing date.</p>
    <div class="table-scroll risk-map-scroll">
      <table class="risk-map" data-testid="risk-map-table">
        <caption class="sr-only">
          Mean construction climate exposure for {selected.label}: rows are year-2 sowing dates, columns are year-1 sowing dates.
          {grid.feasibleCount} of {grid.d1.length * grid.d3.length} combinations are feasible.
        </caption>
        <thead>
          <tr>
            <th scope="col" class="corner">Year 2 ↓ / Year 1 →</th>
            {#each grid.d1 as d}<th scope="col" class="col-date" title={d.label}>{short(d.label).replace('/', ' ')}</th>{/each}
          </tr>
        </thead>
        <tbody>
          {#each grid.cells as row, r}
            <tr>
              <th scope="row" title={grid.d3[r].label}>{short(grid.d3[r].label)}</th>
              {#each row as c, k}
                {#if c.value === null}
                  <td class="risk-cell infeasible" title={cellTitle(c, grid.d1[k].label, grid.d3[r].label)}>
                    <span class="sr-only">infeasible</span>
                  </td>
                {:else}
                  <td
                    class="risk-cell"
                    class:has-policy={c.policies.length > 0}
                    style:background={colorScale(t(c.value))}
                    style:color={textColor(c.value)}
                    title={cellTitle(c, grid.d1[k].label, grid.d3[r].label)}
                    data-testid={c.policies.length ? `risk-cell-${c.policies.join('-')}` : undefined}
                  >
                    {#if c.policies.length}<strong>{c.policies.join(' ')}</strong>{/if}
                    <span class="sr-only">{c.value.toFixed(4)}</span>
                  </td>
                {/if}
              {/each}
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
    <div class="risk-legend" data-testid="risk-map-legend">
      <span class="legend-title">Mean exposure</span>
      <span>{grid.min.toFixed(3)}</span>
      <span class="risk-gradient" aria-hidden="true" style:background={`linear-gradient(to right, ${[0, 0.25, 0.5, 0.75, 1].map((v) => colorScale(v)).join(', ')})`}></span>
      <span>{grid.max.toFixed(3)}</span>
      <span class="legend-item"><span class="risk-swatch infeasible" aria-hidden="true"></span>Infeasible (ZARC ≤ 20% / calendar)</span>
      <span class="legend-item"><strong>P0–P2</strong>&nbsp;= policy plans in this crop sequence{grid.markers.length ? `: ${[...new Set(grid.markers.map((m) => m.policy))].join(', ')}` : ': none'}</span>
    </div>
    <p class="insight-note">
      Exposure here ignores the weights and the other two objectives (cover and diversity), so the lowest-exposure cell is not
      automatically the plan a policy chooses. Second crops are sown after the main-crop harvest.
    </p>
  {/if}
</details>
