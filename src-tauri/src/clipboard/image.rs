use std::path::{Path, PathBuf};

pub struct TempImage {
    pub path: PathBuf,
}

impl TempImage {
    /// Write PNG bytes into an app-specific temp directory under a uniquely named file.
    /// Caller deletes on success (or leaves for debugging on failure, per spec).
    pub fn write(app_temp_dir: &Path, png_bytes: &[u8]) -> std::io::Result<TempImage> {
        std::fs::create_dir_all(app_temp_dir)?;
        let unique = format!(
            "clipboard-{}-{}.png",
            chrono::Utc::now().format("%Y%m%d-%H%M%S-%3f"),
            rand::random::<u32>()
        );
        let path = app_temp_dir.join(unique);
        std::fs::write(&path, png_bytes)?;
        Ok(TempImage { path })
    }

    pub fn delete(&self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_creates_unique_file() {
        let dir = tempfile::tempdir().unwrap();
        let a = TempImage::write(dir.path(), b"fake png data").unwrap();
        let b = TempImage::write(dir.path(), b"fake png data").unwrap();
        assert_ne!(a.path, b.path);
        assert!(a.path.exists());
        assert!(b.path.exists());
        a.delete();
        b.delete();
    }
}
