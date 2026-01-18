import shutil
import subprocess
import sys
from pathlib import Path
import pytest

from fontTools.ttLib import TTFont


REPO_ROOT = Path(__file__).resolve().parents[1]
FONTS_DIR = REPO_ROOT / "fonts"
DEJAU = FONTS_DIR / "DejaVuSans.ttf"
NOTO_VAR = FONTS_DIR / "NotoEmoji-VariableFont_wght.ttf"
SCRIPT = REPO_ROOT / "scripts" / "extend_font.py"


pytestmark = pytest.mark.usefixtures("tmp_path")


def ensure_fonts_exist():
    if not DEJAU.exists() or not NOTO_VAR.exists():
        pytest.skip("Required test fonts not present in repository fonts/ folder")


def get_best_cmap(path: Path):
    tt = TTFont(str(path))
    return tt.getBestCmap() or {}


def run_script(args, cwd=REPO_ROOT):
    cmd = [sys.executable, str(SCRIPT)] + args
    res = subprocess.run(cmd, cwd=str(cwd), capture_output=True, text=True)
    if res.returncode != 0:
        raise RuntimeError(f"Command failed: {cmd}\nstdout: {res.stdout}\nstderr: {res.stderr}")
    return res


def test_copy_one_missing_glyph_and_dst_creation(tmp_path):
    ensure_fonts_exist()

    src_copy = tmp_path / "src.ttf"
    dst_dir = tmp_path / "outdir"
    dst = dst_dir / "mono.ttf"

    shutil.copy(str(DEJAU), str(src_copy))

    combine_cmap = get_best_cmap(NOTO_VAR)
    src_cmap = get_best_cmap(src_copy)

    missing = set(combine_cmap.keys()) - set(src_cmap.keys())
    assert missing, "Expected combine font to have codepoints missing from DejaVu"
    cp = sorted(missing)[0]

    run_script(["--src-font", str(src_copy), "--combine-font", str(NOTO_VAR), "--dst-font", str(dst)])

    assert dst.exists(), "Destination font was not created"

    dst_cmap = get_best_cmap(dst)
    assert cp in dst_cmap, f"Expected codepoint U+{cp:04X} to be present in destination font"


def test_inplace_modification_when_dst_not_provided(tmp_path):
    ensure_fonts_exist()

    src_copy = tmp_path / "src_inplace.ttf"
    shutil.copy(str(DEJAU), str(src_copy))

    combine_cmap = get_best_cmap(NOTO_VAR)
    src_cmap = get_best_cmap(src_copy)
    missing = set(combine_cmap.keys()) - set(src_cmap.keys())
    assert missing
    cp = sorted(missing)[0]

    run_script(["--src-font", str(src_copy), "--combine-font", str(NOTO_VAR)])

    new_cmap = get_best_cmap(src_copy)
    assert cp in new_cmap


def test_variable_to_fixed_instantiation(tmp_path):
    ensure_fonts_exist()

    # Confirm combine is variable and source is fixed
    combine_tt = TTFont(str(NOTO_VAR))
    assert "fvar" in combine_tt, "Test combine font is expected to be variable"

    src_copy = tmp_path / "src_fixed.ttf"
    dst = tmp_path / "dst.ttf"
    shutil.copy(str(DEJAU), str(src_copy))

    # Run script to incorporate glyphs
    run_script(["--src-font", str(src_copy), "--combine-font", str(NOTO_VAR), "--dst-font", str(dst)])

    dst_tt = TTFont(str(dst))
    # Destination should be a fixed font (no fvar introduced)
    assert "fvar" not in dst_tt, "Destination font should remain a fixed font (no fvar table)"

    # Ensure at least one codepoint was copied
    combine_cmap = get_best_cmap(NOTO_VAR)
    src_cmap_before = get_best_cmap(DEJAU)
    dst_cmap = get_best_cmap(dst)
    added = set(dst_cmap.keys()) - set(src_cmap_before.keys())
    assert added, "Expected at least one codepoint to be added to dst font"


def test_verbosity_flag_runs(tmp_path):
    ensure_fonts_exist()

    src_copy = tmp_path / "src_v.ttf"
    dst = tmp_path / "dst_v.ttf"
    shutil.copy(str(DEJAU), str(src_copy))

    # Run with verbose flag
    run_script(["-vv", "--src-font", str(src_copy), "--combine-font", str(NOTO_VAR), "--dst-font", str(dst)])
    assert dst.exists()


def test_src_dir_processing(tmp_path):
    """Process all .ttf files in a source directory and write results to dst-dir."""
    ensure_fonts_exist()

    src_dir = tmp_path / "src_dir"
    src_dir.mkdir()
    # Create two source font files
    a = src_dir / "a.ttf"
    b = src_dir / "b.ttf"
    shutil.copy(str(DEJAU), str(a))
    shutil.copy(str(DEJAU), str(b))

    dst_dir = tmp_path / "out"

    # Capture cmaps before
    a_before = get_best_cmap(a)

    run_script(["--src-dir", str(src_dir), "--combine-font", str(NOTO_VAR), "--dst-dir", str(dst_dir)])

    # Outputs should exist
    a_out = dst_dir / "a.ttf"
    b_out = dst_dir / "b.ttf"
    assert a_out.exists() and b_out.exists()

    a_after = get_best_cmap(a_out)
    # At least one codepoint should have been added compared to the original
    added = set(a_after.keys()) - set(a_before.keys())
    assert added, "Expected at least one codepoint to be added to a.ttf"


def test_src_dir_requires_dst_dir(tmp_path):
    """Using --src-dir without --dst-dir should fail with an error."""
    ensure_fonts_exist()

    src_dir = tmp_path / "src_dir2"
    src_dir.mkdir()
    shutil.copy(str(DEJAU), str(src_dir / "one.ttf"))

    cmd = [sys.executable, str(SCRIPT), "--src-dir", str(src_dir), "--combine-font", str(NOTO_VAR)]
    res = subprocess.run(cmd, cwd=str(REPO_ROOT), capture_output=True, text=True)
    assert res.returncode != 0
    assert "--dst-dir is required" in res.stderr or "required when --src-dir" in res.stderr


def test_src_dir_disallows_dst_font(tmp_path):
    """Using --dst-font together with --src-dir should be rejected."""
    ensure_fonts_exist()

    src_dir = tmp_path / "src_dir3"
    src_dir.mkdir()
    shutil.copy(str(DEJAU), str(src_dir / "one.ttf"))

    cmd = [sys.executable, str(SCRIPT), "--src-dir", str(src_dir), "--combine-font", str(NOTO_VAR), "--dst-font", str(tmp_path / "out.ttf")]
    res = subprocess.run(cmd, cwd=str(REPO_ROOT), capture_output=True, text=True)
    assert res.returncode != 0
    assert "--dst-font cannot be used with --src-dir" in res.stderr or "cannot be used with --src-dir" in res.stderr
