<script lang="ts">
  // AI-assisted (OpenBot agent)
  import type { Catalog } from '$lib/api/client';
  import { colorScale, linearTicks, paddedDomain, planSequenceLabel } from '$lib/presentation';

  type Point = {
    readonly planId: readonly number[];
    readonly meanExposure: number;
    readonly worstExposure: number;
    readonly coverageLoss: number;
    readonly diversityLoss: number;
  };
  type PolicyPoint = Point & { readonly policy: string; readonly onFront: boolean };

  let {
    front,
    policies,
    catalog
  }: { front: readonly Point[]; policies: readonly PolicyPoint[]; catalog: Catalog | null } = $props();

  const W = 700;
  const H = 440;
  const ML = 66;
  const MR = 22;
  const MT = 18;
  const MB = 56;
  const PW = W - ML - MR;
  const PH = H - MT - MB;

  const plotted = $derived<Point[]>([...front, ...policies]);
  const xDom = $derived(paddedDomain(plotted.map((p) => p.meanExposure)));
  const yDom = $derived(paddedDomain(plotted.map((p) => p.coverageLoss)));
  const cDom = $derived.by<[number, number]>(() => {
    const v = plotted.map((p) => p.diversityLoss);
    return v.length ? [Math.min(...v), Math.max(...v)] : [0, 1];
  });

  function sx(v: number): number {
    return ML + ((v - xDom[0]) / (xDom[1] - xDom[0])) * PW;
  }
  function sy(v: number): number {
    return MT + PH - ((v - yDom[0]) / (yDom[1] - yDom[0])) * PH;
  }
  function color(v: number): string {
    return colorScale(cDom[1] > cDom[0] ? (v - cDom[0]) / (cDom[1] - cDom[0]) : 0);
  }
  function digits(dom: [number, number]): number {
    return dom[1] - dom[0] < 0.06 ? 3 : 2;
  }

  const xTicks = $derived(linearTicks(xDom[0], xDom[1], 5));
  const yTicks = $derived(linearTicks(yDom[0], yDom[1], 5));

  const legendValues = $derived.by(() => {
    const distinct = [...new Set(plotted.map((p) => Math.round(p.diversityLoss * 1000) / 1000))].sort((a, b) => a - b);
    return distinct.length <= 6 ? distinct : [cDom[0], (cDom[0] + cDom[1]) / 2, cDom[1]];
  });

  // Simple label placement: push a label down while it overlaps one already placed.
  const labels = $derived.by(() => {
    const placed: Array<{ policy: string; x: number; y: number; px: number; py: number }> = [];
    for (const p of policies) {
      const px = sx(p.meanExposure);
      const py = sy(p.coverageLoss);
      let x = px + 11;
      const anchorRight = x + 34 > ML + PW;
      if (anchorRight) x = px - 11;
      let y = py - 9;
      for (let guard = 0; guard < 12; guard++) {
        const clash = placed.some((l) => Math.abs(l.x - x) < 40 && Math.abs(l.y - y) < 14);
        if (!clash) break;
        y += 15;
      }
      y = Math.min(MT + PH - 4, Math.max(MT + 12, y));
      placed.push({ policy: p.policy, x, y, px, py });
    }
    return placed.map((l) => ({ ...l, anchor: l.x < l.px ? 'end' : 'start' }));
  });

  const summary = $derived(
    `${front.length} Pareto-front plans. Mean exposure from ${xDom[0].toFixed(3)} to ${xDom[1].toFixed(3)}, coverage loss from ${yDom[0].toFixed(2)} to ${yDom[1].toFixed(2)}. ` +
      policies.map((p) => `${p.policy}: mean exposure ${p.meanExposure.toFixed(3)}, coverage loss ${p.coverageLoss.toFixed(3)}${p.onFront ? '' : ' (not on the front)'}`).join('; ') +
      '.'
  );
</script>

<figure class="pareto-figure">
  <div class="plot-shell">
    <svg viewBox={`0 0 ${W} ${H}`} role="img" aria-labelledby="pareto-title pareto-desc" data-testid="pareto-scatter-svg">
      <title id="pareto-title">Pareto front: mean exposure versus coverage loss, colored by diversity loss</title>
      <desc id="pareto-desc">{summary}</desc>
      <rect x={ML} y={MT} width={PW} height={PH} fill="#ffffff" stroke="#dfe8e0" />
      {#each xTicks as t}
        <line x1={sx(t)} y1={MT} x2={sx(t)} y2={MT + PH} stroke="#e2eae4" stroke-dasharray="3 5" />
        <text x={sx(t)} y={MT + PH + 18} text-anchor="middle">{t.toFixed(digits(xDom))}</text>
      {/each}
      {#each yTicks as t}
        <line x1={ML} y1={sy(t)} x2={ML + PW} y2={sy(t)} stroke="#e2eae4" stroke-dasharray="3 5" />
        <text x={ML - 8} y={sy(t) + 4} text-anchor="end">{t.toFixed(digits(yDom))}</text>
      {/each}

      {#each front as row}
        <circle
          cx={sx(row.meanExposure)}
          cy={sy(row.coverageLoss)}
          r="6.5"
          fill={color(row.diversityLoss)}
          stroke="#ffffff"
          stroke-width="1.5"
        >
          <title>
            {planSequenceLabel(row.planId, catalog)} · ID [{row.planId.join(', ')}] · Mean exposure {row.meanExposure.toFixed(4)} · Worst exposure {row.worstExposure.toFixed(4)} · Coverage loss {row.coverageLoss.toFixed(4)} · Diversity loss {row.diversityLoss.toFixed(4)}
          </title>
        </circle>
      {/each}

      {#each policies as p}
        {@const cx = sx(p.meanExposure)}
        {@const cy = sy(p.coverageLoss)}
        <path
          d={`M ${cx} ${cy - 10} L ${cx + 10} ${cy} L ${cx} ${cy + 10} L ${cx - 10} ${cy} Z`}
          fill={color(p.diversityLoss)}
          stroke="#111d17"
          stroke-width="2.2"
          data-testid={`pareto-marker-${p.policy}`}
        >
          <title>
            {p.policy}{p.onFront ? '' : ' (not on the Pareto front)'}: {planSequenceLabel(p.planId, catalog)} · ID [{p.planId.join(', ')}] · Mean exposure {p.meanExposure.toFixed(4)} · Worst exposure {p.worstExposure.toFixed(4)} · Coverage loss {p.coverageLoss.toFixed(4)} · Diversity loss {p.diversityLoss.toFixed(4)}
          </title>
        </path>
      {/each}
      {#each labels as l}
        {#if Math.abs(l.y - (l.py - 9)) > 1}
          <line x1={l.px} y1={l.py} x2={l.x} y2={l.y - 4} stroke="#111d17" stroke-width="0.8" />
        {/if}
        <text x={l.x} y={l.y} text-anchor={l.anchor} class="policy-label" data-testid={`pareto-label-${l.policy}`}>{l.policy}</text>
      {/each}

      <text x={ML + PW / 2} y={H - 10} text-anchor="middle" class="axis-title">Mean exposure (construction, lower is better)</text>
      <text x={-(MT + PH / 2)} y="16" transform="rotate(-90)" text-anchor="middle" class="axis-title">Coverage loss (lower is better)</text>
    </svg>
  </div>
  <figcaption class="pareto-legend" data-testid="pareto-legend">
    <div class="legend-group">
      <span class="legend-title">Diversity loss (color)</span>
      {#each legendValues as v}
        <span class="legend-item"><span class="legend-swatch" style:background={color(v)}></span>{v.toFixed(2)}</span>
      {/each}
    </div>
    <div class="legend-group">
      <span class="legend-item"><span class="legend-dot"></span>Pareto-front plan</span>
      <span class="legend-item"><span class="legend-diamond"></span>Policy plan (labelled P0–P2)</span>
      {#each policies.filter((p) => !p.onFront) as p}
        <span class="legend-item">{p.policy} is not on the Pareto front</span>
      {/each}
    </div>
    <p class="legend-note">Axes are zoomed to the data range (not 0–1). Worst exposure, the fourth objective, is in each point's tooltip and in the table below.</p>
  </figcaption>
</figure>
