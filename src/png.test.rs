use super::*;

#[test]
fn base64_pads() {
    assert_eq!(base64(b""), "");
    assert_eq!(base64(b"f"), "Zg==");
    assert_eq!(base64(b"fo"), "Zm8=");
    assert_eq!(base64(b"foo"), "Zm9v");
    assert_eq!(base64(b"foobar"), "Zm9vYmFy");
}

#[test]
fn a_png_has_its_chunks_and_its_pixels() {
    let png = encode(2, 1, &[255, 0, 0, 255, 0, 0, 255, 128]);
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    assert_eq!(&png[12..16], b"IHDR");
    assert_eq!(&png[16..24], &[0, 0, 0, 2, 0, 0, 0, 1]);
    assert!(png.ends_with(&[0, 0, 0, 0, b'I', b'E', b'N', b'D', 0xAE, 0x42, 0x60, 0x82]));
    // The pixels, back out of the IDAT chunk.
    let len = u32::from_be_bytes(png[33..37].try_into().unwrap()) as usize;
    assert_eq!(&png[37..41], b"IDAT");
    let mut raw = Vec::new();
    std::io::Read::read_to_end(&mut flate2::read::ZlibDecoder::new(&png[41..41 + len]), &mut raw).unwrap();
    assert_eq!(raw, [0, 255, 0, 0, 255, 0, 0, 255, 128]);
}
