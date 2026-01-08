//! Image loading and processing for markdown-to-pdf conversion.
//!
//! This module provides functionality to load images from both local paths and remote URLs,
//! with support for relative path resolution and caching.
//!
//! # Features
//! - Load images from local filesystem paths
//! - Download images from HTTP(S) URLs
//! - Resolve relative paths based on document location
//! - Cache downloaded images to avoid repeated downloads
//!
//! # Example
//!
//! ```rust
//! use markdown2pdf::images::{ImageLoader, ImageFormat};
//! use std::path::Path;
//!
//! // Create a loader for a document at a specific location
//! let loader = ImageLoader::new(Some(Path::new("./document.md")));
//!
//! // This would fail in doctests since we can't actually load files,
//! // but shows the intended API:
//! // let image = loader.load("./images/photo.jpg").await.unwrap();
//! ```

use log::{debug, info, warn, error};use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Represents different image formats supported by the library.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    /// JPEG image format
    Jpeg,
    /// PNG image format
    Png,
    /// SVG image format
    Svg,
    /// WebP image format
    WebP,
    /// GIF image format
    Gif,
}

impl ImageFormat {
    /// Detect image format from file extension or magic bytes.
    ///
    /// # Example
    ///
    /// ```
    /// use markdown2pdf::images::ImageFormat;
    /// assert_eq!(ImageFormat::from_path("photo.jpg"), Some(ImageFormat::Jpeg));
    /// assert_eq!(ImageFormat::from_path("image.png"), Some(ImageFormat::Png));
    /// assert_eq!(ImageFormat::from_path("math.svg"), Some(ImageFormat::Svg));
    /// ```
    pub fn from_path(path: &str) -> Option<ImageFormat> {
        let lower = path.to_lowercase();
        if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
            Some(ImageFormat::Jpeg)
        } else if lower.ends_with(".png") {
            Some(ImageFormat::Png)
        } else if lower.ends_with(".svg") {
            Some(ImageFormat::Svg)
        } else if lower.ends_with(".webp") {
            Some(ImageFormat::WebP)
        } else if lower.ends_with(".gif") {
            Some(ImageFormat::Gif)
        } else {
            None
        }
    }

    /// Get the MIME type for this image format.
    ///
    /// # Example
    ///
    /// ```
    /// use markdown2pdf::images::ImageFormat;
    /// assert_eq!(ImageFormat::Jpeg.mime_type(), "image/jpeg");
    /// assert_eq!(ImageFormat::Png.mime_type(), "image/png");
    /// ```
    pub fn mime_type(&self) -> &'static str {
        match self {
            ImageFormat::Jpeg => "image/jpeg",
            ImageFormat::Png => "image/png",
            ImageFormat::Svg => "image/svg+xml",
            ImageFormat::WebP => "image/webp",
            ImageFormat::Gif => "image/gif",
        }
    }
}

/// Error types for image operations.
#[derive(Debug)]
pub enum ImageError {
    /// Failed to load image from local filesystem
    LoadError(String),
    /// Failed to download image from remote URL
    DownloadError(String),
    /// Failed to resolve image path
    PathResolutionError(String),
    /// Unsupported image format
    UnsupportedFormat(String),
}

impl std::fmt::Display for ImageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImageError::LoadError(e) => write!(f, "Failed to load image: {}", e),
            ImageError::DownloadError(e) => write!(f, "Failed to download image: {}", e),
            ImageError::PathResolutionError(e) => write!(f, "Failed to resolve path: {}", e),
            ImageError::UnsupportedFormat(e) => write!(f, "Unsupported image format: {}", e),
        }
    }
}

impl std::error::Error for ImageError {}

/// Represents loaded image data with metadata.
#[derive(Debug, Clone)]
pub struct ImageData {
    /// Raw image bytes
    pub bytes: Vec<u8>,
    /// Image format
    pub format: ImageFormat,
    /// Original URL or path
    pub source: String,
}

/// Manages loading and caching of images for a document.
///
/// Provides automatic path resolution for relative paths and handles both
/// local filesystem and remote HTTP(S) image loading.
pub struct ImageLoader {
    /// Base directory for resolving relative paths
    base_dir: Option<PathBuf>,
    /// Cache of loaded images to avoid redownloading
    cache: HashMap<String, ImageData>,
    /// Whether to enable remote image downloading
    allow_remote: bool,
}

impl ImageLoader {
    /// Create a new image loader for a document at the specified path.
    ///
    /// The base directory is extracted from the document path and used
    /// to resolve relative image references.
    ///
    /// # Arguments
    /// * `document_path` - Path to the markdown document (optional)
    ///
    /// # Example
    ///
    /// ```rust
    /// use markdown2pdf::images::ImageLoader;
    /// use std::path::Path;
    ///
    /// let loader = ImageLoader::new(Some(Path::new("./docs/document.md")));
    /// // Images will be resolved relative to ./docs/
    /// ```
    pub fn new(document_path: Option<&Path>) -> Self {
        let base_dir = document_path
            .and_then(|p| p.parent())
            .map(|p| p.to_path_buf());
        ImageLoader {
            base_dir,
            cache: HashMap::new(),
            allow_remote: true,
        }
    }

    /// Enable or disable remote image downloading.
    ///
    /// # Example
    ///
    /// ```rust
    /// use markdown2pdf::images::ImageLoader;
    /// let mut loader = ImageLoader::new(None);
    /// loader.set_allow_remote(false);
    /// // Will not download images from URLs
    /// ```
    pub fn set_allow_remote(&mut self, allow: bool) {
        self.allow_remote = allow;
    }

    /// Resolve an image path relative to the document location.
    ///
    /// For absolute URLs (http/https), returns the URL as-is.
    /// For relative paths, resolves them relative to the document directory.
    ///
    /// # Arguments
    /// * `url_or_path` - Image URL or relative path
    ///
    /// # Returns
    /// Either an absolute path string for local images or a full URL for remote images
    ///
    /// # Example
    ///
    /// ```rust
    /// use markdown2pdf::images::ImageLoader;
    /// use std::path::Path;
    ///
    /// let loader = ImageLoader::new(Some(Path::new("./docs/document.md")));
    /// // Relative path gets resolved
    /// // loader.resolve_path("images/photo.jpg") -> "/full/path/to/docs/images/photo.jpg"
    /// // HTTP URL stays as-is
    /// // loader.resolve_path("https://example.com/image.jpg") -> "https://example.com/image.jpg"
    /// ```
    pub fn resolve_path(&self, url_or_path: &str) -> Result<String, ImageError> {
        // Check if it's a remote URL
        if url_or_path.starts_with("http://") || url_or_path.starts_with("https://") {
            return Ok(url_or_path.to_string());
        }

        // If no base directory, use path as-is
        let Some(ref base) = self.base_dir else {
            eprintln!(
                "[ImageLoader] No base directory, using path as-is: {}",
                url_or_path
            );
            return Ok(url_or_path.to_string());
        };

        // Resolve relative path
        let resolved = base.join(url_or_path);
        let resolved_str = resolved.to_str().map(|s| s.to_string()).ok_or_else(|| {
            ImageError::PathResolutionError(format!(
                "Failed to convert path to string: {:?}",
                resolved
            ))
        })?;

        eprintln!(
            "[ImageLoader] Resolved: {} + {} = {}",
            base.display(),
            url_or_path,
            resolved_str
        );
        Ok(resolved_str)
    }

    /// Load an image from a URL or path.
    ///
    /// Attempts to load from cache first, then from local filesystem or remote URL.
    /// Returns image data along with format information.
    ///
    /// # Arguments
    /// * `url_or_path` - Image URL or relative path
    ///
    /// # Errors
    /// Returns `ImageError` if loading fails or image format is unsupported.
    pub fn load(&mut self, url_or_path: &str) -> Result<ImageData, ImageError> {
        // Check cache first
        if let Some(data) = self.cache.get(url_or_path) {
            return Ok(data.clone());
        }

        let resolved = self.resolve_path(url_or_path)?;

        // Detect format
        let format = ImageFormat::from_path(&resolved)
            .ok_or_else(|| ImageError::UnsupportedFormat(resolved.clone()))?;

        // Load the image
        let data = if resolved.starts_with("http://") || resolved.starts_with("https://") {
            if !self.allow_remote {
                return Err(ImageError::DownloadError(
                    "Remote images are disabled".to_string(),
                ));
            }
            self.load_remote(&resolved)?
        } else {
            self.load_local(&resolved)?
        };

        let image_data = ImageData {
            bytes: data,
            format,
            source: url_or_path.to_string(),
        };

        // Cache the result
        self.cache
            .insert(url_or_path.to_string(), image_data.clone());

        Ok(image_data)
    }

    /// Load an image from the local filesystem.
    ///
    /// # Errors
    /// Returns `ImageError::LoadError` if the file cannot be read.
    fn load_local(&self, path: &str) -> Result<Vec<u8>, ImageError> {
        debug!("[ImageLoader] Loading local file: {}", path);

        let bytes = std::fs::read(path).map_err(|e| {
            debug!("[ImageLoader] Failed to read file {}: {}", path, e);
            ImageError::LoadError(format!("Failed to read file {}: {}", path, e))
        })?;

        eprintln!(
            "[ImageLoader] Successfully read {} bytes from {}",
            bytes.len(),
            path
        );
        Ok(bytes)
    }

    /// Download an image from a remote URL.
    ///
    /// Requires the `fetch` feature to be enabled.
    ///
    /// # Errors
    /// Returns `ImageError::DownloadError` if the download fails.
    fn load_remote(&self, url: &str) -> Result<Vec<u8>, ImageError> {
        if !cfg!(feature = "fetch") {
            return Err(ImageError::DownloadError(format!(
                "Remote image loading from {} requires the 'fetch' feature",
                url
            )));
        }
        #[cfg(feature = "fetch")]
        {
            let client = reqwest::blocking::Client::new();
            let response = client.get(url).send().map_err(|e| {
                ImageError::DownloadError(format!("Failed to download {}: {}", url, e))
            })?;

            response
                .bytes()
                .map(|b| b.to_vec())
                .map_err(|e| ImageError::DownloadError(format!("Failed to read response: {}", e)))
        }

        #[cfg(not(feature = "fetch"))]
        {
            Err(ImageError::DownloadError(
                "Remote image loading requires the 'fetch' feature".to_string(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_format_detection() {
        assert_eq!(ImageFormat::from_path("photo.jpg"), Some(ImageFormat::Jpeg));
        assert_eq!(
            ImageFormat::from_path("photo.jpeg"),
            Some(ImageFormat::Jpeg)
        );
        assert_eq!(ImageFormat::from_path("image.png"), Some(ImageFormat::Png));
        assert_eq!(ImageFormat::from_path("math.svg"), Some(ImageFormat::Svg));
        assert_eq!(
            ImageFormat::from_path("photo.webp"),
            Some(ImageFormat::WebP)
        );
        assert_eq!(
            ImageFormat::from_path("animation.gif"),
            Some(ImageFormat::Gif)
        );
        assert_eq!(ImageFormat::from_path("unknown.txt"), None);
    }

    #[test]
    fn test_mime_types() {
        assert_eq!(ImageFormat::Jpeg.mime_type(), "image/jpeg");
        assert_eq!(ImageFormat::Png.mime_type(), "image/png");
        assert_eq!(ImageFormat::Svg.mime_type(), "image/svg+xml");
        assert_eq!(ImageFormat::WebP.mime_type(), "image/webp");
        assert_eq!(ImageFormat::Gif.mime_type(), "image/gif");
    }

    #[test]
    fn test_remote_url_resolution() {
        let loader = ImageLoader::new(Some(Path::new("/path/to/document.md")));
        assert_eq!(
            loader
                .resolve_path("https://example.com/image.jpg")
                .unwrap(),
            "https://example.com/image.jpg"
        );
        assert_eq!(
            loader.resolve_path("http://example.com/image.png").unwrap(),
            "http://example.com/image.png"
        );
    }

    #[test]
    fn test_relative_path_resolution() {
        let loader = ImageLoader::new(Some(Path::new("/path/to/document.md")));
        let resolved = loader.resolve_path("images/photo.jpg").unwrap();
        assert!(resolved.contains("path/to/images/photo.jpg"));
    }

    #[test]
    fn test_image_loader_no_base_dir() {
        let loader = ImageLoader::new(None);
        let resolved = loader.resolve_path("images/photo.jpg").unwrap();
        assert_eq!(resolved, "images/photo.jpg");
    }

    #[test]
    fn test_image_loader_caching() {
        let mut loader = ImageLoader::new(None);
        loader.allow_remote = false;

        // Try loading a non-existent file once
        let result = loader.load("nonexistent.jpg");
        assert!(result.is_err());

        // Second call should also fail (not cached as error)
        let result = loader.load("nonexistent.jpg");
        assert!(result.is_err());
    }
}
