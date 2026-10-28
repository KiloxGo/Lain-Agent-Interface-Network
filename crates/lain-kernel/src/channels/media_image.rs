//! Bounded decoding of current conversation pixels; never accepts a device path.
use image::{AnimationDecoder, DynamicImage, ImageDecoder, ImageFormat, ImageReader, Limits};
use lain_wire::media::{MediaFailure, MediaFrame};
use sha2::{Digest, Sha256};
use std::io::{BufReader, Cursor};

pub(crate) const MAX_SOURCE_BYTES: usize = 8 * 1024 * 1024;
pub(crate) const MAX_FRAME_BYTES: usize = 384 * 1024;
const MAX_PIXELS: u64 = 24_000_000;
const MAX_ANIMATION_PIXELS: u64 = 64_000_000;
const MAX_ANIMATION_FRAMES: usize = 64;

pub(crate) struct PreparedFrame {
    pub metadata: MediaFrame,
    pub bytes: Vec<u8>,
}
pub(crate) struct PreparedImage {
    pub original_width: u32,
    pub original_height: u32,
    pub decoded_frames: u32,
    pub animation_truncated: bool,
    pub frames: Vec<PreparedFrame>,
}
fn limits() -> Limits {
    let mut limits = Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(128 * 1024 * 1024);
    limits
}
fn dimensions(width: u32, height: u32) -> Result<(), MediaFailure> {
    if width == 0
        || height == 0
        || width > 8192
        || height > 8192
        || u64::from(width) * u64::from(height) > MAX_PIXELS
    {
        Err(MediaFailure::TooLarge)
    } else {
        Ok(())
    }
}
fn jpeg(image: &DynamicImage, index: u32, at_ms: u64) -> Result<PreparedFrame, MediaFailure> {
    for edge in [2048, 1600, 1280, 1024, 768, 512] {
        let rgba = if image.width() <= edge && image.height() <= edge {
            image.to_rgba8()
        } else {
            image.thumbnail(edge, edge).to_rgba8()
        };
        let (width, height) = rgba.dimensions();
        let mut rgb = image::RgbImage::new(width, height);
        for (pixel, out) in rgba.pixels().zip(rgb.pixels_mut()) {
            let alpha = u32::from(pixel[3]);
            for channel in 0..3 {
                out[channel] =
                    ((u32::from(pixel[channel]) * alpha + 255 * (255 - alpha) + 127) / 255) as u8;
            }
        }
        let mut bytes = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, 82)
            .encode_image(&DynamicImage::ImageRgb8(rgb))
            .map_err(|_| MediaFailure::InvalidImage)?;
        if bytes.len() <= MAX_FRAME_BYTES {
            let metadata = MediaFrame {
                frame_index: index,
                at_ms,
                width,
                height,
                sha256: format!("{:x}", Sha256::digest(&bytes)),
                encoded_bytes: bytes.len().try_into().map_err(|_| MediaFailure::TooLarge)?,
            };
            return Ok(PreparedFrame { metadata, bytes });
        }
    }
    Err(MediaFailure::TooLarge)
}
fn animation(
    frames: image::Frames<'_>,
    width: u32,
    height: u32,
) -> Result<PreparedImage, MediaFailure> {
    if u64::from(width) * u64::from(height) > 4_000_000 {
        return Err(MediaFailure::TooLarge);
    }
    let mut samples = Vec::new();
    let mut last = None;
    let mut count = 0u32;
    let mut elapsed = 0u64;
    let mut pixels = 0u64;
    for frame in frames.take(MAX_ANIMATION_FRAMES) {
        let frame = frame.map_err(|_| MediaFailure::InvalidImage)?;
        let (numer, denom) = frame.delay().numer_denom_ms();
        let delay = u64::from(numer).div_ceil(u64::from(denom).max(1));
        let image = DynamicImage::ImageRgba8(frame.into_buffer());
        dimensions(image.width(), image.height())?;
        pixels = pixels.saturating_add(u64::from(image.width()) * u64::from(image.height()));
        if pixels > MAX_ANIMATION_PIXELS {
            return Err(MediaFailure::TooLarge);
        }
        if count == 0 || count == 8 {
            samples.push(jpeg(&image, count, elapsed)?);
        }
        last = Some((image, count, elapsed));
        count += 1;
        elapsed = elapsed.saturating_add(delay);
    }
    let (last, index, at) = last.ok_or(MediaFailure::InvalidImage)?;
    if samples
        .last()
        .is_none_or(|s| s.metadata.frame_index != index)
    {
        samples.push(jpeg(&last, index, at)?);
    }
    Ok(PreparedImage {
        original_width: width,
        original_height: height,
        decoded_frames: count,
        animation_truncated: count == MAX_ANIMATION_FRAMES as u32,
        frames: samples,
    })
}
pub(crate) fn prepare(raw: &[u8]) -> Result<PreparedImage, MediaFailure> {
    if raw.is_empty() || raw.len() > MAX_SOURCE_BYTES {
        return Err(MediaFailure::TooLarge);
    }
    let format = image::guess_format(raw).map_err(|_| MediaFailure::UnsupportedFormat)?;
    if !matches!(
        format,
        ImageFormat::Jpeg | ImageFormat::Png | ImageFormat::Gif | ImageFormat::WebP
    ) {
        return Err(MediaFailure::UnsupportedFormat);
    }
    let cursor = || BufReader::new(Cursor::new(raw));
    match format {
        ImageFormat::Gif => {
            let mut decoder = image::codecs::gif::GifDecoder::new(cursor())
                .map_err(|_| MediaFailure::InvalidImage)?;
            let (width, height) = decoder.dimensions();
            dimensions(width, height)?;
            decoder
                .set_limits(limits())
                .map_err(|_| MediaFailure::TooLarge)?;
            return animation(decoder.into_frames(), width, height);
        }
        ImageFormat::Png => {
            let mut decoder = image::codecs::png::PngDecoder::with_limits(cursor(), limits())
                .map_err(|_| MediaFailure::InvalidImage)?;
            let (width, height) = decoder.dimensions();
            dimensions(width, height)?;
            decoder
                .set_limits(limits())
                .map_err(|_| MediaFailure::TooLarge)?;
            if decoder.is_apng().map_err(|_| MediaFailure::InvalidImage)? {
                return animation(
                    decoder
                        .apng()
                        .map_err(|_| MediaFailure::InvalidImage)?
                        .into_frames(),
                    width,
                    height,
                );
            }
        }
        ImageFormat::WebP => {
            let mut decoder = image::codecs::webp::WebPDecoder::new(cursor())
                .map_err(|_| MediaFailure::InvalidImage)?;
            let (width, height) = decoder.dimensions();
            dimensions(width, height)?;
            decoder
                .set_limits(limits())
                .map_err(|_| MediaFailure::TooLarge)?;
            if decoder.has_animation() {
                return animation(decoder.into_frames(), width, height);
            }
        }
        _ => {}
    }
    let mut reader = ImageReader::with_format(cursor(), format);
    reader.limits(limits());
    let mut decoder = reader
        .into_decoder()
        .map_err(|_| MediaFailure::InvalidImage)?;
    let (width, height) = decoder.dimensions();
    dimensions(width, height)?;
    let orientation = decoder
        .orientation()
        .map_err(|_| MediaFailure::InvalidImage)?;
    let mut image = DynamicImage::from_decoder(decoder).map_err(|_| MediaFailure::InvalidImage)?;
    image.apply_orientation(orientation);
    Ok(PreparedImage {
        original_width: width,
        original_height: height,
        decoded_frames: 1,
        animation_truncated: false,
        frames: vec![jpeg(&image, 0, 0)?],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn png_pixels_become_real_jpeg_with_white_alpha_background_and_no_metadata() {
        let mut original = image::RgbaImage::from_pixel(16, 12, image::Rgba([0, 0, 0, 0]));
        original.put_pixel(8, 6, image::Rgba([255, 0, 0, 255]));
        let mut source = Cursor::new(Vec::new());
        DynamicImage::ImageRgba8(original)
            .write_to(&mut source, ImageFormat::Png)
            .unwrap();
        let output = prepare(source.get_ref()).unwrap();
        assert_eq!((output.original_width, output.original_height), (16, 12));
        assert_eq!(output.frames.len(), 1);
        let frame = &output.frames[0];
        assert_eq!(frame.metadata.encoded_bytes as usize, frame.bytes.len());
        assert_eq!((frame.metadata.width, frame.metadata.height), (16, 12));
        let decoded = image::load_from_memory(&frame.bytes).unwrap().to_rgb8();
        let white = decoded.get_pixel(0, 0);
        assert!(white.0.iter().all(|c| *c > 220));
        assert_eq!(
            frame.metadata.sha256,
            format!("{:x}", Sha256::digest(&frame.bytes))
        );
    }
    #[test]
    fn animated_sticker_samples_actual_temporal_frames_and_reports_prefix_limits() {
        let mut raw = Vec::new();
        {
            let mut encoder = image::codecs::gif::GifEncoder::new(&mut raw);
            for n in 0..12 {
                let color = if n < 6 {
                    [255, 0, 0, 255]
                } else {
                    [0, 0, 255, 255]
                };
                encoder
                    .encode_frame(image::Frame::from_parts(
                        image::RgbaImage::from_pixel(8, 8, image::Rgba(color)),
                        0,
                        0,
                        image::Delay::from_numer_denom_ms(100, 1),
                    ))
                    .unwrap();
            }
        }
        let output = prepare(&raw).unwrap();
        assert_eq!(output.decoded_frames, 12);
        assert!(!output.animation_truncated);
        assert_eq!(
            output
                .frames
                .iter()
                .map(|f| f.metadata.frame_index)
                .collect::<Vec<_>>(),
            [0, 8, 11]
        );
        assert_eq!(output.frames[2].metadata.at_ms, 1100);
        assert_ne!(output.frames[0].bytes, output.frames[2].bytes);
    }
    #[test]
    fn invalid_formats_sources_and_dimension_bombs_do_not_become_visual_evidence() {
        assert!(matches!(
            prepare(b"<svg>not accepted</svg>"),
            Err(MediaFailure::UnsupportedFormat)
        ));
        assert!(matches!(
            prepare(&vec![0; MAX_SOURCE_BYTES + 1]),
            Err(MediaFailure::TooLarge)
        ));
        assert!(dimensions(8192, 8192).is_err());
        assert!(dimensions(0, 10).is_err());
        assert!(prepare(b"GIF89a malformed").is_err());
    }
}
