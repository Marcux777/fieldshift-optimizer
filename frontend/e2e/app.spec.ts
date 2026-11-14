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

  test('clears previous results when invalid weights are submitted', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByTestId('outcome-heading')).toContainText('Outcome: NEGATIVE');

    await page.locator('#weight-exposure').fill('0.9');
    await page.getByTestId('compare-rotations-btn').click();

    await expect(page.getByTestId('form-error')).toContainText('sum to 1');
    await expect(page.getByTestId('outcome-heading')).toHaveCount(0);
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
