// Ported from Rust `crates/xberg/src/extractors/image.rs` + `extraction/image.rs`.
//
// Metadata/EXIF only — recognition is not done here (the port runs OCR as a later pass). This
// mirrors the Rust `config.effective_disable_ocr()` branch: build a metadata-only document
// carrying an Image element plus `ImageMetadata` (width, height, format, exif), attaching the
// image's bytes when `config.needs_image_data()`. Dimensions and format come from
// SixLabors.ImageSharp; EXIF from the ImageSharp ExifProfile (see ExifReader).

using SixLabors.ImageSharp;
using SixLabors.ImageSharp.Formats;
using SixLabors.ImageSharp.Metadata.Profiles.Exif;
using XRay.Content.Core;
using XRay.Content.Internal.Exif;
using XRay.Content.Internal.Heif;
using XRay.Content.Types;

namespace XRay.Content.Extractors;

/// <summary>Image metadata extractor (PNG/JPEG/WebP/BMP/TIFF/GIF). No OCR.</summary>
public sealed class ImageExtractor : IExtractor
{
    public IEnumerable<string> SupportedMimeTypes => new[]
    {
        "image/png",
        "image/jpeg",
        "image/jpg",
        "image/pjpeg",
        "image/webp",
        "image/bmp",
        "image/x-bmp",
        "image/x-ms-bmp",
        "image/tiff",
        "image/x-tiff",
        "image/gif",

        // HEIF-family containers. ImageSharp cannot read these, so their metadata comes from
        // the container description instead; see HeifContainer for what that does and does not
        // cover.
        "image/heic",
        "image/heic-sequence",
        "image/heif",
        "image/heif-sequence",
        "image/avif",
    };

    public int Priority => 50;

    public InternalDocument Extract(ReadOnlySpan<byte> content, string mimeType, ExtractionConfig config)
    {
        byte[] bytes = content.ToArray();
        EnforceImagePageLimit(bytes, mimeType, config.SecurityLimits);
        var imageMeta = HeifContainer.IsHeifContainer(bytes)
            ? HeifMetadata(bytes)
            : RasterMetadata(bytes);

        // build_image_internal_document(None, attach_image), where upstream's attach_image is
        // `Some(extracted_image)` when config.needs_image_data() and None otherwise.
        var builder = new InternalDocumentBuilder("image");

        if (config.NeedsImageData())
        {
            bool measured = imageMeta.Width > 0 && imageMeta.Height > 0;

            builder.PushImage(null, new ExtractedImage
            {
                Data = bytes,
                Format = imageMeta.Format,
                ImageIndex = 0,
                // Left unset when the dimensions could not be read — a HEIF container's often
                // cannot — because zero would read as "smaller than the floor" to the OCR pass's
                // size filter and skip the image, where unknown means "recognise it anyway".
                Width = measured ? imageMeta.Width : null,
                Height = measured ? imageMeta.Height : null,
                IsMask = false,
                // image_kind::classify is skipped, as in the other extractors: the classifier is
                // not ported, so ImageKind / KindConfidence stay null.
            }, null, null);
        }
        else
        {
            // push_image_placeholder: an Image element referencing index 0 with nothing behind it.
            var kind = ElementKind.Image(0);
            builder.PushElement(new InternalElement
            {
                Id = InternalElementId.Generate(kind.Discriminant(), "", null, 0),
                Kind = kind,
                Text = "",
                Depth = 0,
                Layer = ContentLayer.Body,
            });
        }

        var doc = builder.Build();
        doc.Metadata = new Metadata { Format = FormatMetadata.Image(imageMeta) };
        doc.MimeType = mimeType;

        return doc;
    }

    /// <summary>
    /// Reject a multi-frame TIFF whose frame count exceeds the configured ceiling.
    /// </summary>
    /// <remarks>
    /// A page here is a TIFF frame; every other supported raster format is single-frame, so the
    /// check is scoped to TIFF. Frames are counted from the header alone — no raster data is
    /// decoded. A file that will not identify is left to the extraction path to report, rather than
    /// turned into a spurious page-limit rejection that masks the real error.
    /// </remarks>
    private static void EnforceImagePageLimit(byte[] bytes, string mimeType, SecurityLimits? limits)
    {
        if (limits?.MaxPages is null) return;
        if (!mimeType.Contains("tiff", StringComparison.OrdinalIgnoreCase)) return;

        int frameCount;
        try { frameCount = Image.Identify(bytes).FrameMetadataCollection.Count; }
        catch (Exception) when (Cancel.NotCancelling) { return; }

        DocumentLimits.EnforcePageCount(frameCount, limits);
    }

    private static ImageMetadata RasterMetadata(byte[] bytes)
    {
        ImageInfo info = Image.Identify(bytes);
        return new ImageMetadata
        {
            Width = (uint)info.Width,
            Height = (uint)info.Height,
            Format = FormatName(info.Metadata.DecodedImageFormat),
            Exif = ExifReader.Extract(info.Metadata.ExifProfile),
        };
    }

    /// <summary>
    /// Metadata for a HEIF-family container, read from its description rather than its pixels.
    /// </summary>
    /// <remarks>
    /// Upstream reports the dimensions libheif hands back after decoding; those are the coded
    /// extent with the clean aperture and rotation applied, which is exactly what the container
    /// itself states, so no picture has to be decoded to agree with it.
    /// </remarks>
    private static ImageMetadata HeifMetadata(byte[] bytes)
    {
        var info = HeifContainer.TryRead(bytes)
            ?? throw new InvalidDataException("Failed to read HEIF container metadata");

        return new ImageMetadata
        {
            Width = info.Width,
            Height = info.Height,
            Format = "HEIF",
            // A file with no EXIF item yields an empty map, the same as a raster image
            // whose profile is absent.
            Exif = ExifReader.Extract(info.Exif is { Length: > 0 } exif ? ReadExifProfile(exif) : null),
        };
    }

    /// <summary>Read a bare TIFF block as an EXIF profile, or nothing if it will not parse.</summary>
    private static ExifProfile? ReadExifProfile(byte[] exif)
    {
        try
        {
            return new ExifProfile(exif);
        }
        catch (Exception e) when (e is ArgumentException or IndexOutOfRangeException)
        {
            return null;
        }
    }

    // Rust uses `format!("{:?}", image::ImageFormat).to_uppercase()`; ImageSharp's format
    // name maps 1:1 for the common raster formats (PNG/JPEG/GIF/BMP/TIFF/WEBP).
    private static string FormatName(IImageFormat? format) =>
        format?.Name.ToUpperInvariant() ?? "";
}
