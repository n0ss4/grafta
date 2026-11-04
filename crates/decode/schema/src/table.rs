//! A FigJam table's rows and columns: entries keyed by an order position, and
//! the sizes recorded for some of them.

use std::collections::HashMap;

use crate::{Field, Guid, Wire};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableAxis {
    Rows,
    Columns,
}

impl TableAxis {
    #[must_use]
    pub const fn positions(self) -> Field {
        match self {
            Self::Rows => Field::TableRowPositions,
            Self::Columns => Field::TableColumnPositions,
        }
    }

    #[must_use]
    pub const fn sizes(self) -> Field {
        match self {
            Self::Rows => Field::TableRowHeights,
            Self::Columns => Field::TableColumnWidths,
        }
    }
}

/// The axis's entries in order: each one's position and id.
pub fn table_entries<W: Wire + ?Sized>(table: &W, axis: TableAxis) -> Vec<(String, Guid)> {
    let mut entries: Vec<(String, Guid)> = table
        .field(axis.positions())
        .map(|list| list.array(Field::Entries))
        .unwrap_or_default()
        .iter()
        .filter_map(|e| Some((e.str(Field::Position)?.to_string(), Guid::read(e.field(Field::Id)?)?)))
        .collect();
    entries.sort();
    entries
}

/// The sizes the table records, by entry; an entry with none shares what is left.
pub fn table_sizes<W: Wire + ?Sized>(table: &W, axis: TableAxis) -> HashMap<Guid, f32> {
    table
        .field(axis.sizes())
        .map(|list| list.array(Field::Entries))
        .unwrap_or_default()
        .iter()
        .filter_map(|e| Some((Guid::read(e.field(Field::Id)?)?, e.f32(Field::Size)?)))
        .collect()
}
