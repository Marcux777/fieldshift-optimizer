// AI-assisted (OpenBot agent)
export const flows = {
  main: 'renders default frozen evaluation computed live by Rust API across all 6 tabs without horizontal overflow',
  'invalid-weights': 'invalid weights are flagged inline, block submission and keep previous results visible as stale',
  'exploratory-soil': 'renders exploratory P0 diagnostics on AD1 soil class while completing P1/P1b/P2 evaluation'
};
export function selection(text = 'main') {
  const names = text.split(',');
  if (!names.length || names.some(n => !Object.hasOwn(flows, n))) throw new Error(`Unknown/empty flow: ${text}`);
  return [...new Set(names)];
}
export function grep(names) {
  return names.map(n => flows[n].replace(/[.*+?^${}()|[\]\\]/g, '\\$&')).join('|');
}
