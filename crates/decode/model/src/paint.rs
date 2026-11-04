//! Paint (fill / stroke) and effect conversion.

use kiwi::Value;
use schema::{Field, Guid, PaintType, Wire, Wired};

use crate::types::{Color, Effect, EffectType, Fill, FillType, GradientStop, Matrix};

pub(crate) fn color(v: &Value) -> Option<Color> {
    Some(Color {
        r: v.f32(Field::R)?,
        g: v.f32(Field::G)?,
        b: v.f32(Field::B)?,
        a: v.f32(Field::A).unwrap_or(1.0),
    })
}

/// A gradient grafta does not list is drawn as a linear one.
const GRADIENT_PREFIX: &str = "GRADIENT";

/// A paint's type; a paint with none is solid.
fn fill_type(paint: &Value) -> FillType {
    match paint.enum_of::<PaintType>(Field::Type) {
        None | Some(Wired::Known(PaintType::Solid)) => FillType::Solid,
        Some(Wired::Known(PaintType::GradientLinear)) => FillType::GradientLinear,
        Some(Wired::Known(PaintType::GradientRadial)) => FillType::GradientRadial,
        Some(Wired::Known(PaintType::GradientAngular)) => FillType::GradientAngular,
        Some(Wired::Known(PaintType::GradientDiamond)) => FillType::GradientDiamond,
        Some(Wired::Known(PaintType::Image)) => FillType::Image,
        Some(Wired::Known(PaintType::Pattern)) => FillType::Pattern,
        Some(Wired::Known(PaintType::Noise)) => FillType::Noise,
        Some(Wired::Known(_)) => FillType::Unknown,
        Some(Wired::Unknown(other)) if other.starts_with(GRADIENT_PREFIX) => FillType::GradientLinear,
        Some(Wired::Unknown(_)) => FillType::Unknown,
    }
}

fn matrix(v: &Value) -> Matrix {
    let d = Matrix::default();
    Matrix {
        m00: v.f32(Field::M00).unwrap_or(d.m00),
        m01: v.f32(Field::M01).unwrap_or(d.m01),
        m02: v.f32(Field::M02).unwrap_or(d.m02),
        m10: v.f32(Field::M10).unwrap_or(d.m10),
        m11: v.f32(Field::M11).unwrap_or(d.m11),
        m12: v.f32(Field::M12).unwrap_or(d.m12),
    }
}

/// Figma stores image hashes as raw bytes; render them as hex so they match the
/// file names used inside the `.fig` archive.
fn image_hash(v: &Value) -> Option<String> {
    let hash = v.field(Field::Hash)?;
    let bytes: Vec<u8> = match hash {
        Value::Str(s) => return Some(s.clone()),
        Value::Bytes(bytes) => bytes.clone(),
        Value::Array(values) => values.iter().filter_map(|b| Some(b.as_u32()? as u8)).collect(),
        _ => return None,
    };
    Some(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

/// The variable a paint's colour is bound to, if the paint uses one.
fn paint_variable_id(paint: &Value, resolver: &VariableResolver<'_>) -> Option<Guid> {
    let alias = paint.field(Field::ColorVar)?.field(Field::Value)?.field(Field::Alias)?;
    resolver.resolve_alias(alias)
}

/// Resolves variable aliases, which may address a variable either directly by
/// GUID or indirectly by library asset key.
pub struct VariableResolver<'a> {
    /// `key` and `key@version` → node id.
    pub asset_refs: &'a std::collections::HashMap<String, Guid>,
}

impl VariableResolver<'_> {
    #[must_use]
    pub fn resolve_alias(&self, alias: &Value) -> Option<Guid> {
        if let Some(id) = alias.field(Field::Guid).and_then(Guid::read) {
            return Some(id);
        }
        let asset_ref = alias.field(Field::AssetRef)?;
        let key = asset_ref.str(Field::Key)?;
        if let Some(version) = asset_ref.str(Field::Version) {
            if let Some(id) = self.asset_refs.get(&format!("{key}@{version}")) {
                return Some(*id);
            }
        }
        self.asset_refs.get(key).copied()
    }
}

pub(crate) fn fills(paints: &[Value], resolver: &VariableResolver<'_>) -> Vec<Fill> {
    paints
        .iter()
        .map(|p| {
            let ty = fill_type(p);
            let gradient_stops = p
                .array(Field::Stops)
                .iter()
                .filter_map(|s| {
                    Some(GradientStop {
                        color: s.field(Field::Color).and_then(color)?,
                        position: s.f32(Field::Position).unwrap_or(0.0),
                    })
                })
                .collect();
            Fill {
                fill_type: ty,
                color: p.field(Field::Color).and_then(color),
                opacity: p.f32(Field::Opacity).unwrap_or(1.0),
                visible: p.bool(Field::Visible).unwrap_or(true),
                blend_mode: p.str(Field::BlendMode).map(str::to_string),
                gradient_stops,
                gradient_transform: p.field(Field::Transform).map(matrix),
                image_hash: p.field(Field::Image).and_then(image_hash),
                image_scale_mode: p.str(Field::ImageScaleMode).map(str::to_string),
                variable_id: paint_variable_id(p, resolver),
            }
        })
        .collect()
}

fn effect_type(effect: &Value) -> EffectType {
    use schema::EffectType as Kind;
    match effect.known::<Kind>(Field::Type) {
        Some(Kind::DropShadow) => EffectType::DropShadow,
        Some(Kind::InnerShadow) => EffectType::InnerShadow,
        Some(Kind::ForegroundBlur) => EffectType::LayerBlur,
        Some(Kind::BackgroundBlur) => EffectType::BackgroundBlur,
        _ => EffectType::Unknown,
    }
}

pub(crate) fn effects(values: &[Value]) -> Vec<Effect> {
    values
        .iter()
        .map(|e| Effect {
            effect_type: effect_type(e),
            color: e.field(Field::Color).and_then(color),
            offset_x: e.field(Field::Offset).and_then(|o| o.f32(Field::X)).unwrap_or(0.0),
            offset_y: e.field(Field::Offset).and_then(|o| o.f32(Field::Y)).unwrap_or(0.0),
            radius: e.f32(Field::Radius).unwrap_or(0.0),
            spread: e.f32(Field::Spread).unwrap_or(0.0),
            visible: e.bool(Field::Visible).unwrap_or(true),
        })
        .collect()
}

/// CSS `background` value for a paint, resolving gradients into
/// `linear-gradient(…)` and friends. Returns `None` for paints CSS cannot
/// express directly (images, patterns).
#[must_use]
pub fn fill_to_css(fill: &Fill) -> Option<String> {
    if !fill.visible {
        return None;
    }
    if fill.fill_type == FillType::Solid {
        return fill.color.map(|c| c.to_css(fill.opacity));
    }
    if fill.fill_type.is_gradient() && !fill.gradient_stops.is_empty() {
        let stops: Vec<String> = fill
            .gradient_stops
            .iter()
            .map(|s| {
                format!(
                    "{} {}%",
                    s.color.to_css(fill.opacity),
                    crate::types::trim_float(s.position * 100.0)
                )
            })
            .collect();
        let stops = stops.join(", ");
        return Some(match fill.fill_type {
            FillType::GradientRadial | FillType::GradientDiamond => {
                format!("radial-gradient({stops})")
            }
            FillType::GradientAngular => format!("conic-gradient({stops})"),
            _ => format!("linear-gradient({}deg, {stops})", gradient_angle_deg(fill)),
        });
    }
    None
}

/// Approximate a CSS gradient angle from Figma's gradient transform. Figma
/// stores the gradient handles as a matrix mapping the unit square onto the
/// gradient; the first column gives its direction.
fn gradient_angle_deg(fill: &Fill) -> String {
    let Some(t) = fill.gradient_transform else {
        return "180".to_string();
    };
    // Direction vector of the gradient axis in layer space.
    let (dx, dy) = (t.m00, t.m10);
    if dx == 0.0 && dy == 0.0 {
        return "180".to_string();
    }
    // CSS angles run clockwise from "to top"; Figma's y axis points down.
    let deg = dx.atan2(dy).to_degrees();
    crate::types::trim_float(((deg % 360.0) + 360.0) % 360.0)
}
