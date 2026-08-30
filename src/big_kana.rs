use std::convert::Infallible;

use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::geometry::{OriginDimensions, Point, Size};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::{Dimensions, Drawable, Pixel};
use embedded_graphics::text::{Baseline, Text};
use mplusfonts::mplus;
use mplusfonts::style::BitmapFontStyle;
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

// Enlarging bitmap pixels further makes the kana dominate large terminals and
// exaggerates their pixel edges. One physically square terminal block per
// source pixel stays readable without looking oversized.
const MAX_BLOCK_ZOOM: u16 = 1;

/// Kana rendered from a bitmap font and scaled to the available terminal area.
pub(crate) struct ResponsiveKana<'a> {
    text: &'a str,
    style: Style,
}

impl<'a> ResponsiveKana<'a> {
    pub(crate) const fn new(text: &'a str, style: Style) -> Self {
        Self { text, style }
    }
}

impl Widget for ResponsiveKana<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() || self.text.is_empty() {
            return;
        }

        let bitmap = rasterize(self.text);
        let mode = RenderMode::largest_that_fits(bitmap.size(), area);

        match mode {
            RenderMode::Blocks { zoom } => render_blocks(&bitmap, area, buf, self.style, zoom),
            RenderMode::HalfBlocks => render_half_blocks(&bitmap, area, buf, self.style),
            RenderMode::Braille => render_braille(&bitmap, area, buf, self.style),
            RenderMode::Text => render_text(self.text, area, buf, self.style),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RenderMode {
    /// Each source pixel is a physically square 2x1 terminal-cell block.
    Blocks { zoom: u16 },
    /// Two vertical source pixels share one terminal cell.
    HalfBlocks,
    /// Eight source pixels share one 2x4 braille cell.
    Braille,
    /// Native terminal text for very small viewports.
    Text,
}

impl RenderMode {
    fn largest_that_fits(bitmap: Size, area: Rect) -> Self {
        if bitmap.width == 0 || bitmap.height == 0 {
            return Self::Text;
        }

        let max_zoom = (u32::from(area.width) / (bitmap.width * 2))
            .min(u32::from(area.height) / bitmap.height)
            .min(u32::from(MAX_BLOCK_ZOOM));
        if max_zoom > 0 {
            return Self::Blocks {
                zoom: max_zoom as u16,
            };
        }

        let half_height = bitmap.height.div_ceil(2);
        if bitmap.width <= u32::from(area.width) && half_height <= u32::from(area.height) {
            return Self::HalfBlocks;
        }

        let braille_width = bitmap.width.div_ceil(2);
        let braille_height = bitmap.height.div_ceil(4);
        if braille_width <= u32::from(area.width) && braille_height <= u32::from(area.height) {
            return Self::Braille;
        }

        Self::Text
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Bitmap {
    size: Size,
    pixels: Vec<bool>,
}

impl Bitmap {
    fn new(size: Size) -> Self {
        Self {
            size,
            pixels: vec![false; (size.width * size.height) as usize],
        }
    }

    fn is_on(&self, x: u32, y: u32) -> bool {
        if x >= self.size.width || y >= self.size.height {
            return false;
        }
        self.pixels[(y * self.size.width + x) as usize]
    }

    fn set(&mut self, point: Point, on: bool) {
        let Ok(x) = u32::try_from(point.x) else {
            return;
        };
        let Ok(y) = u32::try_from(point.y) else {
            return;
        };
        if x < self.size.width && y < self.size.height {
            self.pixels[(y * self.size.width + x) as usize] = on;
        }
    }

    fn trimmed(&self) -> Self {
        let mut left = self.size.width;
        let mut top = self.size.height;
        let mut right = 0;
        let mut bottom = 0;
        let mut found = false;

        for y in 0..self.size.height {
            for x in 0..self.size.width {
                if self.is_on(x, y) {
                    found = true;
                    left = left.min(x);
                    top = top.min(y);
                    right = right.max(x);
                    bottom = bottom.max(y);
                }
            }
        }

        if !found {
            return self.clone();
        }

        let size = Size::new(right - left + 1, bottom - top + 1);
        let mut trimmed = Self::new(size);
        for y in 0..size.height {
            for x in 0..size.width {
                if self.is_on(left + x, top + y) {
                    trimmed.pixels[(y * size.width + x) as usize] = true;
                }
            }
        }
        trimmed
    }
}

impl OriginDimensions for Bitmap {
    fn size(&self) -> Size {
        self.size
    }
}

impl DrawTarget for Bitmap {
    type Color = BinaryColor;
    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(point, color) in pixels {
            self.set(point, color.is_on());
        }
        Ok(())
    }
}

fn rasterize(text: &str) -> Bitmap {
    let font = mplus!(
        1,
        BOLD,
        16,
        true,
        1,
        1,
        ["あいうえおかきくけこさしすせそたちつてとなにぬねのはひふへほまみむめもやゆよらりるれろわをんアイウエオカキクケコサシスセソタチツテトナニヌネノハヒフヘホマミムメモヤユヨラリルレロワヲン"]
    );
    let style = BitmapFontStyle::new(&font, BinaryColor::On);
    let probe = Text::with_baseline(text, Point::zero(), style.clone(), Baseline::Top);
    let bounds = probe.bounding_box();
    let mut bitmap = Bitmap::new(bounds.size);
    let origin = Point::new(-bounds.top_left.x, -bounds.top_left.y);
    Text::with_baseline(text, origin, style, Baseline::Top)
        .draw(&mut bitmap)
        .expect("bitmap draw target is infallible");
    bitmap.trimmed()
}

fn centered_origin(area: Rect, width: u32, height: u32) -> (u16, u16) {
    let width = u16::try_from(width).unwrap_or(u16::MAX).min(area.width);
    let height = u16::try_from(height).unwrap_or(u16::MAX).min(area.height);
    (
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
    )
}

fn set_symbol(buf: &mut Buffer, x: u16, y: u16, symbol: char, style: Style) {
    buf[(x, y)].set_char(symbol).set_style(style);
}

fn render_blocks(bitmap: &Bitmap, area: Rect, buf: &mut Buffer, style: Style, zoom: u16) {
    let pixel_width = zoom * 2;
    let render_width = bitmap.size.width * u32::from(pixel_width);
    let render_height = bitmap.size.height * u32::from(zoom);
    let (origin_x, origin_y) = centered_origin(area, render_width, render_height);

    for source_y in 0..bitmap.size.height {
        for source_x in 0..bitmap.size.width {
            if !bitmap.is_on(source_x, source_y) {
                continue;
            }
            for offset_y in 0..zoom {
                for offset_x in 0..pixel_width {
                    let x = origin_x + source_x as u16 * pixel_width + offset_x;
                    let y = origin_y + source_y as u16 * zoom + offset_y;
                    set_symbol(buf, x, y, '█', style);
                }
            }
        }
    }
}

fn render_half_blocks(bitmap: &Bitmap, area: Rect, buf: &mut Buffer, style: Style) {
    let render_width = bitmap.size.width;
    let render_height = bitmap.size.height.div_ceil(2);
    let (origin_x, origin_y) = centered_origin(area, render_width, render_height);

    for y in 0..render_height {
        for x in 0..render_width {
            let symbol = match (bitmap.is_on(x, y * 2), bitmap.is_on(x, y * 2 + 1)) {
                (true, true) => '█',
                (true, false) => '▀',
                (false, true) => '▄',
                (false, false) => continue,
            };
            set_symbol(buf, origin_x + x as u16, origin_y + y as u16, symbol, style);
        }
    }
}

fn render_braille(bitmap: &Bitmap, area: Rect, buf: &mut Buffer, style: Style) {
    let render_width = bitmap.size.width.div_ceil(2);
    let render_height = bitmap.size.height.div_ceil(4);
    let (origin_x, origin_y) = centered_origin(area, render_width, render_height);

    for cell_y in 0..render_height {
        for cell_x in 0..render_width {
            let mut pattern = 0_u8;
            for pixel_y in 0..4 {
                for pixel_x in 0..2 {
                    if bitmap.is_on(cell_x * 2 + pixel_x, cell_y * 4 + pixel_y) {
                        pattern |= braille_bit(pixel_x, pixel_y);
                    }
                }
            }
            if pattern == 0 {
                continue;
            }
            let symbol = char::from_u32(0x2800 + u32::from(pattern))
                .expect("every braille pattern is valid Unicode");
            set_symbol(
                buf,
                origin_x + cell_x as u16,
                origin_y + cell_y as u16,
                symbol,
                style,
            );
        }
    }
}

const fn braille_bit(x: u32, y: u32) -> u8 {
    match (x, y) {
        (0, 0) => 1 << 0,
        (0, 1) => 1 << 1,
        (0, 2) => 1 << 2,
        (0, 3) => 1 << 6,
        (1, 0) => 1 << 3,
        (1, 1) => 1 << 4,
        (1, 2) => 1 << 5,
        (1, 3) => 1 << 7,
        _ => 0,
    }
}

fn render_text(text: &str, area: Rect, buf: &mut Buffer, style: Style) {
    let spaced = text
        .chars()
        .map(|character| character.to_string())
        .collect::<Vec<_>>()
        .join("   ");
    let padding = area.height.saturating_sub(1) / 2;
    let mut lines = vec![Line::default(); padding as usize];
    lines.push(Line::from(Span::styled(spaced, style)));
    Paragraph::new(lines)
        .alignment(Alignment::Center)
        .render(area, buf);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rasterizes_hiragana_and_katakana() {
        for glyph in ["あ", "ア", "ん", "ン"] {
            let bitmap = rasterize(glyph);
            assert!(bitmap.size.width > 1, "{glyph} should have width");
            assert!(bitmap.size.height > 1, "{glyph} should have height");
            assert!(
                bitmap.pixels.iter().any(|pixel| *pixel),
                "{glyph} should contain lit pixels"
            );
        }
    }

    #[test]
    fn caps_block_zoom_to_keep_large_terminals_smooth() {
        let bitmap = Size::new(12, 12);
        assert_eq!(
            RenderMode::largest_that_fits(bitmap, Rect::new(0, 0, 72, 36)),
            RenderMode::Blocks { zoom: 1 }
        );
        assert_eq!(
            RenderMode::largest_that_fits(bitmap, Rect::new(0, 0, 48, 24)),
            RenderMode::Blocks { zoom: 1 }
        );
    }

    #[test]
    fn progressively_compacts_for_smaller_areas() {
        let bitmap = Size::new(80, 16);
        assert_eq!(
            RenderMode::largest_that_fits(bitmap, Rect::new(0, 0, 80, 8)),
            RenderMode::HalfBlocks
        );
        assert_eq!(
            RenderMode::largest_that_fits(bitmap, Rect::new(0, 0, 40, 4)),
            RenderMode::Braille
        );
        assert_eq!(
            RenderMode::largest_that_fits(bitmap, Rect::new(0, 0, 20, 2)),
            RenderMode::Text
        );
    }

    #[test]
    fn large_viewport_renders_many_block_cells() {
        let area = Rect::new(0, 0, 80, 24);
        let mut buffer = Buffer::empty(area);
        ResponsiveKana::new("あ", Style::default()).render(area, &mut buffer);

        let blocks = buffer
            .content()
            .iter()
            .filter(|cell| cell.symbol() == "█")
            .count();
        assert!(blocks > 10);
    }
}
