//! GDD 5.9 Steam Deck 版面（1280×800）：16 px 最小字級、器物放大、印匣帶 LB／RB 提示、
//! 卡片改為右側 45% 寬的命簿抽屜。
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::egui::{self, FontFamily, FontId, Id, TextStyle};

use super::ink::{self, Ink};

/// 介面版面（5.9）。
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UiLayout {
    #[default]
    Desktop,
    Deck,
}

/// Deck 印匣高度（印 44 + 邊距）。
pub const DECK_TRAY_H: f32 = 56.0;
/// Deck 內文最小字級（5.9）。
pub const DECK_MIN_TEXT: f32 = 16.0;

/// 視窗解析度 ≤ 1280×800 → Deck 版面（Steam 偵測於 10.2 之後接上）。
pub fn layout_for(width: f32, height: f32) -> UiLayout {
    if width <= 1280.0 && height <= 800.0 {
        UiLayout::Deck
    } else {
        UiLayout::Desktop
    }
}

/// `SIMING_LAYOUT=deck|desktop` 覆寫（測試用；設定畫面的手動切換於日後加入）。
fn env_override() -> Option<UiLayout> {
    match std::env::var("SIMING_LAYOUT").ok()?.to_lowercase().as_str() {
        "deck" => Some(UiLayout::Deck),
        "desktop" => Some(UiLayout::Desktop),
        _ => None,
    }
}

pub fn detect_layout(windows: Query<&Window, With<PrimaryWindow>>, mut layout: ResMut<UiLayout>) {
    let next = env_override().or_else(|| windows.single().ok().map(|w| layout_for(w.width(), w.height())));
    if let Some(l) = next {
        if *layout != l {
            *layout = l;
        }
    }
}

/// Deck 版面把所有文字樣式拉到至少 16 px；桌機版面還原 `theme::apply` 的字級。
pub fn apply_text_sizes(ctx: &egui::Context, layout: UiLayout) {
    ctx.style_mut(|style| {
        let (small, body, heading) = match layout {
            UiLayout::Deck => (DECK_MIN_TEXT, DECK_MIN_TEXT, 22.0),
            UiLayout::Desktop => (12.0, 15.0, 20.0),
        };
        style.text_styles.insert(TextStyle::Small, FontId::new(small, FontFamily::Proportional));
        style.text_styles.insert(TextStyle::Body, FontId::new(body, FontFamily::Proportional));
        style.text_styles.insert(TextStyle::Button, FontId::new(body, FontFamily::Proportional));
        style.text_styles.insert(TextStyle::Heading, FontId::new(heading, FontFamily::Proportional));
        style.text_styles.insert(TextStyle::Monospace, FontId::new(body.max(14.0), FontFamily::Monospace));
        style.spacing.button_padding = if layout == UiLayout::Deck { egui::vec2(10.0, 5.0) } else { egui::vec2(8.0, 3.0) };
        style.spacing.interact_size.y = if layout == UiLayout::Deck { 32.0 } else { 18.0 };
    });
}

/// Deck 版面的命簿：右側抽屜，寬 45%（5.8、5.9）。`t` 為展卷量；`body` 填入內容並回傳是否要求關閉。
pub fn card_drawer(ctx: &egui::Context, ink: &Ink, t: f32, open: bool, body: impl FnOnce(&mut egui::Ui) -> bool) -> bool {
    let width = ctx.screen_rect().width() * 0.45;
    let mut close = false;
    egui::SidePanel::right("card_drawer")
        .exact_width(width)
        .resizable(false)
        .show_separator_line(false)
        .frame(egui::Frame::new().inner_margin(egui::Margin { left: 0, right: 8, top: 10, bottom: 10 }))
        .show(ctx, |ui| {
            if !open {
                ui.disable();
            }
            let inner_max = ui.available_height() - ink.px(14.0) * 2.0 - ink.px(6.0) * 2.0 - ink.px(12.0) * 2.0;
            ink::scroll_body(ui, ink, Id::new("card_drawer_body"), t, ui.available_width(), |ui| {
                egui::ScrollArea::vertical().max_height(inner_max).scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden).show(ui, |ui| {
                    close = body(ui);
                });
            });
        });
    close
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deck_threshold() {
        assert_eq!(layout_for(1280.0, 800.0), UiLayout::Deck);
        assert_eq!(layout_for(1024.0, 600.0), UiLayout::Deck);
        assert_eq!(layout_for(1280.0, 801.0), UiLayout::Desktop);
        assert_eq!(layout_for(1600.0, 900.0), UiLayout::Desktop);
    }
}
