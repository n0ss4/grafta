//! The layers FigJam generates for a sticky, a shape with text or a table,
//! by the local id each has in `nodeGenerationData`.

use crate::Guid;

/// The session FigJam generates its layers in.
pub const GENERATED_SESSION: u32 = 40_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeneratedLayer {
    Background = 0,
    Text = 1,
    Author = 2,
}

impl GeneratedLayer {
    const ALL: [Self; 3] = [Self::Background, Self::Text, Self::Author];

    #[must_use]
    pub const fn local_id(self) -> u32 {
        self as u32
    }

    /// The layer's id in the generated session.
    #[must_use]
    pub const fn guid(self) -> Guid {
        Guid::new(GENERATED_SESSION, self.local_id())
    }

    #[must_use]
    pub fn of(local_id: u32) -> Option<Self> {
        Self::ALL.into_iter().find(|layer| layer.local_id() == local_id)
    }
}
