#![deny(clippy::all)]

//! Figma Design: the base every other product extends.

use domain::product::ProductProfile;

pub struct Design;

impl ProductProfile for Design {
    fn product(&self) -> figfile::Product {
        figfile::Product::Design
    }
    fn name(&self) -> &'static str {
        "Figma Design"
    }
    fn key(&self) -> &'static str {
        "design"
    }
}
