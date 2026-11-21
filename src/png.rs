//! PNG images for tools that answer with a picture: RGBA pixels into a
//! PNG file (8-bit RGBA, no filter, zlib), and the file into base64, as
//! MCP's image content carries it.

use std::io::Write;

/// A PNG of `width * height` RGBA pixels, rows top to bottom.
pub fn encode(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut ihdr = Vec::new();
    ihdr.extend(width.to_be_bytes());
    ihdr.extend(height.to_be_bytes());
    // 8 bits, RGBA, deflate, no filter method but 0, no interlace.
    ihdr.extend([8, 6, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &ihdr);
    // Each row starts with its filter: 0, none.
    let mut raw = Vec::with_capacity(rgba.len() + height as usize);
    for row in rgba.chunks((width as usize * 4).max(1)) {
        raw.push(0);
        raw.extend(row);
    }
    let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    z.write_all(&raw).expect("writes to memory");
    chunk(&mut out, b"IDAT", &z.finish().expect("writes to memory"));
    chunk(&mut out, b"IEND", &[]);
    out
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend((data.len() as u32).to_be_bytes());
    let mut crc = flate2::Crc::new();
    crc.update(kind);
    crc.update(data);
    out.extend(kind);
    out.extend(data);
    out.extend(crc.sum().to_be_bytes());
}

/// Standard base64, padded.
pub fn base64(bytes: &[u8]) -> String {
    const ABC: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for group in bytes.chunks(3) {
        let n = group.iter().enumerate().fold(0u32, |n, (i, &b)| n | (b as u32) << (16 - 8 * i));
        for i in 0..4 {
            if i <= group.len() {
                out.push(ABC[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
#[path = "png.test.rs"]
mod tests;
