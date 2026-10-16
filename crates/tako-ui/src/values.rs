//! Conversão das structs da interface para `slint_interpreter::Value` (a API de tema é dinâmica).

use slint::{Color, Image, ModelRc, SharedString, VecModel};
use slint_interpreter::{Struct, Value};

pub trait ToValue {
    fn to_value(&self) -> Value;
}

impl ToValue for str {
    fn to_value(&self) -> Value {
        Value::String(SharedString::from(self))
    }
}
impl ToValue for String {
    fn to_value(&self) -> Value {
        Value::String(SharedString::from(self.as_str()))
    }
}
impl ToValue for bool {
    fn to_value(&self) -> Value {
        Value::Bool(*self)
    }
}
impl ToValue for i32 {
    fn to_value(&self) -> Value {
        Value::Number(*self as f64)
    }
}
impl ToValue for usize {
    fn to_value(&self) -> Value {
        Value::Number(*self as f64)
    }
}
impl ToValue for f32 {
    fn to_value(&self) -> Value {
        Value::Number(*self as f64)
    }
}
impl ToValue for Image {
    fn to_value(&self) -> Value {
        Value::Image(self.clone())
    }
}
impl ToValue for Color {
    fn to_value(&self) -> Value {
        Value::Brush(slint::Brush::SolidColor(*self))
    }
}
impl<T: ToValue> ToValue for Vec<T> {
    fn to_value(&self) -> Value {
        model(self.iter().map(ToValue::to_value).collect())
    }
}
impl<T: ToValue + ?Sized> ToValue for &T {
    fn to_value(&self) -> Value {
        (**self).to_value()
    }
}

pub fn model(values: Vec<Value>) -> Value {
    Value::Model(ModelRc::new(VecModel::from(values)))
}

/// Monta uma struct do tema: `obj(&[("title", t.to_value()), ...])`.
pub fn obj(fields: &[(&str, Value)]) -> Value {
    let mut s = Struct::default();
    for (k, v) in fields {
        s.set_field((*k).to_owned(), v.clone());
    }
    Value::Struct(s)
}

/// `#RRGGBB` (ou `#AARRGGBB`) para cor; inválido → cor de reserva do tema.
pub fn parse_color(hex: &str) -> Color {
    let h = hex.trim().trim_start_matches('#');
    let n = u32::from_str_radix(h, 16).unwrap_or(0x2B3A55);
    match h.len() {
        8 => Color::from_argb_encoded(n),
        _ => Color::from_rgb_u8((n >> 16) as u8, (n >> 8) as u8, n as u8),
    }
}

/// Declara uma struct da API de tema e implementa `ToValue` campo a campo.
/// Os nomes dos campos em Rust usam `_`; no tema, `-`.
#[macro_export]
macro_rules! theme_struct {
    ($(#[$m:meta])* pub struct $name:ident { $(pub $field:ident : $ty:ty),* $(,)? }) => {
        $(#[$m])*
        #[derive(Clone, Default)]
        pub struct $name { $(pub $field: $ty),* }
        impl $crate::values::ToValue for $name {
            fn to_value(&self) -> slint_interpreter::Value {
                let mut s = slint_interpreter::Struct::default();
                $( s.set_field(stringify!($field).replace('_', "-"), $crate::values::ToValue::to_value(&self.$field)); )*
                slint_interpreter::Value::Struct(s)
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors() {
        assert_eq!(parse_color("#C8102E"), Color::from_rgb_u8(0xC8, 0x10, 0x2E));
        assert_eq!(parse_color("lixo"), Color::from_rgb_u8(0x2B, 0x3A, 0x55));
        assert_eq!(parse_color("#80FFFFFF").alpha(), 0x80);
    }
}
