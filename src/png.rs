//! A minimal PNG encoder, written from the spec, so the crate has no dependencies.
//!
//! A PNG is an 8-byte signature followed by chunks (length, type, data, CRC-32).
//! Pixel data goes in IDAT as a zlib stream. We use deflate's "stored" blocks,
//! which hold raw bytes, so the encoder is ~70 lines. The trade-off is file size:
//! the image isn't compressed.

pub fn encode_rgb(width: u32, height: u32, rgb: &[u8]) -> Vec<u8> {
    assert_eq!(
        rgb.len(),
        width as usize * height as usize * 3,
        "buffer size doesn't match dimensions"
    );
    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();

    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]); // 8 bits/channel, truecolour RGB, deflate, no filter, no interlace
    chunk(&mut png, b"IHDR", &ihdr);

    // Each scanline is prefixed with its filter type (0 = none).
    let row = width as usize * 3;
    let mut raw = Vec::with_capacity((row + 1) * height as usize);
    for line in rgb.chunks(row) {
        raw.push(0);
        raw.extend_from_slice(line);
    }
    chunk(&mut png, b"IDAT", &zlib_stored(&raw));
    chunk(&mut png, b"IEND", &[]);
    png
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let start = out.len();
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let crc = crc32(&out[start..]); // covers the type and the data
    out.extend_from_slice(&crc.to_be_bytes());
}

/// zlib header, deflate stored blocks (max 65,535 bytes each), Adler-32 trailer.
fn zlib_stored(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x01];
    let blocks: Vec<&[u8]> = if data.is_empty() {
        vec![&[]]
    } else {
        data.chunks(65_535).collect()
    };
    for (i, block) in blocks.iter().enumerate() {
        out.push((i == blocks.len() - 1) as u8); // BFINAL bit, BTYPE = 00 (stored)
        let len = block.len() as u16;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&(!len).to_le_bytes());
        out.extend_from_slice(block);
    }
    out.extend_from_slice(&adler32(data).to_be_bytes());
    out
}

pub fn crc32(data: &[u8]) -> u32 {
    const TABLE: [u32; 256] = {
        let mut table = [0u32; 256];
        let mut n = 0;
        while n < 256 {
            let mut c = n as u32;
            let mut k = 0;
            while k < 8 {
                c = if c & 1 != 0 {
                    0xEDB8_8320 ^ (c >> 1)
                } else {
                    c >> 1
                };
                k += 1;
            }
            table[n] = c;
            n += 1;
        }
        table
    };
    !data.iter().fold(!0u32, |c, &b| {
        TABLE[((c ^ b as u32) & 0xFF) as usize] ^ (c >> 8)
    })
}

pub fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for chunk in data.chunks(5552) {
        // 5552 is the largest n where the sums can't overflow before the modulo
        for &byte in chunk {
            a += byte as u32;
            b += a;
        }
        a %= 65_521;
        b %= 65_521;
    }
    (b << 16) | a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksums_match_known_values() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b"IEND"), 0xAE42_6082); // the CRC every PNG ends with
        assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
    }

    #[test]
    fn png_structure_is_valid() {
        let (w, h) = (300u32, 250u32); // > 65,535 bytes of pixels, so multiple blocks
        let rgb: Vec<u8> = (0..w * h * 3).map(|i| (i % 251) as u8).collect();
        let png = encode_rgb(w, h, &rgb);
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        assert_eq!(&png[12..16], b"IHDR");
        assert_eq!(u32::from_be_bytes(png[16..20].try_into().unwrap()), w);
        assert_eq!(&png[png.len() - 8..], b"IEND\xAE\x42\x60\x82");

        // Walk the chunks, verify each CRC, and un-wrap the stored deflate blocks.
        let mut pos = 8;
        let mut pixels = Vec::new();
        while pos < png.len() {
            let len = u32::from_be_bytes(png[pos..pos + 4].try_into().unwrap()) as usize;
            let body = &png[pos + 4..pos + 8 + len];
            let crc = u32::from_be_bytes(png[pos + 8 + len..pos + 12 + len].try_into().unwrap());
            assert_eq!(crc32(body), crc);
            if &body[..4] == b"IDAT" {
                let mut z = &body[6..body.len() - 4];
                while !z.is_empty() {
                    let n = u16::from_le_bytes([z[1], z[2]]) as usize;
                    pixels.extend_from_slice(&z[5..5 + n]);
                    z = &z[5 + n..];
                }
            }
            pos += 12 + len;
        }
        let rows: Vec<u8> = pixels
            .chunks(w as usize * 3 + 1)
            .flat_map(|r| r[1..].to_vec())
            .collect();
        assert_eq!(rows, rgb);
    }
}
