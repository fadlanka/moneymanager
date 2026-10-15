//! Receipt image storage on local disk (a Docker volume in production).

use std::path::{Path, PathBuf};

use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Storage {
    root: PathBuf,
}

/// Detect the image type from magic bytes; the client's Content-Type is not trusted.
pub fn sniff_image(bytes: &[u8]) -> Option<(&'static str, &'static str)> {
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some(("image/jpeg", "jpg"))
    } else if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        Some(("image/png", "png"))
    } else if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some(("image/webp", "webp"))
    } else if bytes.len() >= 12
        && &bytes[4..8] == b"ftyp"
        && matches!(&bytes[8..12], b"heic" | b"heix" | b"mif1" | b"msf1")
    {
        // iPhone photos. Gemini reads HEIC; most local models do not.
        Some(("image/heic", "heic"))
    } else {
        None
    }
}

impl Storage {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub async fn ensure_root(&self) -> std::io::Result<()> {
        tokio::fs::create_dir_all(&self.root).await
    }

    /// Returns the path relative to the storage root, which is what the DB stores.
    pub async fn save(
        &self,
        user_id: Uuid,
        receipt_id: Uuid,
        ext: &str,
        bytes: &[u8],
    ) -> std::io::Result<String> {
        let rel = format!("{user_id}/{receipt_id}.{ext}");
        let full = self.root.join(&rel);
        if let Some(dir) = full.parent() {
            tokio::fs::create_dir_all(dir).await?;
        }
        tokio::fs::write(&full, bytes).await?;
        Ok(rel)
    }

    pub async fn read(&self, rel: &str) -> std::io::Result<Vec<u8>> {
        tokio::fs::read(self.resolve(rel)?).await
    }

    fn resolve(&self, rel: &str) -> std::io::Result<PathBuf> {
        // Paths come from our own DB, but refuse traversal anyway.
        let p = Path::new(rel);
        if p.is_absolute()
            || p.components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "bad path",
            ));
        }
        Ok(self.root.join(p))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sniffs_common_formats() {
        assert_eq!(
            sniff_image(&[0xFF, 0xD8, 0xFF, 0xE0]).unwrap().0,
            "image/jpeg"
        );
        assert_eq!(
            sniff_image(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0])
                .unwrap()
                .0,
            "image/png"
        );
        assert!(sniff_image(b"%PDF-1.7").is_none());
    }

    #[test]
    fn refuses_traversal() {
        let s = Storage::new("/tmp/x");
        assert!(s.resolve("../etc/passwd").is_err());
        assert!(s.resolve("/etc/passwd").is_err());
        assert!(s.resolve("u/r.jpg").is_ok());
    }
}
