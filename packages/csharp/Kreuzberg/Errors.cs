using System.Text.RegularExpressions;

namespace Kreuzberg;

public enum KreuzbergErrorKind
{
    Unknown,
    Io,
    Validation,
    Parsing,
    Ocr,
    Cache,
    ImageProcessing,
    Serialization,
    MissingDependency,
    Plugin,
    UnsupportedFormat,
    Runtime,
}

public interface IKreuzbergError
{
    KreuzbergErrorKind Kind { get; }
}

public class KreuzbergException : Exception, IKreuzbergError
{
    public KreuzbergErrorKind Kind { get; }

    public KreuzbergException(KreuzbergErrorKind kind, string message, Exception? inner = null)
        : base(string.IsNullOrWhiteSpace(message) ? "kreuzberg: unknown error" : message, inner)
    {
        Kind = kind;
    }
}

public class KreuzbergValidationException : KreuzbergException
{
    public KreuzbergValidationException(string message, Exception? inner = null)
        : base(KreuzbergErrorKind.Validation, ErrorMapper.PrefixMessage(message, "Validation error"), inner)
    {
    }
}

public class KreuzbergParsingException : KreuzbergException
{
    public KreuzbergParsingException(string message, Exception? inner = null)
        : base(KreuzbergErrorKind.Parsing, ErrorMapper.PrefixMessage(message, "Parsing error"), inner)
    {
    }
}

public class KreuzbergOcrException : KreuzbergException
{
    public KreuzbergOcrException(string message, Exception? inner = null)
        : base(KreuzbergErrorKind.Ocr, ErrorMapper.PrefixMessage(message, "OCR error"), inner)
    {
    }
}

public class KreuzbergCacheException : KreuzbergException
{
    public KreuzbergCacheException(string message, Exception? inner = null)
        : base(KreuzbergErrorKind.Cache, ErrorMapper.PrefixMessage(message, "Cache error"), inner)
    {
    }
}

public class KreuzbergImageProcessingException : KreuzbergException
{
    public KreuzbergImageProcessingException(string message, Exception? inner = null)
        : base(KreuzbergErrorKind.ImageProcessing, ErrorMapper.PrefixMessage(message, "Image processing error"), inner)
    {
    }
}

public class KreuzbergSerializationException : KreuzbergException
{
    public KreuzbergSerializationException(string message, Exception? inner = null)
        : base(KreuzbergErrorKind.Serialization, ErrorMapper.PrefixMessage(message, "Serialization error"), inner)
    {
    }
}

public class KreuzbergMissingDependencyException : KreuzbergException
{
    public string Dependency { get; }

    public KreuzbergMissingDependencyException(string dependency, string message, Exception? inner = null)
        : base(KreuzbergErrorKind.MissingDependency, ErrorMapper.PrefixMessage(message, $"Missing dependency: {dependency}"), inner)
    {
        Dependency = dependency;
    }
}

public class KreuzbergPluginException : KreuzbergException
{
    public string PluginName { get; }

    public KreuzbergPluginException(string pluginName, string message, Exception? inner = null)
        : base(KreuzbergErrorKind.Plugin, ErrorMapper.PrefixMessage(message, "Plugin error"), inner)
    {
        PluginName = pluginName;
    }
}

public class KreuzbergUnsupportedFormatException : KreuzbergException
{
    public string Format { get; }

    public KreuzbergUnsupportedFormatException(string format, string message, Exception? inner = null)
        : base(KreuzbergErrorKind.UnsupportedFormat, ErrorMapper.PrefixMessage(message, $"Unsupported format: {format}"), inner)
    {
        Format = format;
    }
}

public class KreuzbergIOException : IOException, IKreuzbergError
{
    public KreuzbergErrorKind Kind => KreuzbergErrorKind.Io;

    public KreuzbergIOException(string message, Exception? inner = null)
        : base(ErrorMapper.PrefixMessage(message, "IO error"), inner)
    {
    }
}

public class KreuzbergRuntimeException : Exception, IKreuzbergError
{
    public KreuzbergErrorKind Kind => KreuzbergErrorKind.Runtime;

    public KreuzbergRuntimeException(string message, Exception? inner = null)
        : base(ErrorMapper.PrefixMessage(message, "Runtime error"), inner)
    {
    }
}

internal static class ErrorMapper
{
    internal static Exception Unknown(string message)
    {
        return new KreuzbergRuntimeException(message);
    }

    internal static Exception FromNativeError(string? error)
    {
        var trimmed = string.IsNullOrWhiteSpace(error) ? "Unknown error" : error.Trim();

        if (trimmed.StartsWith("Validation error:", StringComparison.OrdinalIgnoreCase))
        {
            return new KreuzbergValidationException(trimmed);
        }

        if (trimmed.StartsWith("Parsing error:", StringComparison.OrdinalIgnoreCase))
        {
            return new KreuzbergParsingException(trimmed);
        }

        if (trimmed.StartsWith("OCR error:", StringComparison.OrdinalIgnoreCase))
        {
            return new KreuzbergOcrException(trimmed);
        }

        if (trimmed.StartsWith("Cache error:", StringComparison.OrdinalIgnoreCase))
        {
            return new KreuzbergCacheException(trimmed);
        }

        if (trimmed.StartsWith("Image processing error:", StringComparison.OrdinalIgnoreCase))
        {
            return new KreuzbergImageProcessingException(trimmed);
        }

        if (trimmed.StartsWith("Serialization error:", StringComparison.OrdinalIgnoreCase))
        {
            return new KreuzbergSerializationException(trimmed);
        }

        if (trimmed.StartsWith("Missing dependency:", StringComparison.OrdinalIgnoreCase))
        {
            var dependency = trimmed["Missing dependency:".Length..].Trim();
            return new KreuzbergMissingDependencyException(dependency, trimmed);
        }

        if (trimmed.StartsWith("Plugin error", StringComparison.OrdinalIgnoreCase))
        {
            var name = ParsePluginName(trimmed);
            return new KreuzbergPluginException(name, trimmed);
        }

        if (trimmed.StartsWith("Unsupported format:", StringComparison.OrdinalIgnoreCase))
        {
            var format = trimmed["Unsupported format:".Length..].Trim();
            return new KreuzbergUnsupportedFormatException(format, trimmed);
        }

        if (trimmed.StartsWith("IO error:", StringComparison.OrdinalIgnoreCase))
        {
            return new KreuzbergIOException(trimmed);
        }

        if (trimmed.StartsWith("Lock poisoned:", StringComparison.OrdinalIgnoreCase) ||
            trimmed.StartsWith("Unsupported operation:", StringComparison.OrdinalIgnoreCase))
        {
            return new KreuzbergRuntimeException(trimmed);
        }

        return new KreuzbergRuntimeException(trimmed);
    }

    private static string ParsePluginName(string message)
    {
        var match = Regex.Match(message, @"Plugin error in ([^:]+)");
        if (match.Success)
        {
            return match.Groups[1].Value;
        }
        return "unknown";
    }

    internal static string PrefixMessage(string? message, string prefix)
    {
        var trimmed = string.IsNullOrWhiteSpace(message) ? string.Empty : message.Trim();
        if (trimmed.StartsWith(prefix, StringComparison.OrdinalIgnoreCase) || trimmed.StartsWith("kreuzberg:", StringComparison.OrdinalIgnoreCase))
        {
            return trimmed;
        }

        var baseMessage = string.IsNullOrWhiteSpace(trimmed) ? $"{prefix}: unknown error" : $"{prefix}: {trimmed}";
        return $"kreuzberg: {baseMessage}";
    }
}
