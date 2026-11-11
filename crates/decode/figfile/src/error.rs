use thiserror::Error;

#[derive(Debug, Error)]
pub enum FigError {
    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("no canvas data found in .fig file")]
    NoCanvas,
    #[error("invalid fig-kiwi magic")]
    InvalidMagic,
    #[error("truncated fig-kiwi container")]
    Truncated,
    #[error("not enough chunks in fig-kiwi container")]
    NotEnoughChunks,
    #[error("kiwi error: {0}")]
    Kiwi(#[from] kiwi::KiwiError),
    #[error("zstd error: {0}")]
    Zstd(String),
    #[error("deflate error: {0}")]
    Deflate(String),
    #[error("{what} inflates past {limit} bytes, the most grafta reads")]
    TooLarge { what: &'static str, limit: u64 },
    #[error("a .fig file is not written in a browser: install grafta to save one")]
    NotWrittenHere,
}
