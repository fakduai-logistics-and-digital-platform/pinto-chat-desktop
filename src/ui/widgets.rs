//! Small building blocks shared by the screens: avatar, icons, badges.
//! Ported from `shared/components/App*.vue`.

use std::sync::Arc;

use gpui_kit::assets::IconName;
use gpui_kit::component::Icon;
use gpui_kit::*;

use crate::theme::Palette;

pub const ICON: f32 = 18.;

/// Tabular figures, so times and counts do not jitter as they change.
pub fn tnum() -> FontFeatures {
    FontFeatures(Arc::new(vec![("tnum".into(), 1)]))
}

pub fn icon(name: IconName, size: f32, color: Hsla) -> Icon {
    Icon::new(name).size(px(size)).text_color(color)
}

/// Verified / official account badge icon (`BadgeCheck`).
pub fn verified_badge(size: f32, p: &Palette) -> Icon {
    icon(IconName::BadgeCheck, size, p.primary_ink)
}

/// Circle avatar: the image once it has loaded, a quiet person glyph until then.
pub fn avatar(image: Option<Arc<Image>>, size: f32, p: &Palette) -> AnyElement {
    let base = div()
        .size(px(size))
        .flex_none()
        .rounded_full()
        .overflow_hidden()
        .bg(p.surface_muted)
        .flex()
        .items_center()
        .justify_center();
    match image {
        Some(img_data) => base
            // The container does not clip, so the image carries its own radius.
            .child(
                img(img_data)
                    .size(px(size))
                    .rounded_full()
                    .object_fit(ObjectFit::Cover),
            )
            .into_any_element(),
        None => base
            .border_1()
            .border_color(p.hairline)
            .child(icon(IconName::User, (size * 0.5).round(), p.ink_disabled))
            .into_any_element(),
    }
}

/// Unread count in `error-strong`, the one place DESIGN.md allows it as a fill.
pub fn unread_badge(count: i64, p: &Palette) -> impl IntoElement {
    let label = if count > 99 {
        "99+".to_string()
    } else {
        count.to_string()
    };
    div()
        .h(px(20.))
        .min_w(px(20.))
        .px(px(6.))
        .rounded_full()
        .bg(p.error_strong)
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(11.))
        .line_height(px(20.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(white())
        .font_features(tnum())
        .child(label)
}

/// Square icon button used in the rail, headers and composer.
pub fn icon_button(
    id: impl Into<ElementId>,
    name: IconName,
    size: f32,
    color: Hsla,
    p: &Palette,
) -> Stateful<Div> {
    let hover = p.surface_muted;
    div()
        .id(id)
        .size(px(size))
        .flex_none()
        .rounded(px(10.))
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .hover(move |s| s.bg(hover))
        .child(icon(name, ICON, color))
}

/// Lets an empty strip move the window, since the title bar is transparent.
pub fn drag_region(el: Div) -> Div {
    el.window_control_area(WindowControlArea::Drag)
        .on_mouse_down(MouseButton::Left, |ev, window, _| {
            if ev.click_count == 2 {
                window.titlebar_double_click();
            } else {
                window.start_window_move();
            }
        })
}
