use std::io::{self, Cursor, Read};
use std::marker::PhantomData;
use std::mem;

use crate::color::ColorType;
use crate::error::{
    DecodingError, ImageError, ImageResult, LimitError, UnsupportedError, UnsupportedErrorKind,
};
use crate::image::{ImageDecoder, ImageFormat};
use crate::io::Limits;

type ZuneColorSpace = zune_core::colorspace::ColorSpace;

/// JPEG decoder
pub struct JpegDecoder<R> {
    input: Vec<u8>,
    orig_color_space: ZuneColorSpace,
    width: u16,
    height: u16,
    limits: Limits,
    // For API compatibility with the previous jpeg_decoder wrapper.
    // Can be removed later, which would be an API break.
    phantom: PhantomData<R>,
}

impl<R: Read> JpegDecoder<R> {
    /// Create a new decoder that decodes from the stream ```r```
    pub fn new(r: R) -> ImageResult<JpegDecoder<R>> {
        let mut input = Vec::new();
        let mut r = r;
        r.read_to_end(&mut input)?;
        let mut decoder = zune_jpeg::JpegDecoder::new(&input);
        decoder.decode_headers().map_err(ImageError::from_jpeg)?;
        // now that we've decoded the headers we can `.unwrap()`
        // all these functions that only fail if called before decoding the headers
        let (width, height) = decoder.dimensions().unwrap();
        let orig_color_space = decoder.get_output_colorspace().unwrap();
        // Limits are disabled by default for backwards compatibility with jpeg_decoder
        // which did not support limits, so enabling them would break crate users
        let limits = Limits::no_limits();
        Ok(JpegDecoder {
            input,
            orig_color_space,
            width,
            height,
            limits,
            phantom: PhantomData,
        })
    }

    /// Some decoders support scaling the image during decoding,
    /// but the current backend, `zune-jpeg`, doesn't,
    /// so this function currently does nothing
    /// and always returns the original dimensions.
    pub fn scale(
        &mut self,
        _requested_width: u16,
        _requested_height: u16,
    ) -> ImageResult<(u16, u16)> {
        // zune-jpeg doesn't support this yet:
        // https://github.com/etemesi254/zune-image/issues/103
        Ok((self.width, self.height))
    }
}

/// Wrapper struct around a `Cursor<Vec<u8>>`
pub struct JpegReader<R>(Cursor<Vec<u8>>, PhantomData<R>);
impl<R> Read for JpegReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
        if self.0.position() == 0 && buf.is_empty() {
            mem::swap(buf, self.0.get_mut());
            Ok(buf.len())
        } else {
            self.0.read_to_end(buf)
        }
    }
}

impl<'a, R: 'a + Read> ImageDecoder<'a> for JpegDecoder<R> {
    type Reader = JpegReader<R>;

    fn dimensions(&self) -> (u32, u32) {
        (u32::from(self.width), u32::from(self.height))
    }

    fn color_type(&self) -> ColorType {
        ColorType::from_jpeg(self.orig_color_space)
    }

    fn icc_profile(&mut self) -> Option<Vec<u8>> {
        let mut decoder = zune_jpeg::JpegDecoder::new(&self.input);
        decoder.decode_headers().ok()?;
        decoder.icc_profile()
    }

    fn into_reader(self) -> ImageResult<Self::Reader> {
        let mut decoder = new_zune_decoder(&self.input, self.orig_color_space, self.limits);
        let data = decoder.decode().map_err(ImageError::from_jpeg)?;
        Ok(JpegReader(Cursor::new(data), PhantomData))
    }

    fn read_image(self, buf: &mut [u8]) -> ImageResult<()> {
        let advertised_len = self.total_bytes();
        let actual_len = buf.len() as u64;

        if actual_len != advertised_len {
            return Err(ImageError::Decoding(DecodingError::new(
                ImageFormat::Jpeg.into(),
                format!(
                    "Length of the decoded data {actual_len}\
                    doesn't match the advertised dimensions of the image\
                    that imply length {advertised_len}"
                ),
            )));
        }

        let mut decoder = new_zune_decoder(&self.input, self.orig_color_space, self.limits);
        decoder.decode_into(buf).map_err(ImageError::from_jpeg)?;
        Ok(())
    }

    fn set_limits(&mut self, limits: Limits) -> ImageResult<()> {
        self.limits = limits;
        Ok(())
    }
}

impl ColorType {
    fn from_jpeg(colorspace: ZuneColorSpace) -> ColorType {
        let colorspace = to_supported_color_space(colorspace);
        use zune_core::colorspace::ColorSpace::*;
        match colorspace {
            // As of zune-jpeg 0.3.13 the output is always 8-bit,
            // but support for 16-bit JPEG might be added in the future.
            RGB => ColorType::Rgb8,
            RGBA => ColorType::Rgba8,
            Luma => ColorType::L8,
            LumaA => ColorType::La8,
            // to_supported_color_space() doesn't return any of the other variants
            _ => unreachable!(),
        }
    }
}

fn to_supported_color_space(orig: ZuneColorSpace) -> ZuneColorSpace {
    use zune_core::colorspace::ColorSpace::*;
    match orig {
        RGB | RGBA | Luma | LumaA => orig,
        // the rest is not supported by `image` so it will be converted to RGB during decoding
        _ => RGB,
    }
}

fn new_zune_decoder(
    input: &[u8],
    orig_color_space: ZuneColorSpace,
    limits: Limits,
) -> zune_jpeg::JpegDecoder {
    let target_color_space = to_supported_color_space(orig_color_space);
    let mut options =
        zune_core::options::DecoderOptions::default().jpeg_set_out_colorspace(target_color_space);
    if let Some(max_width) = limits.max_image_width {
        options = options.set_max_width(max_width as usize); // u32 to usize never truncates
    };
    if let Some(max_height) = limits.max_image_height {
        options = options.set_max_height(max_height as usize); // u32 to usize never truncates
    };
    zune_jpeg::JpegDecoder::new_with_options(options, &input)
}

impl ImageError {
    fn from_jpeg(err: zune_jpeg::errors::DecodeErrors) -> ImageError {
        use zune_jpeg::errors::DecodeErrors::*;
        match err {
            Unsupported(desc) => ImageError::Unsupported(UnsupportedError::from_format_and_kind(
                ImageFormat::Jpeg.into(),
                UnsupportedErrorKind::GenericFeature(format!("{:?}", desc)),
            )),
            LargeDimensions(_) => ImageError::Limits(LimitError::from_kind(
                crate::error::LimitErrorKind::DimensionError,
            )),
            err => ImageError::Decoding(DecodingError::new(ImageFormat::Jpeg.into(), err)),
        }
    }
}
