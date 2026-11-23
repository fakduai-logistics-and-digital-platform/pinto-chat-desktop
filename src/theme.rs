//! Pinto palette from DESIGN.md, plus a neutral dark ramp, applied to both
//! our own views and gpui-component's theme so stock components match.

use gpui_kit::component::{Theme, ThemeMode as ComponentMode, theme::ThemeTokens};
use gpui_kit::*;

use crate::persist::ThemeMode;

pub const FONT_FAMILY: &str = "IBM Plex Sans Thai";

#[derive(Clone, Copy)]
pub struct Palette {
    pub dark: bool,
    pub primary: Hsla,
    pub primary_hover: Hsla,
    pub primary_pressed: Hsla,
    /// 8% brand wash: selection and active rows.
    pub primary_tint: Hsla,
    pub primary_tint_hover: Hsla,
    /// Brand green dark enough to read as text on light surfaces.
    pub primary_ink: Hsla,
    pub on_primary: Hsla,
    pub background: Hsla,
    pub surface: Hsla,
    pub surface_muted: Hsla,
    pub ink: Hsla,
    pub ink_secondary: Hsla,
    pub ink_subtitle: Hsla,
    pub ink_disabled: Hsla,
    pub ink_hint: Hsla,
    pub divider: Hsla,
    pub input_border: Hsla,
    pub hairline: Hsla,
    pub error: Hsla,
    pub error_strong: Hsla,
    pub error_container: Hsla,
    pub skeleton: Hsla,
    pub bubble_other: Hsla,
    pub overlay: Hsla,
    pub shadow: Hsla,
}

impl Global for Palette {}

fn hex(v: u32) -> Hsla {
    rgb(v).into()
}

fn hexa(v: u32) -> Hsla {
    rgba(v).into()
}

impl Palette {
    pub fn light() -> Self {
        Self {
            dark: false,
            primary: hex(0x2ECC71),
            primary_hover: hex(0x27AE60),
            primary_pressed: hex(0x1E8A4B),
            primary_tint: hex(0xEAFBF1),
            primary_tint_hover: hex(0xD4F7E3),
            primary_ink: hex(0x1E8A4B),
            on_primary: hex(0xFFFFFF),
            background: hex(0xFAFAFA),
            surface: hex(0xFFFFFF),
            surface_muted: hex(0xF9FAFB),
            ink: hex(0x433D4D),
            ink_secondary: hex(0x696373),
            ink_subtitle: hex(0x85818B),
            ink_disabled: hex(0xA8A4AD),
            ink_hint: hex(0xD1CED6),
            divider: hex(0xDFDDE3),
            input_border: hex(0xD1CED6),
            hairline: hex(0xEBEBEB),
            error: hex(0xFF7074),
            error_strong: hex(0xFF4C51),
            error_container: hex(0xFFEDEE),
            skeleton: hex(0xE5E7EB),
            bubble_other: hex(0xFFFFFF),
            overlay: hex(0x433D4D),
            shadow: hexa(0x0000001A),
        }
    }

    /// Graphite ramp with the same green. Depth comes from surface steps.
    pub fn dark() -> Self {
        Self {
            dark: true,
            primary: hex(0x2ECC71),
            primary_hover: hex(0x3DD47E),
            primary_pressed: hex(0x27AE60),
            primary_tint: hex(0x1B3426),
            primary_tint_hover: hex(0x224130),
            primary_ink: hex(0x5CD892),
            on_primary: hex(0x0E2418),
            background: hex(0x141416),
            surface: hex(0x1C1C1F),
            surface_muted: hex(0x242428),
            ink: hex(0xECEAF0),
            ink_secondary: hex(0xBDB9C4),
            ink_subtitle: hex(0x928E99),
            ink_disabled: hex(0x6E6A75),
            ink_hint: hex(0x4E4B55),
            divider: hex(0x2E2E33),
            input_border: hex(0x3A3A40),
            hairline: hex(0x29292E),
            error: hex(0xFF7074),
            error_strong: hex(0xFF5A5F),
            error_container: hex(0x3A1F21),
            skeleton: hex(0x2A2A2F),
            bubble_other: hex(0x26262A),
            overlay: hex(0xECEAF0),
            shadow: hexa(0x00000000),
        }
    }
}

pub trait PaletteExt {
    fn palette(&self) -> Palette;
}

impl PaletteExt for App {
    fn palette(&self) -> Palette {
        *self.global::<Palette>()
    }
}

pub fn resolve_dark(mode: ThemeMode, window: Option<&Window>) -> bool {
    match mode {
        ThemeMode::Light => false,
        ThemeMode::Dark => true,
        ThemeMode::System => window
            .map(|w| {
                matches!(
                    w.appearance(),
                    WindowAppearance::Dark | WindowAppearance::VibrantDark
                )
            })
            .unwrap_or(false),
    }
}

/// Installs the palette and maps it onto gpui-component's theme tokens.
pub fn apply(dark: bool, window: Option<&mut Window>, cx: &mut App) {
    let p = if dark {
        Palette::dark()
    } else {
        Palette::light()
    };
    cx.set_global(p);
    Theme::change(
        if dark {
            ComponentMode::Dark
        } else {
            ComponentMode::Light
        },
        window,
        cx,
    );

    let theme = Theme::global_mut(cx);
    theme.font_family = FONT_FAMILY.into();
    theme.font_size = px(14.);
    theme.radius = px(8.);
    theme.radius_lg = px(12.);
    theme.shadow = !dark;
    // Keep the scrollbar visible whenever a list has more rows than fit.
    theme.scrollbar_mode = gpui_kit::component::scroll::ScrollbarMode::Always;

    let c = &mut theme.colors;
    c.background = p.surface;
    c.foreground = p.ink;
    c.border = p.divider;
    c.input = p.input_border;
    c.ring = p.primary;
    c.caret = p.primary;
    c.selection = p.primary_tint_hover;
    c.muted = p.surface_muted;
    c.muted_foreground = p.ink_subtitle;
    c.accent = p.surface_muted;
    c.accent_foreground = p.ink;
    c.popover = p.surface;
    c.popover_foreground = p.ink;
    c.primary = p.primary;
    c.primary_hover = p.primary_hover;
    c.primary_active = p.primary_pressed;
    c.primary_foreground = p.on_primary;
    c.button_primary = p.primary;
    c.button_primary_hover = p.primary_hover;
    c.button_primary_active = p.primary_pressed;
    c.button_primary_foreground = p.on_primary;
    c.secondary = p.surface_muted;
    c.secondary_hover = p.hairline;
    c.secondary_active = p.divider;
    c.secondary_foreground = p.ink;
    c.danger = p.error_strong;
    c.danger_hover = p.error;
    c.danger_active = p.error_strong;
    c.danger_foreground = hex(0xFFFFFF);
    c.button_danger = p.error_strong;
    c.button_danger_hover = p.error;
    c.button_danger_active = p.error_strong;
    c.button_danger_foreground = hex(0xFFFFFF);
    c.list_hover = p.surface_muted;
    c.list_active = p.primary_tint;
    c.list_active_border = p.primary_tint;
    c.switch = p.ink_disabled;
    c.switch_thumb = hex(0xFFFFFF);
    c.skeleton = p.skeleton;
    c.scrollbar = p.transparent();
    c.scrollbar_thumb = p.ink_hint;
    c.scrollbar_thumb_hover = p.ink_disabled;
    c.link = p.primary_ink;
    c.window_border = p.divider;
    c.title_bar = p.surface;
    c.title_bar_border = p.divider;
    // Components read the token set, which is derived from `colors`.
    theme.tokens = ThemeTokens::from(&theme.colors);
}

impl Palette {
    fn transparent(&self) -> Hsla {
        hsla(0., 0., 0., 0.)
    }
}
