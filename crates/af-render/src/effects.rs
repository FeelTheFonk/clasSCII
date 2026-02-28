use af_core::color::{hsv_to_rgb, rgb_to_hsv};
use af_core::frame::{AsciiCell, AsciiGrid};

/// Post-processing effects on AsciiGrid before rendering.

/// Apply strobe: boost fg brightness proportional to onset envelope.
///
/// Replaces the old `apply_beat_flash` with a continuous envelope-driven effect.
/// `envelope` [0.0, 1.0] — onset envelope (1.0 on beat, decays via strobe_decay).
/// `intensity` [0.0, 2.0] — strength multiplier.
pub fn apply_strobe(grid: &mut AsciiGrid, envelope: f32, intensity: f32) {
    if envelope < 0.001 || intensity < 0.001 {
        return;
    }

    let boost = (envelope * intensity * 128.0).min(255.0) as u8;
    if boost == 0 {
        return;
    }

    for cy in 0..grid.height {
        for cx in 0..grid.width {
            let cell = grid.get(cx, cy);
            let fg = (
                cell.fg.0.saturating_add(boost),
                cell.fg.1.saturating_add(boost),
                cell.fg.2.saturating_add(boost),
            );
            grid.set(
                cx,
                cy,
                AsciiCell {
                    ch: cell.ch,
                    fg,
                    bg: cell.bg,
                },
            );
        }
    }
}

/// Apply fade trails: blend current grid with previous grid.
///
/// `decay` [0.0, 1.0]: 0 = no trail, 1 = full persistence.
pub fn apply_fade_trails(current: &mut AsciiGrid, previous: &AsciiGrid, decay: f32) {
    if decay < 0.01 || current.width != previous.width || current.height != previous.height {
        return;
    }

    let d = decay.clamp(0.0, 0.95);
    let keep = 1.0 - d;

    for cy in 0..current.height {
        for cx in 0..current.width {
            let cur = current.get(cx, cy);
            let prev = previous.get(cx, cy);

            // If current cell is blank but previous wasn't, blend
            if cur.ch == ' ' && prev.ch != ' ' {
                let fg = (
                    (f32::from(prev.fg.0) * d) as u8,
                    (f32::from(prev.fg.1) * d) as u8,
                    (f32::from(prev.fg.2) * d) as u8,
                );
                current.set(
                    cx,
                    cy,
                    AsciiCell {
                        ch: prev.ch,
                        fg,
                        bg: cur.bg,
                    },
                );
            } else if cur.ch != ' ' {
                // Blend current with echo of previous
                let fg = (
                    (f32::from(cur.fg.0) * keep + f32::from(prev.fg.0) * d) as u8,
                    (f32::from(cur.fg.1) * keep + f32::from(prev.fg.1) * d) as u8,
                    (f32::from(cur.fg.2) * keep + f32::from(prev.fg.2) * d) as u8,
                );
                current.set(
                    cx,
                    cy,
                    AsciiCell {
                        ch: cur.ch,
                        fg,
                        bg: cur.bg,
                    },
                );
            }
        }
    }
}

/// Apply glow: brighten fg of cells adjacent to bright cells.
///
/// `brightness_buf` must be a pre-allocated buffer of at least `width * height` elements.
/// The caller is responsible for ensuring correct size; this function will resize if needed.
pub fn apply_glow(grid: &mut AsciiGrid, intensity: f32, brightness_buf: &mut Vec<u8>) {
    if intensity < 0.01 {
        return;
    }

    let w = grid.width;
    let h = grid.height;
    let needed = usize::from(w) * usize::from(h);

    // Resize only if dimensions changed (rare — terminal resize only)
    brightness_buf.resize(needed, 0);

    // Read-only pass: fill brightness map
    for y in 0..h {
        for x in 0..w {
            let c = grid.get(x, y);
            brightness_buf[usize::from(y) * usize::from(w) + usize::from(x)] =
                c.fg.0.max(c.fg.1).max(c.fg.2);
        }
    }

    let glow_factor = (intensity * 40.0).min(255.0) as u8;

    for cy in 1..h.saturating_sub(1) {
        for cx in 1..w.saturating_sub(1) {
            let idx = |x: u16, y: u16| usize::from(y) * usize::from(w) + usize::from(x);
            // 4-cardinal neighbors only (skip diagonals for ~50% fewer lookups)
            let max_neighbor = brightness_buf[idx(cx - 1, cy)]
                .max(brightness_buf[idx(cx + 1, cy)])
                .max(brightness_buf[idx(cx, cy - 1)])
                .max(brightness_buf[idx(cx, cy + 1)]);

            if max_neighbor > 140 {
                let cell = grid.get(cx, cy);
                let fg = (
                    cell.fg.0.saturating_add(glow_factor),
                    cell.fg.1.saturating_add(glow_factor),
                    cell.fg.2.saturating_add(glow_factor),
                );
                grid.set(
                    cx,
                    cy,
                    AsciiCell {
                        ch: cell.ch,
                        fg,
                        bg: cell.bg,
                    },
                );
            }
        }
    }
}

/// Apply chromatic aberration: shift R channel left, B channel right.
///
/// `offset` [0.0, 5.0] — pixel offset for R/B channels.
/// `fg_buf` — pre-allocated buffer, resized internally if needed.
pub fn apply_chromatic_aberration(
    grid: &mut AsciiGrid,
    offset: f32,
    fg_buf: &mut Vec<(u8, u8, u8)>,
) {
    if offset < 0.01 {
        return;
    }

    let w = usize::from(grid.width);
    let h = usize::from(grid.height);
    let needed = w * h;

    fg_buf.resize(needed, (0, 0, 0));

    // Read pass: copy all fg colors
    for y in 0..grid.height {
        for x in 0..grid.width {
            let cell = grid.get(x, y);
            fg_buf[usize::from(y) * w + usize::from(x)] = cell.fg;
        }
    }

    let shift = offset.ceil() as i32;
    #[allow(clippy::cast_possible_wrap)] // w derived from u16, always fits i32
    let w_i32 = w as i32;

    // Write pass: shift R left, B right, G stays centered
    for y in 0..grid.height {
        for x in 0..grid.width {
            let xi = i32::from(x);
            let yi = usize::from(y);
            let r_x = (xi - shift).clamp(0, w_i32 - 1) as usize;
            let b_x = (xi + shift).clamp(0, w_i32 - 1) as usize;
            let g_x = usize::from(x);

            let r = fg_buf[yi * w + r_x].0;
            let g = fg_buf[yi * w + g_x].1;
            let b = fg_buf[yi * w + b_x].2;

            let cell = grid.get(x, y);
            grid.set(
                x,
                y,
                AsciiCell {
                    ch: cell.ch,
                    fg: (r, g, b),
                    bg: cell.bg,
                },
            );
        }
    }
}

/// Apply wave distortion: horizontally shift rows with a smooth sinusoidal pattern.
///
/// `amplitude` [0.0, 1.0] — wave strength (max shift = amplitude * 8 cells).
/// `speed` — spatial frequency multiplier (waves per grid height).
/// `phase` — temporal phase offset (driven by persistent wave_phase + audio beat_phase).
/// `row_buf` — pre-allocated buffer, resized internally if needed.
pub fn apply_wave_distortion(
    grid: &mut AsciiGrid,
    amplitude: f32,
    speed: f32,
    phase: f32,
    row_buf: &mut Vec<AsciiCell>,
) {
    // Cap max shift to 8 cells (not grid width) for smooth, non-jarring motion
    const MAX_WAVE_SHIFT: f32 = 8.0;

    if amplitude < 0.001 {
        return;
    }

    let w = grid.width;
    let h = grid.height;
    let hf = f32::from(h);

    row_buf.resize(usize::from(w), AsciiCell::default());

    for y in 0..h {
        let yf = f32::from(y);
        let shift = (amplitude
            * MAX_WAVE_SHIFT
            * (std::f32::consts::TAU * speed * yf / hf + phase).sin()) as i16;

        // Copy row to buffer
        for x in 0..w {
            row_buf[usize::from(x)] = *grid.get(x, y);
        }

        // Write shifted with wrapping (no blank gaps)
        let w_i32 = i32::from(w);
        for x in 0..w {
            let src_x = ((i32::from(x) - i32::from(shift)) % w_i32 + w_i32) % w_i32;
            grid.set(x, y, row_buf[src_x as usize]);
        }
    }
}

/// Apply color pulse: rotate hue of all fg colors.
///
/// `hue_shift` [0.0, 1.0) — amount to rotate (wraps).
pub fn apply_color_pulse(grid: &mut AsciiGrid, hue_shift: f32) {
    if hue_shift.abs() < 0.001 {
        return;
    }

    for cy in 0..grid.height {
        for cx in 0..grid.width {
            let cell = grid.get(cx, cy);
            if cell.ch == ' ' {
                continue;
            }
            // Skip black cells — no hue to rotate, saves HSV conversion
            if cell.fg.0 == 0 && cell.fg.1 == 0 && cell.fg.2 == 0 {
                continue;
            }

            let (h, s, v) = rgb_to_hsv(cell.fg.0, cell.fg.1, cell.fg.2);
            let new_h = (h + hue_shift) % 1.0;
            let fg = hsv_to_rgb(new_h, s, v);

            grid.set(
                cx,
                cy,
                AsciiCell {
                    ch: cell.ch,
                    fg,
                    bg: cell.bg,
                },
            );
        }
    }
}

/// Apply temporal stability: suppress minor character flickering between frames.
///
/// Compares current and previous grid. If a character change is "minor"
/// (both chars have similar visual density), keep the previous character
/// to reduce perceived flicker.
///
/// `threshold` [0.0, 1.0] — 0 = off, higher = more aggressive stabilization.
pub fn apply_temporal_stability(current: &mut AsciiGrid, previous: &AsciiGrid, threshold: f32) {
    if threshold < 0.001 || current.width != previous.width || current.height != previous.height {
        return;
    }

    let t = threshold * 0.3;

    for cy in 0..current.height {
        for cx in 0..current.width {
            let cur = current.get(cx, cy);
            let prev = previous.get(cx, cy);

            // Skip if either is blank
            if cur.ch == ' ' || prev.ch == ' ' {
                continue;
            }

            // Estimate "density" from fg brightness as proxy
            let cur_density = char_density(cur.ch);
            let prev_density = char_density(prev.ch);

            if (cur_density - prev_density).abs() < t {
                // Minor change — keep previous char to reduce flicker
                grid_set_ch(current, cx, cy, prev.ch);
            }
        }
    }
}

/// Estimate character visual density [0.0, 1.0].
/// Uses Unicode block coverage heuristic.
#[inline]
fn char_density(ch: char) -> f32 {
    match ch {
        ' ' => 0.0,
        '.' | ',' | '\'' | '`' | ':' => 0.1,
        '-' | '_' | '~' => 0.15,
        ';' | '!' | '|' | '/' | '\\' => 0.2,
        '+' | '*' | '^' | '"' | 'i' | 'l' | 't' | 'r' | 'c' => 0.3,
        '=' | '(' | ')' | '{' | '}' | '[' | ']' => 0.35,
        'v' | 'x' | 'z' | 'n' | 'u' | 'o' | 'a' | 'e' | 's' => 0.4,
        'A'..='Z' => 0.55,
        '#' | '@' | '%' | '&' | '$' => 0.7,
        '\u{2588}' => 1.0,               // Full block
        '\u{2596}'..='\u{259F}' => 0.25, // Quadrants
        '\u{2800}'..='\u{28FF}' => {
            // Braille: count dots
            let dots = (ch as u32 - 0x2800).count_ones();
            dots as f32 / 8.0
        }
        _ => 0.5,
    }
}

/// Set only the character of a cell (preserving fg/bg).
#[inline]
fn grid_set_ch(grid: &mut AsciiGrid, x: u16, y: u16, ch: char) {
    let cell = grid.get(x, y);
    grid.set(
        x,
        y,
        AsciiCell {
            ch,
            fg: cell.fg,
            bg: cell.bg,
        },
    );
}

/// Apply scan lines: darken every Nth row.
///
/// `gap` — line spacing (0 = disabled, 2-8 typical).
/// `darken_factor` — brightness multiplier for affected lines [0.0, 1.0].
pub fn apply_scan_lines(grid: &mut AsciiGrid, gap: u8, darken_factor: f32) {
    if gap == 0 {
        return;
    }

    let factor = darken_factor.clamp(0.0, 1.0);

    for cy in 0..grid.height {
        if cy % u16::from(gap) != 0 {
            continue;
        }
        for cx in 0..grid.width {
            let cell = grid.get(cx, cy);
            let fg = (
                (f32::from(cell.fg.0) * factor) as u8,
                (f32::from(cell.fg.1) * factor) as u8,
                (f32::from(cell.fg.2) * factor) as u8,
            );
            grid.set(
                cx,
                cy,
                AsciiCell {
                    ch: cell.ch,
                    fg,
                    bg: cell.bg,
                },
            );
        }
    }
}
