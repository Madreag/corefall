//! Photo PNG export. cf-app supplies the rendered RGB byte buffer +
//! width/height + path; cf-photo writes the PNG using the workspace
//! `image` crate's PNG encoder (already a dep for asset bake).

use std::path::Path;

use thiserror::Error;

/// Failure modes for [`export_png`].
#[derive(Debug, Error)]
pub enum ExportError {
    /// Image crate produced an encoding error.
    #[error("photo export encode failed: {0}")]
    Encode(String),
    /// Buffer size doesn't match the supplied dimensions.
    #[error("photo export buffer mismatch: expected {expected} bytes, got {actual}")]
    BufferMismatch {
        /// Expected byte count = w * h * 3.
        expected: usize,
        /// Supplied byte count.
        actual: usize,
    },
    /// Invalid dimension (zero width/height).
    #[error("photo export invalid dimensions: width={width} height={height}")]
    InvalidDimensions {
        /// Supplied width.
        width: u32,
        /// Supplied height.
        height: u32,
    },
}

/// Encode `buf` (`width * height * 3` interleaved RGB bytes) to a PNG at
/// `path`. The image crate's PNG encoder lives behind the `png` feature
/// already enabled in the workspace.
pub fn export_png(path: impl AsRef<Path>, buf: &[u8], width: u32, height: u32) -> Result<(), ExportError> {
    if width == 0 || height == 0 {
        return Err(ExportError::InvalidDimensions { width, height });
    }
    let expected = (width as usize) * (height as usize) * 3;
    if buf.len() != expected {
        return Err(ExportError::BufferMismatch {
            expected,
            actual: buf.len(),
        });
    }
    let buffer = image::RgbImage::from_raw(width, height, buf.to_vec()).ok_or(ExportError::BufferMismatch {
        expected,
        actual: buf.len(),
    })?;
    buffer
        .save_with_format(path.as_ref(), image::ImageFormat::Png)
        .map_err(|e| ExportError::Encode(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_rejects_zero_dim() {
        let err = export_png("/tmp/cf-photo-zero.png", &[], 0, 10);
        assert!(matches!(err, Err(ExportError::InvalidDimensions { .. })));
    }

    #[test]
    fn export_rejects_buffer_mismatch() {
        let err = export_png("/tmp/cf-photo-mismatch.png", &[0u8; 4], 2, 2);
        assert!(matches!(err, Err(ExportError::BufferMismatch { .. })));
    }

    #[test]
    fn export_writes_valid_png() {
        let tmp = std::env::temp_dir().join("cf_photo_export_test.png");
        let buf = vec![128u8; 4 * 4 * 3];
        let res = export_png(&tmp, &buf, 4, 4);
        assert!(res.is_ok(), "{:?}", res);
        let _ = std::fs::remove_file(&tmp);
    }
}
