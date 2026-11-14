# AI-assisted (OpenBot agent)
import json

from streamlit.testing.v1 import AppTest

from fieldshift import catalog, optimization, pipeline


def app():
    return AppTest.from_file(str(pipeline.ROOT / "app.py"), default_timeout=60).run()


def test_invalid_input_and_clear_old_result(monkeypatch):
    at = app()
    at.number_input[0].set_value(0.9)
    at.button[0].click().run()
    assert not at.exception
    assert "sum to 1" in at.error[0].value
    assert "result" not in at.session_state


def test_infeasible_set_is_visible(monkeypatch):
    monkeypatch.setattr(catalog, "enumerate_plans", lambda *a, **k: ([], []))
    monkeypatch.setattr(optimization, "enumerate_plans", lambda *a, **k: ([], []))
    at = app()
    at.button[0].click().run()
    assert not at.exception
    assert any("INFEASIBLE" in h.value for h in at.header)
    assert "no rotation" in at.error[0].value


def test_complete_real_comparison_then_invalid_clears_result():
    at = app()
    at.button[0].click().run()
    assert not at.exception
    assert any(h.value == "Outcome: NEGATIVE" for h in at.header)
    assert len(at.get("plotly_chart")) == 1
    chart = json.loads(at.get("plotly_chart")[0].proto.spec)
    assert all(trace["type"] == "scatter" for trace in chart["data"])
    assert chart["layout"]["xaxis"]["title"]["text"] == "Mean exposure"
    assert "29: 11/Oct" in at.selectbox[3].options
    assert len(at.session_state['result'][1][0].sensitivity) == 15
    assert any("does not replace" in msg.value for msg in at.info)
    assert any("NASA POWER API version" in msg.value for msg in at.markdown)
    assert {tab.label for tab in at.tabs} >= {
        "Policy comparison", "Pareto and alternatives", "Retrospective evaluation",
        "Weight sensitivity"}
    at.number_input[0].set_value(0.9)
    at.button[0].click().run()
    assert not at.exception
    assert "sum to 1" in at.error[0].value
    assert not any(h.value.startswith("Outcome:") for h in at.header)


def test_ad1_p0_diagnosis_does_not_invalidate_feasible_alternatives():
    at = app()
    at.selectbox[0].set_value("AD1")
    at.button[0].click().run()
    assert not at.exception
    assert at.session_state['result'][1][0].n_plans == 169
    assert any("INVIAVEL_SOB_ZARC" in x.value for x in at.warning)
    assert any("30% exceeds 20%" in x.value for x in at.markdown)
    assert any("EXPLORATORY RUN" in x.value for x in at.warning)
