use af_core::config::RenderConfig;
use af_core::frame::{AsciiCell, AsciiGrid, FrameBuffer};
use rayon::prelude::*;

/// Process frame in half-block mode (▄ character).
///
/// Each terminal cell covers 2 vertical pixels. The top pixel's color goes
/// to bg, the bottom pixel's color goes to fg, and the character is '▄'.
///
/// # Example
/// ```
/// use af_core::frame::{FrameBuffer, AsciiGrid};
/// use af_core::config::RenderConfig;
/// use af_ascii::halfblock::process_halfblock;
///
/// let frame = FrameBuffer::new(4, 4);
/// let mut grid = AsciiGrid::new(4, 2);
/// let config = RenderConfig::default();
/// process_halfblock(&frame, &config, &mut grid);
/// ```
pub fn process_halfblock(frame: &FrameBuffer, _config: &RenderConfig, grid: &mut AsciiGrid) {
    let pixel_h = u32::from(grid.height) * 2;
    let pixel_w = u32::from(grid.width);

    grid.cells
        .par_chunks_mut(pixel_w as usize)
        .enumerate()
        .for_each(|(cy, row)| {
            for (cx, cell) in row.iter_mut().enumerate() {
                let px = (cx as u32) * frame.width / pixel_w.max(1);
                let py_top = (cy as u32) * 2 * frame.height / pixel_h.max(1);
                let py_bot = ((cy as u32) * 2 + 1) * frame.height / pixel_h.max(1);

                let px = px.min(frame.width.saturating_sub(1));
                let py_top = py_top.min(frame.height.saturating_sub(1));
                let py_bot = py_bot.min(frame.height.saturating_sub(1));

                let (tr, tg, tb, _) = frame.pixel(px, py_top);
                let (br, bg, bb, _) = frame.pixel(px, py_bot);

                *cell = AsciiCell {
                    ch: '▄',
                    fg: (br, bg, bb), // Bottom pixel = fg
                    bg: (tr, tg, tb), // Top pixel = bg
                };
            }
        });
}
