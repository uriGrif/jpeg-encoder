pub type RGBValue = (u8, u8, u8);
pub type YCbCrValue = (u8, u8, u8);

pub fn rgb_to_ycbcr((r, g, b): RGBValue) -> YCbCrValue {
    let r: f32 = r as f32;
    let g: f32 = g as f32;
    let b: f32 = b as f32;

    let y: u8 = (0.299 * r + 0.587 * g + 0.114 * b) as u8;
    let cb: u8 = (128.0 - 0.168736 * r - 0.331264 * g + 0.5 * b) as u8;
    let cr: u8 = (128.0 + 0.5 * r - 0.418688 * g + 0.081312 * b) as u8; // there are ways of doing this by shifting bits

    (y, cb, cr)
}

pub fn ycbcr_to_rgb((y, cb, cr): YCbCrValue) -> RGBValue {
    let y: f32 = y as f32;
    let cb: f32 = cb as f32;
    let cr: f32 = cr as f32;

    let r: u8 = (y + 1.402 * (cr - 128.0)) as u8;
    let g: u8 = (y - 0.344136 * (cb - 128.0) - 0.714136 * (cr - 128.0)) as u8;
    let b: u8 = (y + 1.772 * (cb - 128.0)) as u8;

    (r, g, b)
}
