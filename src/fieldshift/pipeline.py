# AI-assisted (OpenBot agent)
"""Offline application boundary; all scientific calculations belong to the core."""
from __future__ import annotations

import math
from dataclasses import dataclass
from pathlib import Path

from . import catalog, data, evaluation, indicators, optimization

ROOT = Path(__file__).resolve().parents[2]


class PipelineError(ValueError):
    """An actionable input or snapshot error; never replaced by synthetic data."""


@dataclass(frozen=True)
class Inputs:
    ad_class: str = "AD4"
    texture: str = "argilosa"
    weights: tuple[float, float, float] = (0.5, 0.25, 0.25)
    p0: catalog.PlanSpec | None = None
    config_dir: Path = ROOT / "config"
    snapshot_dir: Path | None = None


def validate_inputs(inputs: Inputs) -> None:
    if inputs.ad_class not in {f"AD{i}" for i in range(1, 7)}:
        raise PipelineError("Select a soil water class from AD1 to AD6.")
    if inputs.texture not in {"arenosa", "media", "argilosa"}:
        raise PipelineError("Select sandy, medium or clay soil texture.")
    w = inputs.weights
    if (len(w) != 3 or any(not math.isfinite(x) or x < 0 for x in w)
            or not math.isclose(sum(w), 1.0, abs_tol=1e-12)):
        raise PipelineError("Weights must be finite, non-negative and sum to 1.")


def snapshot_manifest(inputs: Inputs) -> dict:
    snap = inputs.snapshot_dir or data.latest_snapshot(ROOT / "data" / "snapshots")
    return data.verify_manifest(snap)


def run_all(inputs: Inputs) -> tuple[optimization.RunResult, evaluation.EvalReport]:
    validate_inputs(inputs)
    try:
        snap = inputs.snapshot_dir or data.latest_snapshot(ROOT / "data" / "snapshots")
        climate = data.load_climate(snap)
        manifest = data.read_manifest(snap)
        cat = catalog.load_catalog(inputs.config_dir)
        soil = catalog.load_soil(inputs.config_dir, inputs.ad_class, inputs.texture)
        params = indicators.load_indicator_params(inputs.config_dir, cat)
        rr = optimization.run(
            climate, cat, soil, params, snapshot_id=manifest["snapshot_id"],
            p0=inputs.p0, weights=inputs.weights,
        )
        plans, _ = catalog.enumerate_plans(cat, soil)
        report = evaluation.evaluate(
            rr, climate, evaluation.EvalProtocol(), plans=plans,
            loss_model=evaluation.IndicatorLossModel(params), weights=inputs.weights,
            check_plan=lambda plan: catalog.verify_plan(cat, soil, plan),
        )
        return rr, report
    except (ValueError, RuntimeError, OSError, KeyError) as exc:
        raise PipelineError(f"Cannot complete this run: {exc}") from exc
