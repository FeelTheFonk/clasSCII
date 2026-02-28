//! Algorithmique de Tramage Ordonné (Ordered Dithering)
//! Déploiement des matrices de Bayer et Blue Noise pour l'élargissement d'histogramme sans banding.

use af_core::config::DitherMode;

/// Matrice de Bayer 2x2. Normalisée sur 4 niveaux (0-3).
pub const BAYER_2X2: [[u8; 2]; 2] = [[0, 2], [3, 1]];

/// Matrice de Bayer 4x4. Normalisée sur 16 niveaux (0-15).
pub const BAYER_4X4: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];

/// Matrice de Bayer 8x8. Normalisée sur 64 niveaux (0-63).
pub const BAYER_8X8: [[u8; 8]; 8] = [
    [0, 32, 8, 40, 2, 34, 10, 42],
    [48, 16, 56, 24, 50, 18, 58, 26],
    [12, 44, 4, 36, 14, 46, 6, 38],
    [60, 28, 52, 20, 62, 30, 54, 22],
    [3, 35, 11, 43, 1, 33, 9, 41],
    [51, 19, 59, 27, 49, 17, 57, 25],
    [15, 47, 7, 39, 13, 45, 5, 37],
    [63, 31, 55, 23, 61, 29, 53, 21],
];

/// Blue Noise 16×16 matrix (256 values, 0-255).
/// Pre-computed via void-and-cluster algorithm.
/// Provides perceptually superior dithering compared to Bayer patterns:
/// less visible regularity, better handling of subtle gradients.
#[rustfmt::skip]
pub const BLUE_NOISE_16: [[u8; 16]; 16] = [
    [147,  33, 201,  89, 163,  52, 237, 118,  10, 175, 210,  67, 139,  25, 195, 104],
    [ 72, 224, 112,   5, 215, 134,  28,  81, 198, 135,  42, 248, 103,  56, 230, 157],
    [189,  48, 170, 143,  62, 186,  97, 247, 155,  61, 116, 187,  15, 172, 131,  38],
    [  7, 126, 241,  83, 235, 108,  44, 162,  22,  88, 228, 151,  75, 217,  91,  79],
    [203,  95,  30, 178, 148,  19, 220, 129,  68, 254, 176,  36, 196, 120, 243, 165],
    [152, 218, 139,  55,  70, 194, 255, 100,  43, 145, 110,  59, 138, 180,  11,  50],
    [  2, 115,  77, 250, 167, 117,  14,  84, 208, 192,   9, 222, 251,  93, 205, 111],
    [184, 233,  22, 207, 101,  35, 169, 238, 160,  74, 127, 173,  27,  65, 142, 239],
    [ 58, 156, 132,  45, 229, 144,  60, 107, 124,  46, 245, 102,  85, 221, 164,  20],
    [106, 211,  87, 188, 174,  78, 202, 234,   1,  53, 182, 149, 199, 125,  40, 183],
    [  8,  39, 252, 121,  12,  24, 141, 158,  96, 213, 133,  31,  70, 240, 154,  73],
    [146, 168,  64, 200, 246,  92, 223, 113,  37, 171, 119,  16, 191, 102,  54, 219],
    [226, 122,  29, 162,  41, 180,  49,  80, 244, 253,  76, 236, 159,  82, 209, 128],
    [ 86, 193,  99, 136,  69, 216, 130,  17, 105, 150,  57, 204, 140,  37, 181,   4],
    [  3,  47, 231, 242,  21, 108, 190,  63, 179, 214,  23,  94, 123, 249, 116,  66],
    [166, 114, 153,  57, 197, 155, 227, 137, 248,  87,  32, 177, 212,  13, 145, 232],
];

/// Applique le tramage (Bayer 8x8) à une valeur de luminance brute [0..255].
#[must_use]
#[inline(always)]
pub fn apply_bayer_8x8(lum: u8, x: u32, y: u32, levels: f32) -> u8 {
    if !(2..=253).contains(&lum) {
        return lum;
    }

    let bayer_val = f32::from(BAYER_8X8[(y % 8) as usize][(x % 8) as usize]);
    let threshold = (bayer_val / 64.0) - 0.5;
    let base_val = f32::from(lum) / 255.0;
    let dithered_val = (base_val + threshold * (1.0 / levels.max(2.0))).clamp(0.0, 1.0);
    (dithered_val * 255.0).round() as u8
}

/// Applique le tramage Blue Noise 16×16 à une valeur de luminance brute [0..255].
#[must_use]
#[inline(always)]
pub fn apply_blue_noise_16(lum: u8, x: u32, y: u32, levels: f32) -> u8 {
    if !(2..=253).contains(&lum) {
        return lum;
    }

    let noise_val = f32::from(BLUE_NOISE_16[(y % 16) as usize][(x % 16) as usize]);
    let threshold = (noise_val / 256.0) - 0.5;
    let base_val = f32::from(lum) / 255.0;
    let dithered_val = (base_val + threshold * (1.0 / levels.max(2.0))).clamp(0.0, 1.0);
    (dithered_val * 255.0).round() as u8
}

/// Dispatcher : applique le dithering selon le mode configuré.
#[must_use]
#[inline(always)]
pub fn apply_dither(lum: u8, x: u32, y: u32, levels: f32, mode: &DitherMode) -> u8 {
    match mode {
        DitherMode::Bayer8x8 => apply_bayer_8x8(lum, x, y, levels),
        DitherMode::BlueNoise16 => apply_blue_noise_16(lum, x, y, levels),
        DitherMode::None => lum,
    }
}
