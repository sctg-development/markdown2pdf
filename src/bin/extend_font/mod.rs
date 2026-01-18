//! Extend font binary modules
//!
//! This module contains the implementation of the extend_font binary,
//! which extends a source font by copying missing glyphs from a combine font.

pub mod args;
pub mod cmap_builder;
pub mod font_subsetter;
pub mod font_utils;
pub mod glyph_copier;
pub mod glyph_copier_impl;
pub mod logging;
pub mod write_fonts_analysis;
