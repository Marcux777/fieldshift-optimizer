<script lang="ts">
  // AI-assisted (OpenBot agent)
  import '$lib/insights.css';
  // Piece 4: Evidence, Gaps and Risks for each policy, from /v1/run, /v1/evaluate and docs/LIMITATIONS.md.
  import { POLICY_LABELS, type Catalog, type EvalReport, type RunResult } from '$lib/api/client';
  import { GLOBAL_GAPS, POLICIES, evidenceGapsRisks } from '$lib/insights';

  let { run, report, catalog, collapsed = false }: { run: RunResult; report: EvalReport; catalog: Catalog | null; collapsed?: boolean } = $props();

  const cards = $derived(POLICIES.map((p) => evidenceGapsRisks(run, report, catalog, p)));
  const first = $derived(cards[0]);
  const fmt = (v: number | null) => (v === null ? '—' : v.toFixed(4));
</script>

<details class="insight-block insight-details" data-testid="evidence-gaps-risks" open={!collapsed}>
  <summary><h3 class="text-base font-bold">Evidence, gaps and risks per plan</h3></summary>
  <p class="insight-note" data-testid="evidence-shared">
    Shared evidence: NASA POWER snapshot <code>{first.evidence.snapshotId}</code>, protocol v{first.evidence.protocolVersion};
    {first.evidence.constructionPairs} construction pairs ({first.evidence.constructionRange}) and {first.evidence.testPairs}
    held-out test pairs ({first.evidence.testRange}); {first.evidence.excluded} pairs excluded.
  </p>
  <div class="egr-grid">
    {#each cards as c}
      <article class="egr-card" data-testid={`egr-card-${c.policy}`} aria-labelledby={`egr-${c.policy}`}>
        <h4 id={`egr-${c.policy}`}>{c.policy}: {POLICY_LABELS[c.policy]}</h4>
        <h5>Evidence</h5>
        <p>{c.evidence.selection} Measured on the {c.evidence.testPairs} held-out pairs.</p>
        <h5>Gaps</h5>
        {#if c.gaps.planSpecific.length}
          <ul>
            {#each c.gaps.planSpecific as g}
              <li>{g.text} <span class="egr-source">[N] {g.source}</span></li>
            {/each}
          </ul>
        {:else}
          <p>No crop-specific [N] assumption for this plan.</p>
        {/if}
        <p class="egr-source">Plus the {GLOBAL_GAPS.length} global assumptions below.</p>
        <h5>Risks</h5>
        {#if c.feasible}
          <ul>
            <li>Worst test year (M1): <strong>{fmt(c.risks.worstYear)}</strong></li>
            <li>Mean loss in the {c.risks.dryPairs} dry test pairs (M3): <strong>{fmt(c.risks.dryLoss)}</strong></li>
            <li>
              {#if c.risks.weightChanges}
                Choice changes at <strong data-testid={`egr-weights-${c.policy}`}>{c.risks.weightChanges.changed}/{c.risks.weightChanges.total}</strong>
                fixed weight settings.
              {:else}
                Fixed plan: it does not depend on the weights.
              {/if}
            </li>
          </ul>
        {:else}
          <p>Infeasible under ZARC ≤ 20% for these inputs: no test metrics.</p>
        {/if}
      </article>
    {/each}
  </div>
  <details class="chart-data">
    <summary>Global [N] assumptions (apply to every plan)</summary>
    <ul class="scope-list">
      {#each GLOBAL_GAPS as g}
        <li>{g.text} <span class="egr-source">{g.source}</span></li>
      {/each}
    </ul>
  </details>
</details>
