use anyhow::Result;
use std::path::Path;
use tokio::io::AsyncReadExt;

#[derive(thiserror::Error, Debug)]
pub enum ValidationError {
    #[error("File not found: {0}")]
    FileNotFound(String),
    #[error("Invalid AppImage magic bytes")]
    InvalidMagicBytes,
    #[error("IO error while reading file: {0}")]
    IoError(#[from] std::io::Error),
}

pub async fn validate_appimage(path: &Path) -> Result<()> {
    tracing::debug!("Validating AppImage at {:?}", path);

    if !path.exists() {
        return Err(ValidationError::FileNotFound(path.to_string_lossy().to_string()).into());
    }

    let mut file = tokio::fs::File::open(path)
        .await
        .map_err(ValidationError::IoError)?;
    let mut buffer = [0u8; 11];

    // Ler os primeiros 11 bytes para verificar cabeçalhos ELF e AppImage
    if file.read_exact(&mut buffer).await.is_err() {
        return Err(ValidationError::InvalidMagicBytes.into());
    }

    // ELF magic bytes: 0x7F 'E' 'L' 'F'
    let is_elf = buffer[0..4] == [0x7f, 0x45, 0x4c, 0x46];

    // AppImage magic bytes at offset 8: 'A' 'I' 0x01 or 0x02
    let is_appimage = buffer[8..10] == [0x41, 0x49] && (buffer[10] == 0x01 || buffer[10] == 0x02);

    if !is_elf || !is_appimage {
        tracing::error!(
            "Failed magic bytes check. ELF: {}, AppImage: {}",
            is_elf,
            is_appimage
        );
        return Err(ValidationError::InvalidMagicBytes.into());
    }

    tracing::info!("AppImage validated successfully.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    /// Gera 11 bytes válidos de um AppImage Tipo 2 (ELF + assinatura AI\x02).
    fn valid_appimage_header() -> Vec<u8> {
        let mut bytes = vec![0u8; 64];
        // ELF magic
        bytes[0] = 0x7f;
        bytes[1] = b'E';
        bytes[2] = b'L';
        bytes[3] = b'F';
        // AppImage magic at offset 8
        bytes[8] = b'A';
        bytes[9] = b'I';
        bytes[10] = 0x02;
        bytes
    }

    #[tokio::test]
    async fn validates_correct_appimage_type2() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("valid.AppImage");
        tokio::fs::write(&path, valid_appimage_header())
            .await
            .unwrap();
        assert!(validate_appimage(&path).await.is_ok());
    }

    #[tokio::test]
    async fn validates_correct_appimage_type1() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("valid_t1.AppImage");
        let mut header = valid_appimage_header();
        header[10] = 0x01; // Type 1
        tokio::fs::write(&path, header).await.unwrap();
        assert!(validate_appimage(&path).await.is_ok());
    }

    #[tokio::test]
    async fn rejects_file_with_invalid_elf_magic() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("notelf.AppImage");
        let mut header = valid_appimage_header();
        header[0] = 0x00; // ELF magic quebrado
        tokio::fs::write(&path, header).await.unwrap();
        let err = validate_appimage(&path).await.unwrap_err();
        assert!(err.to_string().contains("Invalid AppImage magic bytes"));
    }

    #[tokio::test]
    async fn rejects_file_with_wrong_appimage_signature() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("notai.AppImage");
        let mut header = valid_appimage_header();
        header[8] = 0x00; // AI signature quebrada
        tokio::fs::write(&path, header).await.unwrap();
        let err = validate_appimage(&path).await.unwrap_err();
        assert!(err.to_string().contains("Invalid AppImage magic bytes"));
    }

    #[tokio::test]
    async fn rejects_file_too_short() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tiny.AppImage");
        tokio::fs::write(&path, b"\x7fELF").await.unwrap(); // apenas 4 bytes
        let err = validate_appimage(&path).await.unwrap_err();
        assert!(err.to_string().contains("Invalid AppImage magic bytes"));
    }

    #[tokio::test]
    async fn rejects_nonexistent_file() {
        let path = std::path::Path::new("/tmp/aura_test_nao_existe.AppImage");
        let err = validate_appimage(path).await.unwrap_err();
        assert!(err.to_string().contains("File not found"));
    }
}
