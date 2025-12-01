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

// ErrorCode represents FFI error codes from kreuzberg-ffi.
type ErrorCode int32

const (
	ErrorCodeSuccess           ErrorCode = 0
	ErrorCodeGenericError      ErrorCode = 1
	ErrorCodePanic             ErrorCode = 2
	ErrorCodeInvalidArgument   ErrorCode = 3
	ErrorCodeIoError           ErrorCode = 4
	ErrorCodeParsingError      ErrorCode = 5
	ErrorCodeOcrError          ErrorCode = 6
	ErrorCodeMissingDependency ErrorCode = 7
)

// String returns the string representation of an ErrorCode.
func (ec ErrorCode) String() string {
	switch ec {
	case ErrorCodeSuccess:
		return "Success"
	case ErrorCodeGenericError:
		return "GenericError"
	case ErrorCodePanic:
		return "Panic"
	case ErrorCodeInvalidArgument:
		return "InvalidArgument"
	case ErrorCodeIoError:
		return "IoError"
	case ErrorCodeParsingError:
		return "ParsingError"
	case ErrorCodeOcrError:
		return "OcrError"
	case ErrorCodeMissingDependency:
		return "MissingDependency"
	default:
		return "Unknown"
	}
}

// PanicContext contains panic context information from kreuzberg-ffi.
type PanicContext struct {
	File         string `json:"file"`
	Line         int    `json:"line"`
	Function     string `json:"function"`
	Message      string `json:"message"`
	TimestampSec int64  `json:"timestamp_secs"`
}

// String returns a formatted string representation of PanicContext.
func (pc *PanicContext) String() string {
	if pc == nil {
		return ""
	}
	return fmt.Sprintf("%s:%d in %s: %s", pc.File, pc.Line, pc.Function, pc.Message)
}

// KreuzbergError is implemented by all custom error types returned by the Go binding.
type KreuzbergError interface {
	error
	Kind() ErrorKind
	PanicCtx() *PanicContext
}

type baseError struct {
	kind       ErrorKind
	message    string
	cause      error
	panicCtx   *PanicContext
	nativeCode ErrorCode
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

func (e *baseError) PanicCtx() *PanicContext {
	return e.panicCtx
}

// ErrorCodeFromError returns the error code associated with a Kreuzberg error.
func (e *baseError) ErrorCodeFromError() ErrorCode {
	return e.nativeCode
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

func makeBaseError(kind ErrorKind, message string, cause error, code ErrorCode, panicCtx *PanicContext) baseError {
	var msg string
	if panicCtx != nil {
		msg = formatErrorMessageWithCause(message, cause) + " [panic: " + panicCtx.String() + "]"
	} else {
		msg = formatErrorMessageWithCause(message, cause)
	}
	return baseError{
		kind:       kind,
		message:    msg,
		cause:      cause,
		panicCtx:   panicCtx,
		nativeCode: code,
	}
}

func newValidationErrorWithContext(message string, cause error, code ErrorCode, panicCtx *PanicContext) *ValidationError {
	return &ValidationError{baseError: makeBaseError(ErrorKindValidation, message, cause, code, panicCtx)}
}

func newParsingErrorWithContext(message string, cause error, code ErrorCode, panicCtx *PanicContext) *ParsingError {
	return &ParsingError{baseError: makeBaseError(ErrorKindParsing, message, cause, code, panicCtx)}
}

func newOCRErrorWithContext(message string, cause error, code ErrorCode, panicCtx *PanicContext) *OCRError {
	return &OCRError{baseError: makeBaseError(ErrorKindOCR, message, cause, code, panicCtx)}
}

func newCacheErrorWithContext(message string, cause error, code ErrorCode, panicCtx *PanicContext) *CacheError {
	return &CacheError{baseError: makeBaseError(ErrorKindCache, message, cause, code, panicCtx)}
}

func newImageProcessingErrorWithContext(message string, cause error, code ErrorCode, panicCtx *PanicContext) *ImageProcessingError {
	return &ImageProcessingError{baseError: makeBaseError(ErrorKindImageProcessing, message, cause, code, panicCtx)}
}

func newSerializationErrorWithContext(message string, cause error, code ErrorCode, panicCtx *PanicContext) *SerializationError {
	return &SerializationError{baseError: makeBaseError(ErrorKindSerialization, message, cause, code, panicCtx)}
}

func newMissingDependencyErrorWithContext(dependency string, message string, cause error, code ErrorCode, panicCtx *PanicContext) *MissingDependencyError {
	return &MissingDependencyError{
		baseError:  makeBaseError(ErrorKindMissingDependency, messageWithFallback(message, fmt.Sprintf("Missing dependency: %s", dependency)), cause, code, panicCtx),
		Dependency: dependency,
	}
}

func newPluginErrorWithContext(plugin string, message string, cause error, code ErrorCode, panicCtx *PanicContext) *PluginError {
	return &PluginError{
		baseError:  makeBaseError(ErrorKindPlugin, messageWithFallback(message, "Plugin error"), cause, code, panicCtx),
		PluginName: plugin,
	}
}

func newUnsupportedFormatErrorWithContext(format string, message string, cause error, code ErrorCode, panicCtx *PanicContext) *UnsupportedFormatError {
	return &UnsupportedFormatError{
		baseError: makeBaseError(ErrorKindUnsupportedFormat, messageWithFallback(message, fmt.Sprintf("Unsupported format: %s", format)), cause, code, panicCtx),
		Format:    format,
	}
}

func newIOErrorWithContext(message string, cause error, code ErrorCode, panicCtx *PanicContext) *IOError {
	return &IOError{baseError: makeBaseError(ErrorKindIO, message, cause, code, panicCtx)}
}

func newRuntimeErrorWithContext(message string, cause error, code ErrorCode, panicCtx *PanicContext) *RuntimeError {
	return &RuntimeError{baseError: makeBaseError(ErrorKindRuntime, message, cause, code, panicCtx)}
}

// Backward compatibility wrappers for error constructors without context.
// nolint:unused
func newValidationError(message string, cause error) *ValidationError {
	return newValidationErrorWithContext(message, cause, ErrorCodeInvalidArgument, nil)
}

// nolint:unused
func newParsingError(message string, cause error) *ParsingError {
	return newParsingErrorWithContext(message, cause, ErrorCodeParsingError, nil)
}

// nolint:unused
func newOCRError(message string, cause error) *OCRError {
	return newOCRErrorWithContext(message, cause, ErrorCodeOcrError, nil)
}

// nolint:unused
func newCacheError(message string, cause error) *CacheError {
	return newCacheErrorWithContext(message, cause, ErrorCodeGenericError, nil)
}

// nolint:unused
func newImageProcessingError(message string, cause error) *ImageProcessingError {
	return newImageProcessingErrorWithContext(message, cause, ErrorCodeGenericError, nil)
}

// nolint:unused
func newSerializationError(message string, cause error) *SerializationError {
	return newSerializationErrorWithContext(message, cause, ErrorCodeGenericError, nil)
}

// nolint:unused
func newMissingDependencyError(dependency string, message string, cause error) *MissingDependencyError {
	return newMissingDependencyErrorWithContext(dependency, message, cause, ErrorCodeMissingDependency, nil)
}

// nolint:unused
func newPluginError(plugin string, message string, cause error) *PluginError {
	return newPluginErrorWithContext(plugin, message, cause, ErrorCodeGenericError, nil)
}

// nolint:unused
func newUnsupportedFormatError(format string, message string, cause error) *UnsupportedFormatError {
	return newUnsupportedFormatErrorWithContext(format, message, cause, ErrorCodeGenericError, nil)
}

// nolint:unused
func newIOError(message string, cause error) *IOError {
	return newIOErrorWithContext(message, cause, ErrorCodeIoError, nil)
}

// nolint:unused
func newRuntimeError(message string, cause error) *RuntimeError {
	return newRuntimeErrorWithContext(message, cause, ErrorCodeGenericError, nil)
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

func classifyNativeError(message string, code ErrorCode, panicCtx *PanicContext) error {
	trimmed := strings.TrimSpace(message)
	if trimmed == "" {
		return newRuntimeErrorWithContext("unknown error", nil, code, panicCtx)
	}

	switch {
	case strings.HasPrefix(trimmed, "Validation error:"):
		return newValidationErrorWithContext(trimmed, nil, code, panicCtx)
	case strings.HasPrefix(trimmed, "Parsing error:"):
		return newParsingErrorWithContext(trimmed, nil, code, panicCtx)
	case strings.HasPrefix(trimmed, "OCR error:"):
		return newOCRErrorWithContext(trimmed, nil, code, panicCtx)
	case strings.HasPrefix(trimmed, "Cache error:"):
		return newCacheErrorWithContext(trimmed, nil, code, panicCtx)
	case strings.HasPrefix(trimmed, "Image processing error:"):
		return newImageProcessingErrorWithContext(trimmed, nil, code, panicCtx)
	case strings.HasPrefix(trimmed, "Serialization error:"):
		return newSerializationErrorWithContext(trimmed, nil, code, panicCtx)
	case strings.HasPrefix(trimmed, "Missing dependency:"):
		dependency := strings.TrimSpace(trimmed[len("Missing dependency:"):])
		return newMissingDependencyErrorWithContext(dependency, trimmed, nil, code, panicCtx)
	case strings.HasPrefix(trimmed, "Plugin error in "):
		plugin := parsePluginName(trimmed)
		return newPluginErrorWithContext(plugin, trimmed, nil, code, panicCtx)
	case strings.HasPrefix(trimmed, "Unsupported format:"):
		format := strings.TrimSpace(trimmed[len("Unsupported format:"):])
		return newUnsupportedFormatErrorWithContext(format, trimmed, nil, code, panicCtx)
	case strings.HasPrefix(trimmed, "IO error:"):
		return newIOErrorWithContext(trimmed, nil, code, panicCtx)
	case strings.HasPrefix(trimmed, "Lock poisoned:"):
		return newRuntimeErrorWithContext(trimmed, nil, code, panicCtx)
	case strings.HasPrefix(trimmed, "Unsupported operation:"):
		return newRuntimeErrorWithContext(trimmed, nil, code, panicCtx)
	}

	lower := strings.ToLower(trimmed)
	if strings.Contains(lower, "lock poisoned") {
		return newRuntimeErrorWithContext(trimmed, nil, code, panicCtx)
	}
	if strings.Contains(lower, "missing dependency") {
		return newMissingDependencyErrorWithContext("", trimmed, nil, code, panicCtx)
	}

	return newRuntimeErrorWithContext(trimmed, nil, code, panicCtx)
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
