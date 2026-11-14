# FieldShift SvelteKit frontend (EXPERIMENTAL)

A responsive, evidence-first replacement for the Streamlit interface. The Rust backend supplies **only frozen 2026 evaluation results** at this stage. It is intentionally not a live crop rotation optimizer.

## Development

Start the Rust service following `backend/README.md`. Then:

```bash
cd frontend
npm install
npm run check
npm run dev
```

Use `.env` (copy from `.env.example`) to configure `VITE_API_BASE_URL`. Never hard-code a private token. If hosted separately, configure `FRONTEND_ORIGIN` on Rust to the exact browser origin. Deploy this SvelteKit app as static output (`build/`) and the Rust backend separately on a server/container platform; Streamlit Community Cloud cannot host this architecture.

Charts are original native SVG; data are read from frozen report files, not recalculated. Strategy sensitivity shows the *existing* 15-point grid, not hypothetical live recomputation. Validate mobile and browser behavior before replacing the published submission app.
