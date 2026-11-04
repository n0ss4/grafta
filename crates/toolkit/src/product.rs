//! The one list of the products, each its own crate: Design first, the
//! product of a new file. Their traits are the domain's.

pub use domain::product::*;

pub use ::{design, figjam, sites, slides};

/// Every product, Design first.
pub static PRODUCTS: &[&dyn ProductProfile] = &[&design::Design, &slides::Slides, &figjam::FigJam, &sites::Sites];

/// Every product, as the toolkit is given them.
#[must_use]
pub fn all() -> Products {
    Products::new(PRODUCTS)
}

#[cfg(test)]
mod tests;
