<script lang="ts">
  // AI-assisted (OpenBot agent)
  import type { Catalog } from '$lib/api/client';
  import { describePlan, planSequenceLabel } from '$lib/presentation';

  let {
    planId,
    catalog,
    compact = false,
    inline = false,
    showId = true
  }: {
    planId: readonly number[];
    catalog: Catalog | null;
    compact?: boolean;
    inline?: boolean;
    showId?: boolean;
  } = $props();

  const steps = $derived(describePlan(planId, catalog));
  const label = $derived(planSequenceLabel(planId, catalog));
</script>

{#if inline}
  <div class="plan-inline" data-testid="plan-timeline">
    <span>
      {#each steps as s, i}
        {#if i === 2}<span class="plan-inline-sep" aria-hidden="true"> | </span>{:else if i > 0}<span aria-hidden="true"> → </span>{/if}<span class="plan-inline-step"><strong>{s.window}</strong> {s.cropName}{s.sowing ? ` (${s.sowing})` : ''}</span>
      {/each}
    </span>
    {#if showId}<code class="plan-id">ID [{planId.join(', ')}]</code>{/if}
  </div>
{:else}
<div class="plan-timeline" class:compact role="group" aria-label={label} data-testid="plan-timeline">
  <div class="plan-years">
    {#each [1, 2] as year}
      <div class="plan-year">
        <span class="plan-year-label">Year {year}</span>
        <ol class="plan-chips">
          {#each steps.filter((s) => s.year === year) as s}
            <li class="plan-chip" class:second={s.window === 'J2' || s.window === 'J4'} class:fallow={s.cropKey === 'pousio'}>
              <span class="chip-window">{s.window}</span>
              <span class="chip-crop">{s.cropName}</span>
              <span class="chip-date">
                {s.sowing ? `sown ${s.sowing}` : s.cropKey === 'pousio' ? 'no crop' : 'after harvest'}
              </span>
            </li>
          {/each}
        </ol>
      </div>
    {/each}
  </div>
  {#if showId}
    <code class="plan-id" title="Plan ID: crop and sowing-period indices">ID [{planId.join(', ')}]</code>
  {/if}
</div>
{/if}
