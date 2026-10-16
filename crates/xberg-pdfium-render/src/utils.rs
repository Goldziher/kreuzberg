pub(crate) mod pixels {

    const BYTES_PER_GRAYSCALE_PIXEL: usize = 1;
    const BYTES_PER_THREE_CHANNEL_PIXEL: usize = 3;
    const BYTES_PER_FOUR_CHANNEL_PIXEL: usize = 4;

    /// Converts the given byte array, containing pixel data encoded as three-channel RGB,
    /// into pixel data encoded as four-channel RGBA. A new alpha channel is created with full opacity.
    ///
    /// This function assumes that the RGB pixel data does not include any additional alignment bytes.
    /// If the source data includes alignment bytes, then the [aligned_bgr_to_rgba()] function
    /// should be used instead.
    #[allow(unused)]
    pub(crate) fn unaligned_rgb_to_rgba(rgb: &[u8]) -> Vec<u8> {
        rgb.as_chunks::<BYTES_PER_THREE_CHANNEL_PIXEL>()
            .0
            .iter()
            .flat_map(|channels| [channels[0], channels[1], channels[2], 255])
            .collect::<Vec<_>>()
    }

    /// Converts the given byte array, containing pixel data encoded as three-channel RGB with
    /// one or more empty alignment bytes, into pixel data encoded as four-channel RGBA.
    /// A new alpha channel is created with full opacity.
    ///
    /// Alignment bytes are used to ensure that the stride of a bitmap - the length in bytes of
    /// a single scanline - is always a multiple of four, irrespective of the pixel data format.
    /// The number of empty alignment bytes to be skipped is determined from the given width
    /// and stride parameters.
    ///
    /// If alignment bytes are not used in the source pixel data, then the [unaligned_rgb_to_rgba()]
    /// function should be used instead.
    #[inline]
    pub(crate) fn aligned_rgb_to_rgba(rgb: &[u8], width: usize, stride: usize) -> Vec<u8> {
        rgb.chunks_exact(stride)
            .flat_map(|scanline| {
                scanline[..width * BYTES_PER_THREE_CHANNEL_PIXEL]
                    .as_chunks::<BYTES_PER_THREE_CHANNEL_PIXEL>()
                    .0
                    .iter()
            })
            .flat_map(|channels| [channels[0], channels[1], channels[2], 255])
            .collect::<Vec<_>>()
    }

    /// Converts the given byte array, containing pixel data encoded as three-channel BGR,
    /// into pixel data encoded as four-channel RGBA. A new alpha channel is created with full opacity.
    ///
    /// This function assumes that the BGR pixel data does not include any additional alignment bytes.
    /// If the source data includes alignment bytes, then the [aligned_bgr_to_rgba()] function
    /// should be used instead.
    #[inline]
    pub(crate) fn unaligned_bgr_to_rgba(bgr: &[u8]) -> Vec<u8> {
        bgr.as_chunks::<BYTES_PER_THREE_CHANNEL_PIXEL>()
            .0
            .iter()
            .flat_map(|channels| [channels[2], channels[1], channels[0], 255])
            .collect::<Vec<_>>()
    }

    /// Converts the given byte array, containing pixel data encoded as three-channel BGR with
    /// one or more empty alignment bytes, into pixel data encoded as four-channel RGBA.
    /// A new alpha channel is created with full opacity.
    ///
    /// Alignment bytes are used to ensure that the stride of a bitmap - the length in bytes of
    /// a single scanline - is always a multiple of four, irrespective of the pixel data format.
    /// The number of empty alignment bytes to be skipped is determined from the given width
    /// and stride parameters.
    ///
    /// If alignment bytes are not used in the source pixel data, then the [unaligned_bgr_to_rgba()]
    /// function should be used instead.
    #[inline]
    pub(crate) fn aligned_bgr_to_rgba(bgr: &[u8], width: usize, stride: usize) -> Vec<u8> {
        bgr.chunks_exact(stride)
            .flat_map(|scanline| {
                scanline[..width * BYTES_PER_THREE_CHANNEL_PIXEL]
                    .as_chunks::<BYTES_PER_THREE_CHANNEL_PIXEL>()
                    .0
                    .iter()
            })
            .flat_map(|channels| [channels[2], channels[1], channels[0], 255])
            .collect::<Vec<_>>()
    }

    /// Converts the given byte array, containing pixel data encoded as four-channel BGRA,
    /// into pixel data encoded as four-channel RGBA.
    #[inline]
    pub(crate) fn bgra_to_rgba(bgra: &[u8]) -> Vec<u8> {
        bgra.as_chunks::<BYTES_PER_FOUR_CHANNEL_PIXEL>()
            .0
            .iter()
            .flat_map(|channels| [channels[2], channels[1], channels[0], channels[3]])
            .collect::<Vec<_>>()
    }

    /// Converts the given byte array, containing pixel data encoded as three-channel RGB,
    /// into pixel data encoded as four-channel BGRA. A new alpha channel is created with full opacity.
    #[inline]
    pub(crate) fn unaligned_rgb_to_bgra(rgb: &[u8]) -> Vec<u8> {
        unaligned_bgr_to_rgba(rgb)
    }

    /// Converts the given byte array, containing pixel data encoded as four-channel RGBA,
    /// into pixel data encoded as four-channel BGRA.
    #[inline]
    pub(crate) fn rgba_to_bgra(rgba: &[u8]) -> Vec<u8> {
        bgra_to_rgba(rgba)
    }

    /// Converts the given byte array, containing pixel data encoded as three-channel BGR,
    /// into pixel data encoded as four-channel BGRA. A new alpha channel is created with full opacity.
    ///
    /// This function assumes that the BGR pixel data does not include any additional alignment bytes.
    /// If the source data includes alignment bytes, then the [aligned_bgr_to_rgba()] function
    /// should be used instead.
    #[allow(unused)]
    #[inline]
    pub(crate) fn unaligned_bgr_to_bgra(bgr: &[u8]) -> Vec<u8> {
        unaligned_rgb_to_rgba(bgr)
    }

    /// Converts the given byte array, containing pixel data encoded as three-channel BGR with
    /// one or more empty alignment bytes, into pixel data encoded as four-channel BGRA.
    /// A new alpha channel is created with full opacity.
    ///
    /// Alignment bytes are used to ensure that the stride of a bitmap - the length in bytes of
    /// a single scanline - is always a multiple of four, irrespective of the pixel data format.
    /// The number of empty alignment bytes to be skipped is determined from the given width
    /// and stride parameters.
    ///
    /// If alignment bytes are not used in the source pixel data, then the [unaligned_rgb_to_rgba()]
    /// function should be used instead.
    #[allow(unused)]
    #[inline]
    pub(crate) fn aligned_bgr_to_bgra(bgr: &[u8], width: usize, stride: usize) -> Vec<u8> {
        aligned_rgb_to_rgba(bgr, width, stride)
    }

    /// Converts the given byte array, containing pixel data encoded as one-channel grayscale
    /// with one or more empty alignment bytes alignment bytes, into unaligned pixel data.
    ///
    /// Alignment bytes are used to ensure that the stride of a bitmap - the length in bytes of
    /// a single scanline - is always a multiple of four, irrespective of the pixel data format.
    /// The number of empty alignment bytes to be skipped is determined from the given width
    /// and stride parameters.
    #[inline]
    pub(crate) fn aligned_grayscale_to_unaligned(grayscale: &[u8], width: usize, stride: usize) -> Vec<u8> {
        grayscale
            .chunks_exact(stride)
            .flat_map(|scanline| &scanline[..width * BYTES_PER_GRAYSCALE_PIXEL])
            .copied()
            .collect::<Vec<_>>()
    }
}

pub(crate) mod dates {
    use chrono::prelude::*;
    use std::fmt::Display;

    /// Converts a [DateTime] to a formatted PDF date string, as defined in The PDF Reference
    /// Manual, sixth edition, section 3.8.3, on page 160.
    #[inline]
    pub(crate) fn date_time_to_pdf_string<T, O>(date: DateTime<T>) -> String
    where
        T: TimeZone<Offset = O>,
        O: Display,
    {
        let date_part = date.format("%Y%m%d%H%M%S");

        let timezone_part = format!("{}'", date.format("%:z"))
            .replace("+00:00'", "Z00'00'")
            .replace(':', "'");

        format!("D:{date_part}{timezone_part}")
    }
}

pub(crate) mod mem {
    /// Creates an empty byte buffer of the given length.
    #[inline]
    pub(crate) fn create_byte_buffer(length: usize) -> Vec<u8> {
        create_sized_buffer::<u8>(length)
    }

    /// Creates an empty buffer of the given type with the given capacity.
    /// The contents of the buffer will be uninitialized.
    #[inline]
    #[allow(clippy::uninit_vec)]
    pub(crate) fn create_sized_buffer<T>(capacity: usize) -> Vec<T> {
        let mut buffer = Vec::<T>::with_capacity(capacity);

        unsafe {
            buffer.set_len(capacity);
        }

        buffer
    }
}

pub(crate) mod utf16le {
    use utf16string::{LittleEndian, WString};

    /// Converts the given Rust &str into an UTF16-LE encoded byte buffer.
    #[inline]
    pub(crate) fn get_pdfium_utf16le_bytes_from_str(str: &str) -> Vec<u8> {
        let mut bytes = WString::<LittleEndian>::from(str).into_bytes();

        bytes.push(0);
        bytes.push(0);

        bytes
    }

    /// Converts the bytes in the given buffer from UTF16-LE to a standard Rust String.
    #[allow(unused_mut)]
    pub(crate) fn get_string_from_pdfium_utf16le_bytes(mut buffer: Vec<u8>) -> Option<String> {
        #[cfg(target_arch = "wasm32")]
        {
            use web_sys::TextDecoder;

            if let Ok(decoder) = TextDecoder::new_with_label("utf-16le")
                && let Ok(result) = decoder.decode_with_u8_array(&buffer)
            {
                let result = result.trim_end_matches(char::from(0));

                if !result.is_empty() {
                    return Some(result.to_owned());
                } else {
                    return None;
                }
            }
        }

        if let Ok(string) = WString::<LittleEndian>::from_utf16(buffer) {
            let result = string.to_utf8().trim_end_matches(char::from(0)).to_owned();

            if !result.is_empty() { Some(result) } else { None }
        } else {
            None
        }
    }
}

pub(crate) mod files {
    use crate::bindgen::{FPDF_FILEACCESS, FPDF_FILEWRITE};
    use std::io::{Read, Seek, SeekFrom, Write};
    use std::ops::Deref;
    use std::os::raw::{c_int, c_uchar, c_ulong, c_void};
    use std::ptr::null_mut;
    use std::slice;

    /// Returns a wrapped Pdfium `FPDF_FILEACCESS` struct that uses the given reader as an
    /// input source for Pdfium's file access callback function.
    ///
    /// Because Pdfium must know the total content length in advance prior to loading
    /// any portion of it, the given reader must implement the `Seek` trait as well as
    /// the `Read` trait.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    pub(crate) fn get_pdfium_file_accessor_from_reader<'a, R: Read + Seek + 'a>(
        mut reader: R,
    ) -> Box<FpdfFileAccessExt<'a>> {
        let content_length = reader.seek(SeekFrom::End(0)).unwrap_or(0) as c_ulong;

        let mut result = Box::new(FpdfFileAccessExt {
            content_length,
            get_block: Some(read_block_from_callback),
            file_access_ptr: null_mut(),
            reader: Box::new(reader),
        });

        let file_access_ptr: *const FpdfFileAccessExt = result.deref();

        result.as_mut().file_access_ptr = file_access_ptr as *mut FpdfFileAccessExt;

        result
    }

    trait PdfiumDocumentReader: Read + Seek {}

    impl<R: Read + Seek> PdfiumDocumentReader for R {}

    #[repr(C)]
    pub(crate) struct FpdfFileAccessExt<'a> {
        content_length: c_ulong,
        get_block: Option<
            unsafe extern "C" fn(
                reader_ptr: *mut FpdfFileAccessExt,
                position: c_ulong,
                buf: *mut c_uchar,
                size: c_ulong,
            ) -> c_int,
        >,
        file_access_ptr: *mut FpdfFileAccessExt<'a>,
        reader: Box<dyn PdfiumDocumentReader + 'a>,
    }

    impl<'a> FpdfFileAccessExt<'a> {
        /// Returns an `FPDF_FILEACCESS` pointer suitable for passing to `FPDF_LoadCustomDocument()`.
        #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
        #[cfg_attr(feature = "thread_safe", allow(dead_code))]
        #[inline]
        pub(crate) fn as_fpdf_file_access_mut_ptr(&mut self) -> &mut FPDF_FILEACCESS {
            unsafe { &mut *(self as *mut FpdfFileAccessExt as *mut FPDF_FILEACCESS) }
        }
    }

    pub(crate) extern "C" fn read_block_from_callback(
        file_access_ptr: *mut FpdfFileAccessExt,
        position: c_ulong,
        buf: *mut c_uchar,
        size: c_ulong,
    ) -> c_int {
        unsafe {
            let reader = (*file_access_ptr).reader.as_mut();

            #[allow(clippy::unnecessary_cast)]
            let result = match reader.seek(SeekFrom::Start(position as u64)) {
                Ok(_) => reader.read(slice::from_raw_parts_mut(buf, size as usize)).unwrap_or(0),
                Err(_) => 0,
            };

            result as c_int
        }
    }

    /// Returns a wrapped Pdfium `FPDF_FILEWRITE` struct that uses the given writer as an
    /// output source for Pdfium's file writing callback function.
    pub(crate) fn get_pdfium_file_writer_from_writer<W: Write + 'static>(writer: &mut W) -> FpdfFileWriteExt<'_> {
        FpdfFileWriteExt {
            version: 1,
            write_block: Some(write_block_from_callback),
            writer,
        }
    }

    trait PdfiumDocumentWriter: Write {}

    impl<W: Write> PdfiumDocumentWriter for W {}

    #[repr(C)]
    pub(crate) struct FpdfFileWriteExt<'a> {
        version: c_int,
        write_block: Option<
            unsafe extern "C" fn(file_write_ext_ptr: *mut FpdfFileWriteExt, buf: *const c_void, size: c_ulong) -> c_int,
        >,
        writer: &'a mut dyn PdfiumDocumentWriter,
    }

    impl<'a> FpdfFileWriteExt<'a> {
        /// Returns an `FPDF_FILEWRITE` pointer suitable for passing to `FPDF_SaveAsCopy()`
        /// or `FPDF_SaveWithVersion()`.
        #[inline]
        pub(crate) fn as_fpdf_file_write_mut_ptr(&mut self) -> &mut FPDF_FILEWRITE {
            unsafe { &mut *(self as *mut FpdfFileWriteExt as *mut FPDF_FILEWRITE) }
        }

        /// Flushes the buffer of the underlying Rust writer.
        #[inline]
        pub(crate) fn flush(&mut self) -> std::io::Result<()> {
            self.writer.flush()
        }
    }

    pub(crate) extern "C" fn write_block_from_callback(
        file_write_ext_ptr: *mut FpdfFileWriteExt,
        buf: *const c_void,
        size: c_ulong,
    ) -> c_int {
        unsafe {
            match (*file_write_ext_ptr)
                .writer
                .write_all(slice::from_raw_parts(buf as *const u8, size as usize))
            {
                Ok(()) => 1,
                Err(_) => 0,
            }
        }
    }
}

#[cfg(test)]
pub(crate) mod test {

    use crate::error::PdfiumError;
    use crate::pdfium::Pdfium;
    use std::path::PathBuf;

    /// Returns the path to a vendored pdfium-render test fixture PDF.
    ///
    /// Test fixtures live in `test_documents/vendored/pdfium-render/` at the workspace root.
    pub(crate) fn test_fixture_path(filename: &str) -> PathBuf {
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("failed to resolve workspace root");
        workspace_root
            .join("test_documents/vendored/pdfium-render")
            .join(filename)
    }

    #[inline]
    #[cfg(pdfium_use_static)]
    pub(crate) fn test_bind_to_pdfium() -> Pdfium {
        Pdfium::default()
    }

    #[inline]
    #[cfg(not(pdfium_use_static))]
    pub(crate) fn test_bind_to_pdfium() -> Pdfium {
        match Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
            .or_else(|_| Pdfium::bind_to_system_library())
        {
            Ok(bindings) => Pdfium::new(bindings),
            Err(PdfiumError::PdfiumLibraryBindingsAlreadyInitialized) => Pdfium {},
            Err(e) => panic!("{:?}", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::utils::dates::*;
    use crate::utils::pixels::*;
    use crate::utils::utf16le::*;
    use chrono::prelude::*;

    #[test]
    fn test_unaligned_bgr_to_rgba() {
        let data: [u8; 15] = [2, 1, 0, 3, 6, 5, 4, 7, 10, 9, 8, 11, 14, 13, 12];

        let result = unaligned_rgb_to_bgra(data.as_slice());

        assert_eq!(
            result,
            [
                0, 1, 2, 255, 5, 6, 3, 255, 10, 7, 4, 255, 11, 8, 9, 255, 12, 13, 14, 255
            ]
        );
    }

    #[test]
    fn test_aligned_bgr_to_rgba() {
        let data: [u8; 24] = [
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
        ];

        let result = aligned_bgr_to_rgba(data.as_slice(), 1, 4);

        assert_eq!(
            result,
            [
                2, 1, 0, 255, 6, 5, 4, 255, 10, 9, 8, 255, 14, 13, 12, 255, 18, 17, 16, 255, 22, 21, 20, 255
            ]
        );

        let result = aligned_bgr_to_rgba(data.as_slice(), 2, 8);

        assert_eq!(
            result,
            [
                2, 1, 0, 255, 5, 4, 3, 255, 10, 9, 8, 255, 13, 12, 11, 255, 18, 17, 16, 255, 21, 20, 19, 255
            ]
        );

        let result = aligned_bgr_to_rgba(data.as_slice(), 3, 12);

        assert_eq!(
            result,
            [
                2, 1, 0, 255, 5, 4, 3, 255, 8, 7, 6, 255, 14, 13, 12, 255, 17, 16, 15, 255, 20, 19, 18, 255
            ]
        );

        let result = aligned_bgr_to_rgba(data.as_slice(), 4, 12);

        assert_eq!(
            result,
            [
                2, 1, 0, 255, 5, 4, 3, 255, 8, 7, 6, 255, 11, 10, 9, 255, 14, 13, 12, 255, 17, 16, 15, 255, 20, 19, 18,
                255, 23, 22, 21, 255
            ]
        );
    }

    #[test]
    fn test_bgra_to_rgba() {
        let data: [u8; 16] = [2, 1, 0, 3, 6, 5, 4, 7, 10, 9, 8, 11, 14, 13, 12, 15];

        let result = bgra_to_rgba(data.as_slice());

        assert_eq!(result, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]);
    }

    #[test]
    fn test_rgb_to_bgra() {
        let data: [u8; 15] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14];

        let result = unaligned_rgb_to_bgra(data.as_slice());

        assert_eq!(
            result,
            [
                2, 1, 0, 255, 5, 4, 3, 255, 8, 7, 6, 255, 11, 10, 9, 255, 14, 13, 12, 255
            ]
        );
    }

    #[test]
    fn test_rgba_to_bgra() {
        let data: [u8; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];

        let result = rgba_to_bgra(data.as_slice());

        assert_eq!(result, [2, 1, 0, 3, 6, 5, 4, 7, 10, 9, 8, 11, 14, 13, 12, 15]);
    }

    #[test]
    fn test_aligned_grayscale_to_unaligned() {
        let data: [u8; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];

        let result = aligned_grayscale_to_unaligned(data.as_slice(), 1, 4);

        assert_eq!(result, [0, 4, 8, 12]);
    }

    #[test]
    fn test_date_time_to_pdf_date_string() {
        assert_eq!(
            date_time_to_pdf_string(Utc.with_ymd_and_hms(1998, 12, 23, 19, 52, 00).unwrap()),
            "D:19981223195200Z00'00'"
        );

        assert_eq!(
            date_time_to_pdf_string(
                FixedOffset::west_opt(8 * 3600)
                    .unwrap()
                    .from_local_datetime(
                        &NaiveDate::from_ymd_opt(1998, 12, 23)
                            .unwrap()
                            .and_hms_opt(19, 52, 00)
                            .unwrap()
                    )
                    .unwrap()
            ),
            "D:19981223195200-08'00'"
        )
    }

    #[test]
    fn test_valid_utf16le_from_emoji() {
        let emoji = "💁👵🧕";

        assert_eq!(
            get_string_from_pdfium_utf16le_bytes(get_pdfium_utf16le_bytes_from_str(emoji)).unwrap(),
            emoji
        );
    }
}
