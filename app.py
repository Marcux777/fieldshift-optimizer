# AI-assisted (OpenBot agent)
"""Community Cloud entrypoint: no container-specific paths or runtime services."""
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT / "src"))

from fieldshift.ui_app import main  # noqa: E402

main()
