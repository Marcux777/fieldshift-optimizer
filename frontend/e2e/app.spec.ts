// AI-assisted (OpenBot agent)
import { expect, test, type Page } from '@playwright/test';

/**
 * Horizontal overflow measured against the real layout width (document clientWidth) and the configured
 * viewport. On mobile emulation an overflowing page is zoomed out, which makes window.innerWidth grow
 * with the content; comparing scrollWidth with innerWidth alone therefore never fails there.
 */
async function measureOverflow(page: Page) {
  const viewport = page.viewportSize()?.width ?? 0;
  const m = await page.evaluate(() => ({
    scrollWidth: document.documentElement.scrollWidth,
    clientWidth: document.documentElement.clientWidth,
    innerWidth: window.innerWidth
  }));
  return { ...m, viewport, overflow: m.scrollWidth > m.clientWidth || m.innerWidth > viewport };
}

async function expectNoHorizontalOverflow(page: Page) {
  const m = await measureOverflow(page);
  expect(m, JSON.stringify(m)).toMatchObject({ overflow: false });
}

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

    await expectNoHorizontalOverflow(page);

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
    await expectNoHorizontalOverflow(page);

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
    await expect(page.getByTestId('p0-infeasible-warning-P0')).toContainText('Infeasible under ZARC ≤ 20%');
    await expect(page.getByTestId('policy-card-P0')).toContainText('Infeasible under ZARC ≤ 20%');
    await expect(page.locator('body')).not.toContainText('INVIAVEL_SOB_ZARC');
    await expect(page.getByTestId('p0-diagnostic-line').first()).toContainText('30% exceeds 20%');

    const nav = page.getByRole('navigation', { name: 'Primary navigation' });
    await nav.getByRole('button', { name: 'Policy comparison' }).click();
    await expect(page.getByTestId('policy-comparison-section')).toContainText('Infeasible under ZARC ≤ 20%');
    await expect(page.getByTestId('policy-comparison-section')).toContainText('P2: Minimax regret');
  });

  test('overflow check fails when content is wider than the screen (negative control)', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByTestId('outcome-heading')).toContainText('Outcome: NEGATIVE');
    expect((await measureOverflow(page)).overflow).toBe(false);
    await page.addStyleTag({ content: '.content-wrap::after { content: ""; display: block; width: 2000px; height: 1px; }' });
    await expect.poll(async () => (await measureOverflow(page)).overflow).toBe(true);
  });

  test('all six navigation items are visible and the GitHub link is reachable on every screen', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByTestId('outcome-heading')).toContainText('Outcome: NEGATIVE');
    const viewport = page.viewportSize()?.width ?? 0;
    const buttons = page.getByRole('navigation', { name: 'Primary navigation' }).getByRole('button');
    await expect(buttons).toHaveCount(6);
    for (let i = 0; i < 6; i++) {
      await expect(buttons.nth(i)).toBeVisible();
      const box = await buttons.nth(i).boundingBox();
      expect(box, `tab ${i}`).not.toBeNull();
      expect(box!.x).toBeGreaterThanOrEqual(0);
      expect(box!.x + box!.width).toBeLessThanOrEqual(viewport);
    }
    const github = page.locator('a[href="https://github.com/Marcux777/fieldshift-optimizer"]').filter({ visible: true });
    await expect(github.first()).toBeVisible();
    await expectNoHorizontalOverflow(page);
  });

  test('clicking a navigation item scrolls its section into view and marks it as current', async ({ page }) => {
    await page.emulateMedia({ reducedMotion: 'reduce' });
    await page.goto('/');
    await expect(page.getByTestId('outcome-heading')).toContainText('Outcome: NEGATIVE');
    const nav = page.getByRole('navigation', { name: 'Primary navigation' });
    await expect(nav.getByRole('button', { name: 'Overview' })).toHaveAttribute('aria-current', 'page');

    const cases: Array<[string, string]> = [
      ['Weight sensitivity', 'weight-sensitivity-section'],
      ['Pareto and alternatives', 'pareto-section'],
      ['Sources and limitations', 'sources-section'],
      ['Retrospective evaluation', 'retrospective-evaluation-section'],
      ['Policy comparison', 'policy-comparison-section']
    ];
    for (const [tab, testId] of cases) {
      await page.evaluate(() => window.scrollTo(0, 0));
      await nav.getByRole('button', { name: tab }).click();
      await expect(page.getByTestId(testId)).toBeInViewport();
      await expect(page.getByTestId(testId)).toBeFocused();
      await expect(nav.getByRole('button', { name: tab })).toHaveAttribute('aria-current', 'page');
      await expect(nav.locator('[aria-current="page"]')).toHaveCount(1);
      await expect(page).toHaveTitle(new RegExp(`^${tab} \\| FieldShift Optimizer`));
    }
    await nav.getByRole('button', { name: 'Overview' }).click();
    await expect(page.getByTestId('outcome-heading')).toBeInViewport();
  });

  test('sidebar stays in view while scrolling on desktop', async ({ page }, testInfo) => {
    test.skip(testInfo.project.name !== 'desktop-chromium', 'mobile keeps the stacked layout');
    await page.goto('/');
    await expect(page.getByTestId('outcome-heading')).toContainText('Outcome: NEGATIVE');
    await page.evaluate(() => window.scrollTo(0, 4000));
    await expect.poll(() => page.evaluate(() => window.scrollY)).toBeGreaterThan(3000);
    const sidebar = page.locator('aside.sidebar');
    await expect(sidebar).toHaveCSS('position', 'sticky');
    const box = await sidebar.boundingBox();
    expect(box!.y).toBe(0);
    await expect(page.getByRole('navigation', { name: 'Primary navigation' }).getByRole('button', { name: 'Weight sensitivity' })).toBeInViewport();
  });

  test('after Compare rotations the result heading is scrolled into view, focused and announced', async ({ page }) => {
    await page.emulateMedia({ reducedMotion: 'reduce' });
    await page.goto('/');
    await expect(page.getByTestId('outcome-heading')).toContainText('Outcome: NEGATIVE');
    await page.locator('#weight-exposure').fill('0.75');
    await page.locator('#weight-diversity').fill('0');
    await page.getByTestId('compare-rotations-btn').scrollIntoViewIfNeeded();
    // The form sits below the verdict, so the heading starts out of view above the button.
    const before = await page.getByTestId('outcome-heading').boundingBox();
    expect(before!.y < 0 || before!.y > 150).toBe(true);
    await page.getByTestId('compare-rotations-btn').click();
    await expect(page.getByTestId('outcome-heading')).toContainText('Exploratory outcome');
    await expect(page.getByTestId('outcome-heading')).toBeInViewport();
    await expect.poll(async () => (await page.getByTestId('outcome-heading').boundingBox())!.y).toBeLessThanOrEqual(100);
    await expect(page.getByTestId('outcome-heading')).toBeFocused();
    await expect(page.getByTestId('results-live-region')).toContainText('Results updated');
  });

  test('a busy API answer (429) is retried and the new result replaces the old one', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByTestId('outcome-heading')).toContainText('Outcome: NEGATIVE');
    let evaluateCalls = 0;
    await page.route('**/v1/evaluate', async (route) => {
      evaluateCalls += 1;
      if (evaluateCalls === 1) {
        await route.fulfill({
          status: 429,
          headers: { 'Retry-After': '0', 'Content-Type': 'application/json' },
          body: JSON.stringify({ error: { code: 'BUSY', message: 'busy' } })
        });
      } else {
        await route.continue();
      }
    });
    await page.locator('#soil-ad-select').selectOption('AD1');
    await page.getByTestId('compare-rotations-btn').click();
    await expect(page.getByTestId('outcome-heading')).toContainText('Exploratory outcome: SUPPORTED');
    await expect(page.getByTestId('kpi-n-plans')).toHaveText('169');
    await expect(page.getByTestId('form-error')).toHaveCount(0);
    expect(evaluateCalls).toBe(2);
  });

  test('when the API stays busy the previous result stays visible with an error and a stale warning', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByTestId('outcome-heading')).toContainText('Outcome: NEGATIVE');
    let runCalls = 0;
    await page.route('**/v1/run', async (route) => {
      runCalls += 1;
      await route.fulfill({
        status: 503,
        headers: { 'Retry-After': '0', 'Content-Type': 'application/json' },
        body: JSON.stringify({ error: { code: 'BUSY', message: 'All computation slots stayed busy' } })
      });
    });
    await page.locator('#soil-ad-select').selectOption('AD1');
    await page.getByTestId('compare-rotations-btn').click();
    await expect(page.getByTestId('form-error')).toContainText('busy');
    await expect(page.getByTestId('outcome-heading')).toContainText('Outcome: NEGATIVE');
    await expect(page.getByTestId('kpi-n-plans')).toHaveText('576');
    await expect(page.getByTestId('stale-results-warning')).toContainText('previous inputs');
    expect(runCalls).toBe(4);
  });

  test('block B: English verdict, one top notice, form after the verdict, no raw codes', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByTestId('outcome-heading')).toContainText('Outcome: NEGATIVE');

    const verdict = page.getByTestId('verdict-text');
    await expect(verdict).toHaveText(
      'NEGATIVE: minimax regret (P2) did not reduce maximum test regret versus expected loss (P1b) — 0.1744 vs 0.1466; median D +0.0064, 95% CI [-0.0609, +0.0375].'
    );
    const visibleText = await page.locator('body').innerText();
    for (const raw of ['NEGATIVO', 'perda esperada', 'INVIAVEL', 'OK\n']) {
      expect(visibleText, raw).not.toContain(raw);
    }
    await expect(page.getByTestId('policy-card-P2')).toContainText('Feasible');

    // One short notice at the top; the full text lives in Sources and limitations.
    await expect(page.locator('.banner-stack > *')).toHaveCount(1);
    await expect(page.getByTestId('disclaimer-banner')).toContainText('does not replace');
    await expect(page.getByTestId('sources-section').getByTestId('full-disclaimer')).toContainText('does not replace');
    await expect(page.getByTestId('sources-section').getByTestId('frozen-outcome-banner')).toContainText('Frozen evaluation outcome: NEGATIVE');
    await expect(page.locator('footer')).not.toContainText('No claims');

    // Intro, then verdict, then the form.
    const order = await page.evaluate(() => {
      const y = (sel: string) => document.querySelector(sel)!.getBoundingClientRect().top + window.scrollY;
      return { intro: y('[data-testid="tool-intro"]'), verdict: y('[data-testid="verdict-text"]'), form: y('[data-testid="inputs-form"]') };
    });
    expect(order.intro).toBeLessThan(order.verdict);
    expect(order.verdict).toBeLessThan(order.form);
    await expectNoHorizontalOverflow(page);
  });

  test('block B: weight presets come from the 15-point grid and fill the three fields', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByTestId('outcome-heading')).toContainText('Outcome: NEGATIVE');
    const preset = page.getByTestId('weight-preset');
    await expect(preset.locator('option')).toHaveCount(16);
    await expect(preset).toHaveValue('4');
    await expect(preset.locator('option:checked')).toHaveText('0.50 / 0.25 / 0.25 (default)');
    await preset.selectOption({ label: '0.75 / 0.25 / 0.00' });
    await expect(page.locator('#weight-exposure')).toHaveValue('0.75');
    await expect(page.locator('#weight-coverage')).toHaveValue('0.25');
    await expect(page.locator('#weight-diversity')).toHaveValue('0');
    await expect(page.getByTestId('weights-status')).toHaveText('Weights add up to 1.');
    await page.locator('#weight-exposure').fill('0.4');
    await expect(preset).toHaveValue('custom');
  });

  test('block B: infeasible P0 options are disabled with their ZARC reason before submitting', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByTestId('outcome-heading')).toContainText('Outcome: NEGATIVE');
    const dec1 = page.locator('#p0-dec1');
    await expect(dec1.locator('option[value="27"]')).toBeDisabled();
    await expect(dec1.locator('option[value="27"]')).toContainText('unavailable: ZARC risk 40% > 20%');
    await expect(dec1.locator('option[value="25"]')).toContainText('not indicated by ZARC');
    await expect(dec1.locator('option[value="29"]')).toBeEnabled();
    await expect(page.getByTestId('p0-availability-note')).toContainText('AD4');

    // Switching J1 to first-season maize with sowing period 29 is flagged before submission.
    await page.locator('#p0-j1').selectOption('milho_1a');
    await expect(page.getByTestId('p0-feasibility-note')).toContainText('not feasible under ZARC ≤ 20%');
    await expect(page.getByTestId('p0-feasibility-note')).toContainText('J1: ZARC risk 30% > 20%');
    await expect(dec1.locator('option[value="29"]')).toBeDisabled();
    // Dec 31 exists for maize but not followed by second-season maize; dec 30 fits the rest of the plan.
    await expect(dec1.locator('option[value="31"]')).toContainText('not with the other choices');
    await dec1.selectOption('30');
    await expect(page.getByTestId('p0-feasibility-note')).toHaveCount(0);
  });
});
