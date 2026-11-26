use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::draw;
use crate::layout::{MIN_HEIGHT, MIN_WIDTH};
use crate::theme;

pub const MESSAGE: &str = "Agrandis ta fenêtre (80×24 minimum)";

pub fn render(buf: &mut Buffer, area: Rect, now: f64) {
    draw::clear(buf, area);
    if area.is_empty() {
        return;
    }
    let pulse = 0.5 + 0.5 * (now * 2.0).sin();
    let msg = draw::truncate(MESSAGE, area.width as usize);
    let w = msg.chars().count() as u16;
    let y = area.y + area.height / 2;
    let x = area.x + area.width.saturating_sub(w) / 2;
    draw::text(
        buf,
        x,
        y,
        &msg,
        theme::bold(theme::GOLD.lerp(theme::PINK, pulse)),
        area.right(),
    );
    if area.height > 2 {
        let size = format!(
            "actuellement {}×{} · minimum {}×{}",
            area.width, area.height, MIN_WIDTH, MIN_HEIGHT
        );
        let size = draw::truncate(&size, area.width as usize);
        let w = size.chars().count() as u16;
        let x = area.x + area.width.saturating_sub(w) / 2;
        draw::text(buf, x, y + 1, &size, theme::fg(theme::GREY), area.right());
    }
}
