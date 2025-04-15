//! Theme for the Configurator
use cursive::theme::{Theme, BorderStyle};
use once_cell::sync::Lazy;

/// Styling for the configurator - using a Lazy static to avoid const initialization issues
pub static THEME: Lazy<Theme> = Lazy::new(|| Theme {
    shadow: true,
    borders: BorderStyle::Simple,
    palette: cursive::theme::Palette::default(),
});
