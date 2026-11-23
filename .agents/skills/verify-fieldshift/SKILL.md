---
name: verify-fieldshift
description: Verify FieldShift Rust/Svelte changes locally with isolated real UI flows, bounded execution and preserved evidence; use before handing off a FieldShift change.
---

Run [the entrypoint](../../../scripts/verify-fieldshift) from this repository with a fresh mission ID:

```bash
scripts/verify-fieldshift --mission review-example --flows main,invalid-weights,exploratory-soil
```

Read [the feature map](../../../feature-map.yaml) for the supported flows. Unknown or empty flows are usage errors. Default is main on desktop and mobile. For an uncertain change impact, run all three flows. Dependencies may download; scientific inputs and app computation remain offline.

The [helper](scripts/verify.mjs) builds an isolated source copy, starts its own Rust API serving the static Svelte build, and runs Playwright against it. Evidence is preserved under `.vsdd/evidence/<mission>/`: source/build identity, logs, process IDs, JSON result, screenshots and traces. Do not reuse a mission ID or substitute an existing app server.

Interpret exits: PASS=0, FAIL=1, usage=2, BLOCKED=3. Read result.json, including cleanup, before reporting. BLOCKED means a real external prerequisite is absent; candidate compilation/start/assertion failures are FAIL. Never call a partial run PASS.

For QA calibration only, `--fixture A` mutates the banner in the isolated copy; B keeps it intact; C omits its snapshot. All use the same flow and assertions. These are engineering fixtures, not scientific results. See [operating limits](../../../docs/REGRESSION_SHIELD.md).
