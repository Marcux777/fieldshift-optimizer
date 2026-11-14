// AI-assisted (OpenBot agent)
import { expect, test } from '@playwright/test';

test.describe('FieldShift Optimizer Rust + Svelte 5 E2E', () => {
  test('renders default frozen evaluation computed live by Rust API across all 6 tabs without horizontal overflow', async ({
    page
  }, testInfo) => {
    await page.goto('/');

    await expect(page.getByTestId('frozen-outcome-banner')).toContainText(
      'Frozen evaluation outcome: NEGATIVE'
    );
    await expect(page.getByTestId('disclaimer-banner')).toContainText('does not replace');
    await expect(page.getByTestId('outcome-heading')).toContainText('Outcome: NEGATIVE');

    await expect(page.getByTestId('kpi-n-plans')).toHaveText('576');
    await expect(page.getByTestId('kpi-n-front')).toHaveText('36');
    await expect(page.locator('#p0-dec1 option[value="29"]')).toHaveText('29: 11/Oct');

    await expect(page.getByTestId('run-result-hash')).toHaveText(
      'b43703326690e6d0ddd2be48641fbdb9ba46908810ebfca0c8c90a379c79fd66'
    );
    await expect(page.getByTestId('eval-result-hash')).toHaveText(
      '504c28e85e54cbd4e9470e8a423499299dcbed643cb9b2ff8aa461873e134c18'
    );

    const hasOverflowOverview = await page.evaluate(
      () => document.documentElement.scrollWidth > window.innerWidth
    );
    expect(hasOverflowOverview).toBe(false);

    await page.screenshot({
      path: testInfo.outputPath(`overview-${testInfo.project.name}.png`),
      fullPage: true
    });

    const nav = page.getByRole('navigation', { name: 'Primary navigation' });

    await nav.getByRole('button', { name: 'Policy comparison' }).click();
    await expect(page.getByTestId('policy-comparison-section')).toBeVisible();
    await expect(page.getByTestId('policy-comparison-section')).toContainText('P0: Current plan');
    await expect(page.getByTestId('policy-comparison-section')).toContainText('P1b: Expected loss');
    await expect(page.getByTestId('policy-comparison-section')).toContainText('P2: Minimax regret');

    await nav.getByRole('button', { name: 'Pareto and alternatives' }).click();
    await expect(page.getByTestId('pareto-section')).toBeVisible();
    await expect(page.getByTestId('pareto-scatter-svg')).toContainText('Mean exposure');
    await expect(page.getByTestId('pareto-scatter-svg')).toContainText('Coverage loss');

    await nav.getByRole('button', { name: 'Retrospective evaluation' }).click();
    await expect(page.getByTestId('retrospective-evaluation-section')).toBeVisible();
    await expect(page.getByTestId('retrospective-evaluation-section')).toContainText('P2 vs P1b');
    await expect(page.getByTestId('retrospective-evaluation-section')).toContainText('NEGATIVE');

    await nav.getByRole('button', { name: 'Weight sensitivity' }).click();
    await expect(page.getByTestId('weight-sensitivity-section')).toBeVisible();
    const sensitivityRows = page.locator('[data-testid="sensitivity-table"] tbody tr');
    await expect(sensitivityRows).toHaveCount(15);

    await nav.getByRole('button', { name: 'Sources and limitations' }).click();
    await expect(page.getByTestId('sources-section')).toBeVisible();
    await expect(page.getByTestId('sources-section')).toContainText(
      'NASA POWER API version: v2.10.3'
    );
    const limitationItems = page.locator('[data-testid="sources-section"] ul.limitations li');
    await expect(limitationItems).toHaveCount(10);
  });

  // Replaces the former "clears previous results when invalid weights are submitted" test:
  // Level 1 item 7 requires keeping the previous results (marked stale) and validating inline.
  test('invalid weights are flagged inline, block submission and keep previous results visible as stale', async ({
    page
  }) => {
    await page.goto('/');
    await expect(page.getByTestId('outcome-heading')).toContainText('Outcome: NEGATIVE');
    await expect(page.getByTestId('weights-status')).toHaveText('Weights add up to 1.');
    const heightBefore = await page.evaluate(() => document.documentElement.scrollHeight);

    await page.locator('#weight-exposure').fill('0.9');

    await expect(page.getByTestId('weights-status')).toContainText('Weights add up to 1.40');
    await expect(page.locator('#weight-exposure')).toHaveAttribute('aria-invalid', 'true');
    await expect(page.getByTestId('compare-rotations-btn')).toBeDisabled();
    await page.locator('#weight-exposure').press('Enter');

    await expect(page.getByTestId('stale-results-warning')).toContainText('previous inputs');
    await expect(page.getByTestId('outcome-heading')).toContainText('Outcome: NEGATIVE');
    await expect(page.getByTestId('kpi-n-plans')).toHaveText('576');
    await expect(page.getByTestId('run-result-hash')).toHaveText(
      'b43703326690e6d0ddd2be48641fbdb9ba46908810ebfca0c8c90a379c79fd66'
    );
    await expect(page.getByTestId('sensitivity-table').locator('tbody tr')).toHaveCount(15);
    const heightAfter = await page.evaluate(() => document.documentElement.scrollHeight);
    expect(heightAfter).toBeGreaterThanOrEqual(heightBefore * 0.95);

    await page.locator('#weight-exposure').fill('0.5');
    await expect(page.getByTestId('weights-status')).toHaveText('Weights add up to 1.');
    await expect(page.getByTestId('compare-rotations-btn')).toBeEnabled();
    await expect(page.getByTestId('stale-results-warning')).toHaveCount(0);
  });

  test('explains the tool, grouping, classification rule, sensitivity classes and Pareto legend without overflow or console errors', async ({
    page
  }, testInfo) => {
    const consoleErrors: string[] = [];
    page.on('console', (msg) => {
      if (msg.type() === 'error') consoleErrors.push(msg.text());
    });
    page.on('pageerror', (err) => consoleErrors.push(err.message));

    await page.goto('/');
    await expect(page.getByTestId('outcome-heading')).toContainText('Outcome: NEGATIVE');

    // Item 5: intro and glossary
    await expect(page.getByTestId('tool-intro')).toContainText('Rio Verde, Goiás');
    await expect(page.getByTestId('tool-intro')).toContainText('decision-support tool');
    const glossary = page.getByTestId('glossary');
    for (const term of [
      'Minimax regret',
      'Maximum regret (M1)',
      'M0–M5',
      'LOO Hamming (M4c)',
      'ZARC',
      'Pareto front',
      'P0, P1, P1b, P2',
      'Held-out test'
    ]) {
      await expect(glossary.locator('dt', { hasText: term }).first()).toBeAttached();
    }
    await page.locator('a.term', { hasText: 'M4c LOO Hamming' }).click();
    await expect(glossary).toHaveAttribute('open', '');
    await expect(page.locator('#glossary-m4c')).toBeVisible();

    // Item 4: zoomed Pareto chart with legend and labelled policy markers
    const svg = page.getByTestId('pareto-scatter-svg');
    for (const p of ['P0', 'P1', 'P1b', 'P2']) {
      await expect(svg.getByTestId(`pareto-label-${p}`)).toHaveText(p);
      await expect(svg.getByTestId(`pareto-marker-${p}`)).toBeAttached();
    }
    await expect(svg.locator('circle')).toHaveCount(36);
    await expect(svg).not.toContainText('1.00');
    await expect(page.getByTestId('pareto-legend')).toContainText('Diversity loss (color)');
    await expect(page.getByTestId('pareto-legend')).toContainText('P0 is not on the Pareto front');

    // Item 1: grouping note and one row per policy plan
    await expect(page.getByTestId('pareto-grouping-note')).toContainText('ignores sowing dates');
    await expect(page.getByTestId('pareto-grouping-note')).toContainText('P1/P1b/P2');
    const altTable = page.getByTestId('alternatives-table');
    await expect(altTable.locator('tbody tr')).toHaveCount(8);
    await expect(altTable).toContainText('ID [0, 5, 3, 1, 7, 4]');
    await expect(altTable).toContainText('ID [0, 4, 3, 1, 7, 4]');

    // Item 6: crop sequence and sowing date as the main label, ID secondary
    const policySection = page.getByTestId('policy-comparison-section');
    await expect(policySection.getByTestId('plan-timeline').first()).toContainText('Soybean');
    await expect(policySection.getByTestId('plan-timeline').first()).toContainText('sown 11/Oct');

    // Item 2: exact classification rule and per-comparison reason
    await expect(page.getByTestId('classification-rule')).toContainText('docs/EVALUATION_PROTOCOL.md');
    await expect(page.getByTestId('classification-rule')).toContainText('entirely above 0');
    await expect(page.getByTestId('comparisons-table')).toContainText(
      'Negative because the 95% CI of D is entirely above 0'
    );
    await expect(page.getByTestId('primary-classification-reason')).toContainText(
      "P2's maximum test regret (M1) is greater than or equal"
    );

    // Item 3: held-out classification columns exist and are filled
    const sens = page.getByTestId('sensitivity-table');
    await expect(sens.locator('thead')).toContainText('P2 vs P1b (held-out)');
    await expect(sens.getByTestId('sensitivity-class-p1b')).toHaveCount(15);
    await expect(sens.locator('tr.default-row [data-testid="sensitivity-class-p1b"]')).toHaveText('NEGATIVE');
    await expect(sens.getByTestId('sensitivity-class-p1b').filter({ hasText: '—' })).toHaveCount(0);

    // No horizontal overflow against the configured viewport (also catches mobile zoom-out)
    const viewportWidth = page.viewportSize()?.width ?? 0;
    const widths = await page.evaluate(() => ({
      scroll: document.documentElement.scrollWidth,
      inner: window.innerWidth
    }));
    expect(widths.inner).toBe(viewportWidth);
    expect(widths.scroll).toBeLessThanOrEqual(viewportWidth);

    await page.screenshot({
      path: testInfo.outputPath(`level1-${testInfo.project.name}.png`),
      fullPage: true
    });
    expect(consoleErrors).toEqual([]);
  });

  test('renders exploratory P0 diagnostics on AD1 soil class while completing P1/P1b/P2 evaluation', async ({
    page
  }) => {
    await page.goto('/');
    await expect(page.getByTestId('kpi-n-plans')).toHaveText('576');

    await page.locator('#soil-ad-select').selectOption('AD1');
    await page.getByTestId('compare-rotations-btn').click();

    await expect(page.getByTestId('exploratory-warning')).toContainText('EXPLORATORY RUN');
    await expect(page.getByTestId('outcome-heading')).toContainText(
      'Exploratory outcome: SUPPORTED'
    );
    await expect(page.getByTestId('kpi-n-plans')).toHaveText('169');
    await expect(page.getByTestId('p0-infeasible-warning-P0')).toContainText('INVIAVEL_SOB_ZARC');
    await expect(page.getByTestId('p0-diagnostic-line').first()).toContainText('30% exceeds 20%');

    const nav = page.getByRole('navigation', { name: 'Primary navigation' });
    await nav.getByRole('button', { name: 'Policy comparison' }).click();
    await expect(page.getByTestId('policy-comparison-section')).toContainText('INVIAVEL_SOB_ZARC');
    await expect(page.getByTestId('policy-comparison-section')).toContainText('P2: Minimax regret');
  });
});
