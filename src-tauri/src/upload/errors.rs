#[derive(Debug, thiserror::Error)]
pub enum UploadError {
    #[error("configuration invalid: {0}")]
    ConfigInvalid(String),
    #[error("ssh or scp is missing")]
    BinariesMissing,
    #[error("clipboard is empty or unsupported")]
    ClipboardEmpty,
    #[error("clipboard contains a directory which is not supported")]
    ClipboardDirectory,
    #[error("mkdir failed: {0}")]
    MkdirFailed(String),
    #[error("upload failed: {stderr} (partial at {part_path})")]
    ScpFailed { stderr: String, part_path: String },
    #[error("destination already exists; partial kept at {0}")]
    FinalExists(String),
    #[error("already in progress")]
    InProgress,
    #[error("clipboard write-back failed")]
    ClipboardWrite,
    #[error("local path is not valid UTF-8: {0}")]
    LocalPathInvalid(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}
