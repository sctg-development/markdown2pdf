#!/usr/bin/env python3
"""
extend_font.py

Copy missing glyphs from one font (combine font) into another (src font).

Features:
- Verbosity control (-v, -vv, -vvv, -q, --verbose=LEVEL)
- --src-font: source font to add glyphs to
- --src-dir: directory of source fonts to process (all *.ttf files will be processed individually)
- --combine-font: font containing glyphs to copy
- --dst-font: destination font to write (optional). If omitted, src font is modified in place.
- --dst-dir: destination directory to write modified fonts when using --src-dir
- If --dst-font contains a directory path that does not exist, it will be created.
- If combine font is variable and src font is fixed, the combine font will be instantiated
  (frozen) at the weight of the src font (OS/2 usWeightClass) before copying glyphs.

Implementation notes:
- Uses fontTools (TTFont) and fontTools.varLib.instancer.instantiateVariableFont
  to create fixed instances of variable fonts when needed.
- Copies TrueType outlines (glyf) and horizontal metrics (hmtx). CFF support is intentionally
  left out to keep implementation focused and testable for common TTF cases used by tests.
- Copies composite glyphs recursively so components are present.

The code is designed to be well-documented and unit-tested (see tests/extend_font_test.py).
"""

from __future__ import annotations

import argparse
import copy
import logging
import os
from pathlib import Path
import sys
from typing import Dict, Optional, Set, Tuple

from fontTools.ttLib import TTFont
from fontTools.varLib.instancer import instantiateVariableFont


__all__ = ["copy_missing_glyphs", "main"]


logger = logging.getLogger("extend_font")


def configure_logging(verbosity: int) -> None:
    """Configure logging level based on -v/-q/--verbose settings.

    Verbosity level mapping:
      -v -> INFO
      -vv -> DEBUG
      -q -> WARNING (quiet)
      --verbose=N -> custom mapping (0=WARNING,1=INFO,2=DEBUG,3=DEBUG)
    """
    if verbosity <= 0:
        level = logging.WARNING
    elif verbosity == 1:
        level = logging.INFO
    else:
        level = logging.DEBUG
    logging.basicConfig(level=level, format="%(levelname)s: %(message)s")


def is_variable_font(tt: TTFont) -> bool:
    return "fvar" in tt


def get_best_cmap(tt: TTFont) -> Dict[int, str]:
    """Return the best cmap (Unicode -> glyphName) for a font or empty dict."""
    try:
        cmap = tt.getBestCmap()
        return dict(cmap or {})
    except Exception:
        return {}


def instantiate_combine_if_needed(combine: TTFont, src: TTFont) -> TTFont:
    """If combine is variable and src is fixed, instantiate combine at src weight.

    The target weight is taken from src['OS/2'].usWeightClass when available.
    Returns a TTFont instance (may be the original combine if no instantiation needed).
    """
    if not is_variable_font(combine):
        logger.debug("Combine font is not variable; no instantiation needed")
        return combine

    if is_variable_font(src):
        logger.debug("Both fonts are variable; skipping instantiation")
        return combine

    # Determine target weight from src
    weight_value = None
    try:
        weight_value = int(src["OS/2"].usWeightClass)
        logger.info("Instantiating variable combine font at weight %d", weight_value)
    except Exception:
        logger.warning("Could not determine weight from src font; defaulting to 400 (Regular)")
        weight_value = 400

    # Try to instantiate on wght axis; if axis missing, default to 400.
    axis_settings = {"wght": weight_value}
    try:
        inst = instantiateVariableFont(combine, axis_settings, inplace=False)
        logger.debug("Instantiation successful; returned fixed font instance")
        return inst
    except Exception as e:
        logger.warning("Could not instantiate variable font (falling back): %s", e)
        return combine


def pick_cmap_subtables_for_writing(tt: TTFont):
    """Return list of cmap subtables that accept Unicode mappings to write into.

    Prefers Windows Unicode subtables (platformID=3, platEncID in 1 or 10), else
    returns all subtables that are Unicode-capable.
    """
    subtables = []
    if "cmap" not in tt:
        return subtables
    for t in tt["cmap"].tables:
        try:
            if t.isUnicode():
                subtables.append(t)
        except AttributeError:
            # Older fontTools subtable object
            subtables.append(t)
    return subtables


def add_cmap_mapping(tt: TTFont, codepoint: int, glyph_name: str) -> None:
    """Add a codepoint -> glyph mapping to an appropriate cmap subtable.

    - For codepoints > U+FFFF prefer a format 12 (platform 3, encoding 10) subtable.
    - For BMP codepoints, add to existing Unicode subtables (format 4 or 12) and
      create a format 4 table if none exists.
    """
    # Ensure a cmap table exists
    if "cmap" not in tt:
        from fontTools.ttLib import newTable

        table = newTable("cmap")
        table.tables = []
        tt["cmap"] = table

    # For codepoints outside BMP, prefer a format 12 / platform 3 platEncID 10 subtable
    if codepoint > 0xFFFF:
        for sub in tt["cmap"].tables:
            fmt = getattr(sub, "format", None)
            if fmt == 12 or (getattr(sub, "platformID", None) == 3 and getattr(sub, "platEncID", None) == 10):
                sub.cmap[codepoint] = glyph_name
                return
        # create a new format 12 table
        from fontTools.ttLib.tables._c_m_a_p import cmap_format_12

        new = cmap_format_12(3, 10)
        new.cmap = {codepoint: glyph_name}
        tt["cmap"].tables.append(new)
        return

    # BMP codepoint: add to all existing Unicode subtables where it fits
    written = False
    for sub in tt["cmap"].tables:
        try:
            if sub.isUnicode():
                # format 4 can't handle >0xFFFF, but this path is for BMP, so it's fine
                sub.cmap[codepoint] = glyph_name
                written = True
        except Exception:
            # Fallback: try to write regardless
            try:
                sub.cmap[codepoint] = glyph_name
                written = True
            except Exception:
                continue

    if not written:
        # No unicode subtable to write into; create a format 4 (Windows BMP) table
        from fontTools.ttLib.tables._c_m_a_p import cmap_format_4

        new = cmap_format_4(3, 1)
        new.cmap = {codepoint: glyph_name}
        tt["cmap"].tables.append(new)
        return


def copy_glyph_recursive(src_font: TTFont, glyph_name: str, dst_font: TTFont, copied: Set[str]) -> None:
    """Copy a glyph from src_font into dst_font, including components recursively.

    Args:
        src_font: source font that contains the glyph (combine/instantiated)
        glyph_name: the glyph name to copy
        dst_font: the font to copy into (src font)
        copied: set used to avoid infinite recursion / duplicate copies
    """
    if glyph_name in dst_font.getGlyphOrder():
        logger.debug("Glyph '%s' already present in destination; skipping", glyph_name)
        return
    if glyph_name in copied:
        logger.debug("Glyph '%s' already copied in this run; skipping", glyph_name)
        return

    logger.info("Copying glyph '%s'", glyph_name)
    copied.add(glyph_name)

    # Only handle TrueType glyf outlines for now
    if "glyf" in src_font and "glyf" in dst_font:
        src_glyf = src_font["glyf"].glyphs.get(glyph_name)
        if src_glyf is None:
            logger.warning("Glyph '%s' not found in combine glyf table; skipping", glyph_name)
            return

        # If composite, ensure components are present in dst
        try:
            is_comp = getattr(src_glyf, "isComposite", lambda: False)()
        except Exception:
            # Some glyf objects may not implement isComposite as a method
            is_comp = getattr(src_glyf, "isComposite", False)

        if is_comp:
            for comp in src_glyf.components:
                comp_name = comp.glyphName
                if comp_name not in dst_font.getGlyphOrder():
                    copy_glyph_recursive(src_font, comp_name, dst_font, copied)

        # Deep copy the glyf object
        dst_font["glyf"].glyphs[glyph_name] = copy.deepcopy(src_glyf)

        # Copy hmtx metric if present
        if "hmtx" in src_font and "hmtx" in dst_font:
            metric = src_font["hmtx"].metrics.get(glyph_name)
            if metric is not None:
                dst_font["hmtx"].metrics[glyph_name] = metric

        # Add glyph to glyph order at the end
        order = dst_font.getGlyphOrder()
        order.append(glyph_name)
        dst_font.setGlyphOrder(order)

        # Update maxp
        try:
            dst_font["maxp"].numGlyphs = len(order)
        except Exception:
            pass

    else:
        raise NotImplementedError("Only TrueType glyf fonts are supported by this script for glyph copying")


def copy_missing_glyphs(src_path: str, combine_path: str, dst_path: Optional[str] = None, verbosity: int = 0) -> Tuple[int, Set[int]]:
    """Main function to copy missing glyphs.

    Returns a tuple (num_glyphs_added, set_of_codepoints_added).
    """
    configure_logging(verbosity)

    src_path = Path(src_path)
    combine_path = Path(combine_path)

    if not src_path.exists():
        logger.error("Source font '%s' does not exist", src_path)
        raise FileNotFoundError(str(src_path))
    if not combine_path.exists():
        logger.error("Combine font '%s' does not exist", combine_path)
        raise FileNotFoundError(str(combine_path))

    if dst_path is None:
        dst_path = str(src_path)
    else:
        dst_path = str(dst_path)
        dst_dir = os.path.dirname(dst_path)
        if dst_dir and not os.path.exists(dst_dir):
            logger.info("Creating directories for destination %s", dst_dir)
            os.makedirs(dst_dir, exist_ok=True)

    # Load fonts
    logger.debug("Loading src font: %s", src_path)
    src_font = TTFont(str(src_path))
    logger.debug("Loading combine font: %s", combine_path)
    combine_font = TTFont(str(combine_path))

    # If combine is variable and src is fixed, instantiate combine
    combined_for_copy = instantiate_combine_if_needed(combine_font, src_font)

    combine_cmap = get_best_cmap(combined_for_copy)
    src_cmap = get_best_cmap(src_font)

    missing_codepoints = set(combine_cmap.keys()) - set(src_cmap.keys())
    logger.info("Found %d candidate codepoints in combine font, %d missing in src", len(combine_cmap), len(missing_codepoints))

    if not missing_codepoints:
        logger.info("No missing glyphs to copy")
        return 0, set()

    added_count = 0
    added_codepoints: Set[int] = set()
    copied_glyphs: Set[str] = set()

    for cp in sorted(missing_codepoints):
        glyph_name = combine_cmap.get(cp)
        if glyph_name is None:
            continue
        try:
            copy_glyph_recursive(combined_for_copy, glyph_name, src_font, copied_glyphs)
        except NotImplementedError as e:
            logger.error("Could not copy glyph '%s' (codepoint U+%04X): %s", glyph_name, cp, e)
            continue

        # Add cmap mapping(s)
        try:
            add_cmap_mapping(src_font, cp, glyph_name)
        except Exception:
            logger.warning("Failed to add cmap mapping for U+%04X -> %s", cp, glyph_name)
            continue

        added_count += 1
        added_codepoints.add(cp)

    # Save result
    logger.info("Adding %d codepoints -> %d glyphs", len(added_codepoints), len(copied_glyphs))
    logger.debug("Saving destination font to %s", dst_path)
    src_font.save(dst_path)

    return added_count, added_codepoints


def parse_args(argv):
    parser = argparse.ArgumentParser(description="Copy missing glyphs from a combine font into a source font")

    # Either a single font or a directory of fonts must be provided (mutually exclusive)
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--src-font", help="Source font to add glyphs to (mutually exclusive with --src-dir)")
    group.add_argument("--src-dir", help="Directory containing source .ttf fonts to process (mutually exclusive with --src-font)")

    parser.add_argument("--combine-font", required=True, help="Font containing glyphs to copy")
    parser.add_argument("--dst-font", help="Destination font to write (optional). If omitted, src font is modified in place")
    parser.add_argument("--dst-dir", help="Destination directory to write modified fonts (required when --src-dir is used)")

    # Verbosity handling: -v counts, -q quiet, --verbose=LEVEL overrides
    parser.add_argument("-v", action="count", default=0, help="Increase verbosity (can be specified multiple times)")
    parser.add_argument("-q", action="store_true", help="Quiet mode (minimal output)")
    parser.add_argument("--verbose", type=int, help="Set verbosity level explicitly (0=warning,1=info,2=debug)")

    args = parser.parse_args(argv)

    # When a directory of source fonts is provided, a destination directory is required
    if args.src_dir and not args.dst_dir:
        parser.error("--dst-dir is required when --src-dir is specified")

    # --dst-font does not make sense when processing a directory
    if args.src_dir and args.dst_font:
        parser.error("--dst-font cannot be used with --src-dir; use --dst-dir instead")

    return args


def main(argv=None):
    args = parse_args(argv if argv is not None else sys.argv[1:])

    if args.q:
        verbosity = 0
    elif args.verbose is not None:
        verbosity = max(0, min(3, int(args.verbose)))
    else:
        verbosity = args.v

    # Directory mode: process all .ttf files in the source directory and write to dst-dir
    if getattr(args, "src_dir", None):
        src_dir = Path(args.src_dir)
        dst_dir = Path(args.dst_dir)

        if not src_dir.exists() or not src_dir.is_dir():
            logger.error("Source directory '%s' does not exist or is not a directory", src_dir)
            return 2
        if not dst_dir.exists():
            logger.info("Creating destination directory '%s'", dst_dir)
            dst_dir.mkdir(parents=True, exist_ok=True)

        ttf_files = sorted([p for p in src_dir.iterdir() if p.is_file() and p.suffix.lower() == ".ttf"])
        if not ttf_files:
            logger.warning("No .ttf files found in source directory '%s'", src_dir)
            return 0

        total_added = 0
        failed = 0
        for src_path in ttf_files:
            dst_path = dst_dir / src_path.name
            try:
                added, cps = copy_missing_glyphs(str(src_path), args.combine_font, str(dst_path), verbosity=verbosity)
                logger.info("Processed '%s' -> added %d codepoints", src_path.name, len(cps))
                total_added += added
            except Exception as e:
                logger.error("Failed processing '%s': %s", src_path.name, e)
                failed += 1

        logger.info("Directory processing complete: %d files processed, %d total codepoints added, %d failures", len(ttf_files), total_added, failed)
        if failed:
            return 1
        return 0

    # Single file mode
    added, cps = copy_missing_glyphs(args.src_font, args.combine_font, args.dst_font, verbosity=verbosity)
    logger.info("Completed: %d codepoints added", len(cps))

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
