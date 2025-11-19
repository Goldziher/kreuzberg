package kreuzberg

import (
	"fmt"
	"strings"
)

// ErrorKind identifies the category of a Kreuzberg error.
type ErrorKind string

const (
	ErrorKindUnknown           ErrorKind = "unknown"
	ErrorKindIO                ErrorKind = "io"
	ErrorKindValidation        ErrorKind = "validation"
	ErrorKindParsing           ErrorKind = "parsing"
	ErrorKindOCR               ErrorKind = "ocr"
	ErrorKindCache             ErrorKind = "cache"
	ErrorKindImageProcessing   ErrorKind = "image_processing"
	ErrorKindSerialization     ErrorKind = "serialization"
	ErrorKindMissingDependency ErrorKind = "missing_dependency"
	ErrorKindPlugin            ErrorKind = "plugin"
	ErrorKindUnsupportedFormat ErrorKind = "unsupported_format"
	ErrorKindRuntime           ErrorKind = "runtime"
)

// KreuzbergError is implemented by all custom error types returned by the Go binding.
type KreuzbergError interface {
	error
	Kind() ErrorKind
}

type baseError struct {
	kind    ErrorKind
	message string
	cause   error
}

func (e *baseError) Error() string {
	return e.message
}

func (e *baseError) Kind() ErrorKind {
	return e.kind
}

func (e *baseError) Unwrap() error {
	return e.cause
}

type ValidationError struct {
	baseError
}

type ParsingError struct {
	baseError
}

type OCRError struct {
	baseError
}

type CacheError struct {
	baseError
}

type ImageProcessingError struct {
	baseError
}

type SerializationError struct {
	baseError
}

type MissingDependencyError struct {
	baseError
	Dependency string
}

type PluginError struct {
	baseError
	PluginName string
}

type UnsupportedFormatError struct {
	baseError
	Format string
}

type IOError struct {
	baseError
}

type RuntimeError struct {
	baseError
}

func makeBaseError(kind ErrorKind, message string, cause error) baseError {
	return baseError{
		kind:    kind,
		message: formatErrorMessageWithCause(message, cause),
		cause:   cause,
	}
}

func newValidationError(message string, cause error) *ValidationError {
	return &ValidationError{baseError: makeBaseError(ErrorKindValidation, message, cause)}
}

func newParsingError(message string, cause error) *ParsingError {
	return &ParsingError{baseError: makeBaseError(ErrorKindParsing, message, cause)}
}

func newOCRError(message string, cause error) *OCRError {
	return &OCRError{baseError: makeBaseError(ErrorKindOCR, message, cause)}
}

func newCacheError(message string, cause error) *CacheError {
	return &CacheError{baseError: makeBaseError(ErrorKindCache, message, cause)}
}

func newImageProcessingError(message string, cause error) *ImageProcessingError {
	return &ImageProcessingError{baseError: makeBaseError(ErrorKindImageProcessing, message, cause)}
}

func newSerializationError(message string, cause error) *SerializationError {
	return &SerializationError{baseError: makeBaseError(ErrorKindSerialization, message, cause)}
}

func newMissingDependencyError(dependency string, message string, cause error) *MissingDependencyError {
	return &MissingDependencyError{
		baseError:  makeBaseError(ErrorKindMissingDependency, messageWithFallback(message, fmt.Sprintf("Missing dependency: %s", dependency)), cause),
		Dependency: dependency,
	}
}

func newPluginError(plugin string, message string, cause error) *PluginError {
	return &PluginError{
		baseError:  makeBaseError(ErrorKindPlugin, messageWithFallback(message, "Plugin error"), cause),
		PluginName: plugin,
	}
}

func newUnsupportedFormatError(format string, message string, cause error) *UnsupportedFormatError {
	return &UnsupportedFormatError{
		baseError: makeBaseError(ErrorKindUnsupportedFormat, messageWithFallback(message, fmt.Sprintf("Unsupported format: %s", format)), cause),
		Format:    format,
	}
}

func newIOError(message string, cause error) *IOError {
	return &IOError{baseError: makeBaseError(ErrorKindIO, message, cause)}
}

func newRuntimeError(message string, cause error) *RuntimeError {
	return &RuntimeError{baseError: makeBaseError(ErrorKindRuntime, message, cause)}
}

func messageWithFallback(message string, fallback string) string {
	trimmed := strings.TrimSpace(message)
	if trimmed != "" {
		return trimmed
	}
	return fallback
}

func formatErrorMessageWithCause(message string, cause error) string {
	msg := formatErrorMessage(message)
	if cause != nil {
		return fmt.Sprintf("%s: %v", msg, cause)
	}
	return msg
}

func formatErrorMessage(message string) string {
	trimmed := strings.TrimSpace(message)
	if trimmed == "" {
		trimmed = "unknown error"
	}
	if strings.HasPrefix(strings.ToLower(trimmed), "kreuzberg:") {
		return trimmed
	}
	return "kreuzberg: " + trimmed
}

func classifyNativeError(message string) error {
	trimmed := strings.TrimSpace(message)
	if trimmed == "" {
		return newRuntimeError("unknown error", nil)
	}

	switch {
	case strings.HasPrefix(trimmed, "Validation error:"):
		return newValidationError(trimmed, nil)
	case strings.HasPrefix(trimmed, "Parsing error:"):
		return newParsingError(trimmed, nil)
	case strings.HasPrefix(trimmed, "OCR error:"):
		return newOCRError(trimmed, nil)
	case strings.HasPrefix(trimmed, "Cache error:"):
		return newCacheError(trimmed, nil)
	case strings.HasPrefix(trimmed, "Image processing error:"):
		return newImageProcessingError(trimmed, nil)
	case strings.HasPrefix(trimmed, "Serialization error:"):
		return newSerializationError(trimmed, nil)
	case strings.HasPrefix(trimmed, "Missing dependency:"):
		dependency := strings.TrimSpace(trimmed[len("Missing dependency:"):])
		return newMissingDependencyError(dependency, trimmed, nil)
	case strings.HasPrefix(trimmed, "Plugin error in "):
		plugin := parsePluginName(trimmed)
		return newPluginError(plugin, trimmed, nil)
	case strings.HasPrefix(trimmed, "Unsupported format:"):
		format := strings.TrimSpace(trimmed[len("Unsupported format:"):])
		return newUnsupportedFormatError(format, trimmed, nil)
	case strings.HasPrefix(trimmed, "IO error:"):
		return newIOError(trimmed, nil)
	case strings.HasPrefix(trimmed, "Lock poisoned:"):
		return newRuntimeError(trimmed, nil)
	case strings.HasPrefix(trimmed, "Unsupported operation:"):
		return newRuntimeError(trimmed, nil)
	}

	lower := strings.ToLower(trimmed)
	if strings.Contains(lower, "lock poisoned") {
		return newRuntimeError(trimmed, nil)
	}
	if strings.Contains(lower, "missing dependency") {
		return newMissingDependencyError("", trimmed, nil)
	}

	return newRuntimeError(trimmed, nil)
}

func parsePluginName(message string) string {
	start := strings.Index(message, "'")
	if start == -1 {
		return ""
	}
	rest := message[start+1:]
	end := strings.Index(rest, "'")
	if end == -1 {
		return ""
	}
	return rest[:end]
}
