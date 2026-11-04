//! The ports' adapters: the disk's, and memory's for tests. The ports
//! themselves are the domain's.

pub use domain::port::*;

pub use ::{disk, memory};

/// The disk, `figfile`, `render` and JSON, as `grafta mcp` and `grafta tool`
/// use.
#[must_use]
pub fn on_disk() -> Ports {
    Ports {
        store: std::sync::Arc::new(disk::Disk),
        codec: std::sync::Arc::new(disk::FigCodec),
        journal: std::sync::Arc::new(disk::DiskJournal),
        images: std::sync::Arc::new(disk::Disk),
        raster: std::sync::Arc::new(disk::Render),
        measure: std::sync::Arc::new(crate::edge::measure::Json),
        json: std::sync::Arc::new(crate::edge::measure::Text),
    }
}
