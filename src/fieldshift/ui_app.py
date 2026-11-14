# AI-assisted (OpenBot agent)
"""English Streamlit interface; domain rules are delegated to pipeline/core."""
import json
from dataclasses import asdict

import pandas as pd
import plotly.express as px
import streamlit as st

from . import catalog, pipeline

CROPS = {
    "soja": "Soybean", "milho_1a": "First-season maize",
    "milho_2a": "Second-season maize", "sorgo_2a": "Second-season sorghum",
    "milho_2a_braquiaria": "Second-season maize + brachiaria", "pousio": "Fallow",
}
OUTCOMES = {
    "NEGATIVO": "NEGATIVE", "INCONCLUSIVO": "INCONCLUSIVE", "SUSTENTADO": "SUPPORTED",
    "INVIAVEL": "INFEASIBLE", "DADOS_INSUFICIENTES": "INSUFFICIENT DATA",
}
POLICIES = {"P0": "Current plan", "P1": "Mean climate", "P1b": "Expected loss",
            "P2": "Minimax regret"}
DISCLAIMER = (
    "Decision support only: this tool supports but does not replace the farmer's judgment. "
    "Indicators describe climate exposure and calendar proxies, not yield, profit, water savings "
    "or soil recovery."
)


def crop_name(key):
    return CROPS.get(key, key.replace("_", " ").title())


def inputs_form(cat):
    with st.form("inputs"):
        st.subheader("Farm and priorities")
        cols = st.columns(2)
        ad = cols[0].selectbox("Soil water class", [f"AD{i}" for i in range(1, 7)], index=3)
        texture = cols[1].selectbox("Soil texture", ["argilosa", "media", "arenosa"],
                                   format_func=lambda x: {"argilosa": "Clay", "media": "Medium",
                                                          "arenosa": "Sandy"}[x])
        st.caption("AD4 + clay is a demo assumption, not a measured soil classification.")
        wcols = st.columns(3)
        weights = tuple(col.number_input(label, min_value=0.0, max_value=1.0, value=val,
                                         step=0.05) for col, label, val in zip(
            wcols, ["Climate exposure weight", "Coverage weight", "Diversity weight"],
            [0.5, 0.25, 0.25], strict=True))
        st.caption("Weights must sum to 1. Lower losses are better for all three objectives.")
        st.subheader("Editable current plan (P0)")
        default = catalog.default_p0_spec(cat)
        selected = {}
        for year, first, second, dec_key in [(1, "j1", "j2", "dec1"),
                                             (2, "j3", "j4", "dec3")]:
            cols = st.columns(3)
            options = cat.windows[first.upper()].crops
            selected[first] = cols[0].selectbox(
                f"Year {year}: main crop", options,
                index=options.index(getattr(default, first)), format_func=crop_name)
            selected[dec_key] = cols[1].selectbox(
                f"Year {year}: sowing ten-day period",
                cat.windows[first.upper()].decendios,
                index=cat.windows[first.upper()].decendios.index(getattr(default, dec_key)),
                format_func=lambda d: f"{d}: {catalog.t_label(catalog.dec_start_t(d))}")
            options = (*cat.windows[second.upper()].crops, "pousio")
            selected[second] = cols[2].selectbox(
                f"Year {year}: second crop", options,
                index=options.index(getattr(default, second)), format_func=crop_name)
        st.caption("Second-crop sowing follows harvest. ZARC risk is fixed at 20%; "
                   "an infeasible P0 is excluded from regret comparisons, never repaired silently.")
        st.warning("Do not tune inputs to improve historical test results. "
                   "Changes are exploratory and do not replace the frozen evaluation.")
        submitted = st.form_submit_button("Compare rotations", type="primary")
    return pipeline.Inputs(ad_class=ad, texture=texture, weights=weights,
                           p0=catalog.PlanSpec(**selected)), submitted


def render_report(rr, report):
    outcome = OUTCOMES.get(report.outcome, report.outcome)
    st.header(f"Outcome: {outcome}")
    st.caption("Primary retrospective comparison: P2 vs P1b. All outcomes use the same format.")
    if rr.status != "OK":
        if rr.status == "INVIAVEL":
            st.error("INFEASIBLE: no rotation satisfies the fixed ZARC and calendar constraints.")
        else:
            st.error("INSUFFICIENT DATA: too few complete historical pairs; no recommendation.")
        st.write(rr.scenarios)
        return
    tabs = st.tabs(["Policy comparison", "Pareto and alternatives", "Retrospective evaluation",
                    "Weight sensitivity"])
    with tabs[0]:
        st.caption(f"{rr.n_plans} feasible plans. Plans are selected using construction data only.")
        rows = []
        for key, label in POLICIES.items():
            pm = report.policies.get(key)
            rows.append({"Policy": f"{key}: {label}", "Feasible": bool(pm and pm.feasible),
                         "Max test regret": pm.M1_max_regret if pm else None,
                         "Mean test regret": pm.M2_mean_regret if pm else None,
                         "Mean test loss": pm.mean_loss_test if pm else None})
        st.dataframe(pd.DataFrame(rows), hide_index=True)
        for key, label in POLICIES.items():
            entry = rr.policies[key]
            with st.expander(f"{key}: {label}", expanded=key == "P2"):
                if entry["status"] != "OK":
                    st.warning("Infeasible under ZARC 20% / calendar rules; excluded from regret.")
                    continue
                plan = entry["plan"]
                st.dataframe(pd.DataFrame([
                    {"Window": o["window"], "Crop": crop_name(o["crop"]),
                     "Sowing (day/month)": o["sow"], "Harvest day index": o["harvest_t"]}
                    for o in plan["occupations"]]), hide_index=True)
                m = entry["metrics_construction"]
                st.write(f"Construction mean loss: {m['mean_L']:.4f}; "
                         f"worst regret: {m['max_regret']:.4f}; "
                         f"loss under mean climate: {m['L_sbar']:.4f}.")
                st.write({"P0": "A fixed user-specified baseline, not an optimized recommendation.",
                          "P1": "Minimizes loss under the construction mean climate.",
                          "P1b": "Minimizes average loss across actual construction scenarios.",
                          "P2": "Minimizes the largest construction regret; this does not ensure "
                                "better performance on the held-out years."}[key])
                changed = [row["weights"] for row in rr.sensitivity
                           if row[key] != plan["id"]] if key != "P0" else []
                st.write(f"Selection changes at {len(changed)} of 15 fixed weight settings. "
                         "Soil and calendar constraints can also change feasibility.")
                if changed:
                    st.write("Weight settings selecting a different plan:", changed)
    with tabs[1]:
        st.caption("Construction-only Pareto front: mean and worst climate exposure, "
                   "coverage loss and diversity loss. Lower is better; color shows worst exposure.")
        frame = pd.DataFrame(rr.objectives, columns=["Plan", "Mean exposure", "Worst exposure",
                                                     "Coverage loss", "Diversity loss"])
        front_ids = {tuple(x) for x in rr.pareto["front_ids"]}
        front = frame[frame["Plan"].map(lambda x: tuple(x) in front_ids)].copy()
        front["Plan"] = front["Plan"].astype(str)
        fig = px.scatter_3d(front, x="Mean exposure", y="Coverage loss", z="Diversity loss",
                            color="Worst exposure", hover_name="Plan")
        st.plotly_chart(fig, width="stretch")
        st.write(f"{rr.pareto['n_front']} non-dominated objective vectors. "
                 "Equal vectors are represented once; distinct rotations are listed below.")
        by_id = {tuple(row[0]): row[1:] for row in rr.objectives}
        cat = catalog.load_catalog(pipeline.ROOT / "config")
        for alternative in rr.alternatives:
            pid = alternative["plan_id"]
            sequence = [crop_name(cat.crops[pid[i]].key) for i in (0, 2, 3, 5)]
            st.write({"Plan": alternative["plan_id"], "Policies": alternative["labels"],
                      "Crop sequence J1–J4": " → ".join(sequence),
                      "On front": alternative["on_front"],
                      "Mean/worst exposure, coverage/diversity loss":
                          by_id[tuple(alternative["plan_id"])],
                      "Minimum crop-window differences": alternative["min_hamming_to_others"]})
    with tabs[2]:
        st.write("Construction: 1981/82–2009/10. Test: 2010/11–2024/25. "
                 "No scenario pair crosses the split; 29 February is discarded.")
        st.write(f"Valid pairs: {len(report.construction)} construction, {len(report.test)} test. "
                 f"Excluded pairs: {len(report.excluded_scenarios)}.")
        for c in report.comparisons:
            st.subheader(f"{c.a} vs {c.b}: {OUTCOMES.get(c.classification, c.classification)}")
            st.write(f"Maximum test regret: {c.m1_a:.4f} vs {c.m1_b:.4f}. "
                     f"Median regret difference: {c.median_D:+.4f}; "
                     f"95% block-bootstrap CI: [{c.ci95_median_D[0]:+.4f}, "
                     f"{c.ci95_median_D[1]:+.4f}].")
            st.write(f"Wins / ties / losses: {c.wins} / {c.ties} / {c.losses}. "
                     f"Identical plans: {c.identical_plans}.")
            st.write("Exact Wilcoxon on non-overlapping pairs:", c.wilcoxon_nonoverlap)
        st.write(f"Dry test pairs: {len(report.dry_test_scenarios)}; "
                 f"construction dry threshold: {report.dry_threshold_mm:.1f} mm. "
                 f"Price of robustness in mean climate: {report.M5_price_of_robustness:.4f}.")
        st.dataframe(pd.DataFrame([asdict(pm) for pm in report.policies.values()]).drop(
            columns=["per_scenario_regret", "per_scenario_loss"]), hide_index=True)
        st.caption("M0: feasibility; M1: worst regret; M2: typical regret; M3: dry-year results; "
                   "M4: stability; M5: cost under mean climate. Regret is loss minus the best "
                   "feasible plan's loss in the same scenario; it is not financial regret.")
        export = {"snapshot_id": rr.snapshot_id, "config_hash": rr.config_hash,
                  "optimizer_result_hash": rr.result_hash(),
                  "evaluation_result_hash": report.result_hash(),
                  "run_result": rr.to_dict(), "evaluation": report.to_dict()}
        st.download_button("Download evaluation JSON", json.dumps(export, indent=2),
                           "evaluation.json", "application/json")
    with tabs[3]:
        st.write("All 15 fixed weight combinations are reported. Test results must not be used "
                 "to choose weights. Changed inputs are exploratory, not confirmatory results.")
        rows = [{"Exposure": row["weights"][0], "Coverage": row["weights"][1],
                 "Diversity": row["weights"][2], "P1": str(row["P1"]),
                 "P1b": str(row["P1b"]), "P2": str(row["P2"])} for row in rr.sensitivity]
        st.dataframe(pd.DataFrame(rows), hide_index=True)
        st.dataframe(pd.DataFrame([
            {"Weights": row["weights"],
             "P2 vs P1b": OUTCOMES.get(row["P2_vs_P1b"]["classification"]),
             "P2 vs P1": OUTCOMES.get(row["P2_vs_P1"]["classification"]),
             "Max regret P1": row["M1"]["P1"],
             "Max regret P1b": row["M1"]["P1b"],
             "Max regret P2": row["M1"]["P2"]} for row in report.sensitivity]),
            hide_index=True)


def sources(inputs):
    st.subheader("Sources, assumptions, uncertainties and limitations")
    try:
        manifest = pipeline.snapshot_manifest(inputs)
        st.write(manifest["citation"]["reference"])
        st.write(manifest["citation"]["data_reference"])
        st.write("Access time (UTC):", manifest["collected_utc"])
        st.write("Snapshot ID:", manifest["snapshot_id"])
        st.write("NASA POWER API version:", manifest["source"]["api_version"])
        st.write("Daily cell: −18.0°, −50.625°; precipitation in mm/day; temperature in °C.")
        st.link_button("NASA POWER referencing guide", manifest["citation"]["referencing_guide"])
    except (ValueError, OSError, RuntimeError) as exc:
        st.error(f"Snapshot provenance cannot be verified: {exc}")
    st.markdown("[ZARC / MAPA open data](https://dados.agricultura.gov.br/) — Rio Verde, "
                "2026/27 risk table, Group I, fixed 20% risk limit. "
                "Not certification for insurance or credit.")
    st.write("[N] Assumptions: soybean 100-day cycle; emergence in 7 days; zero operational "
             "margin; proportional crop phases; rainfall/heat thresholds; equal exposure-component "
             "weights; brachiaria desiccation 15 days before year 2; default AD4/clay soil.")
    st.write("Uncertainties: one regional climate cell, no measured farm soil, historical climate "
             "only, low statistical power, overlapping scenario pairs. ZARC calibration overlaps "
             "test years; POWER rainfall correction may use a longer climatology. "
             "The 2026/27 ZARC table is applied retrospectively to all years.")
    st.write("No claim of yield, profit, water savings, soil recovery, novelty or future-climate "
             "validity. AI-assisted implementation, tests and documentation (OpenBot agents). "
             "NASA data are not synthetic; synthetic fixtures are confined to tests.")


def main():
    st.set_page_config(page_title="FieldShift Optimizer", page_icon="🌱", layout="wide")
    st.title("FieldShift Optimizer")
    st.info(DISCLAIMER)
    st.caption("Rio Verde, Goiás · Rainfed farming · Two agricultural years · Four crop windows")
    try:
        cat = catalog.load_catalog(pipeline.ROOT / "config")
    except (ValueError, OSError) as exc:
        st.error(f"Cannot load the crop catalog: {exc}")
        return
    inputs, submitted = inputs_form(cat)
    if submitted:
        st.session_state.pop("result", None)
        try:
            with st.spinner("Comparing rotations with the verified offline NASA snapshot…"):
                st.session_state.result = (inputs, pipeline.run_all(inputs))
        except pipeline.PipelineError as exc:
            st.error(str(exc))
    if "result" in st.session_state:
        used, (rr, report) = st.session_state.result
        if used != inputs:
            st.warning("Inputs changed. Submit again to update results.")
        st.caption(f"Snapshot: {rr.snapshot_id} · Configuration: {rr.config_hash}")
        render_report(rr, report)
    else:
        st.write("Choose the farm inputs and compare rotations. No recommendation has been run.")
    with st.tabs(["Sources, assumptions, uncertainties and limitations"])[0]:
        sources(inputs)
