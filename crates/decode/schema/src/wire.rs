//! A closed set of names the format spells one way.

/// A name of the format: a field, or a value of one of its enums.
pub trait WireName: Copy + Eq + 'static {
    const ALL: &'static [Self];

    fn as_wire(self) -> &'static str;

    /// `None` for a name the schema has but grafta does not list.
    #[must_use]
    fn from_wire(wire: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|name| name.as_wire() == wire)
    }

    /// The one of `allowed` spelt `wire`.
    #[must_use]
    fn listed(wire: &str, allowed: &[Self]) -> Option<Self> {
        allowed.iter().copied().find(|name| name.as_wire() == wire)
    }

    /// `allowed` as the format spells them, joined by `separator`.
    #[must_use]
    fn joined(allowed: &[Self], separator: &str) -> String {
        allowed.iter().map(|name| name.as_wire()).collect::<Vec<_>>().join(separator)
    }
}

/// Declares an enum of the format's names, each variant with its spelling.
#[macro_export]
macro_rules! wire {
    ($(#[$meta:meta])* $vis:vis enum $name:ident { $($variant:ident = $wire:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        $vis enum $name {
            $($variant),+
        }

        impl $crate::WireName for $name {
            const ALL: &'static [Self] = &[$(Self::$variant),+];

            fn as_wire(self) -> &'static str {
                match self {
                    $(Self::$variant => $wire),+
                }
            }
        }
    };
}

/// An enum of the format's values, written into a message as itself.
macro_rules! wire_value {
    ($($name:ident),+ $(,)?) => {
        $(
            impl From<$name> for kiwi::Value {
                fn from(value: $name) -> Self {
                    kiwi::Value::Enum($crate::WireName::as_wire(value))
                }
            }
        )+
    };
}
pub(crate) use wire_value;

#[cfg(test)]
mod tests;
