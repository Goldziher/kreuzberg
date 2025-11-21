package kreuzberg

/*
#cgo CFLAGS: -I${SRCDIR}/../../../crates/kreuzberg-ffi
#cgo LDFLAGS: -L${SRCDIR}/../../../target/release -L${SRCDIR}/../../../target/debug -lkreuzberg_ffi
#include "../../../crates/kreuzberg-ffi/kreuzberg.h"
#include <stdlib.h>
*/
import "C"

import (
	"encoding/json"
	"unsafe"
)

// RegisterOCRBackend exposes kresuzberg_register_ocr_backend to Go callers.
//
// `callback` must be a C-callable function pointer (typically produced via
// `//export` in a cgo file) that follows the OcrBackendCallback contract.
func RegisterOCRBackend(name string, callback C.OcrBackendCallback) error {
	if name == "" {
		return newValidationError("ocr backend name cannot be empty", nil)
	}
	if callback == nil {
		return newValidationError("ocr backend callback cannot be nil", nil)
	}

	cName := C.CString(name)
	defer C.free(unsafe.Pointer(cName))

	if ok := C.kreuzberg_register_ocr_backend(cName, callback); !bool(ok) {
		return lastError()
	}
	return nil
}

// RegisterPostProcessor registers a Go-defined post processor in the Rust pipeline.
//
// The callback must conform to PostProcessorCallback (typically defined via
// `//export`).
func RegisterPostProcessor(name string, priority int32, callback C.PostProcessorCallback) error {
	if name == "" {
		return newValidationError("post processor name cannot be empty", nil)
	}
	if callback == nil {
		return newValidationError("post processor callback cannot be nil", nil)
	}

	cName := C.CString(name)
	defer C.free(unsafe.Pointer(cName))

	if ok := C.kreuzberg_register_post_processor(cName, callback, C.int32_t(priority)); !bool(ok) {
		return lastError()
	}
	return nil
}

// UnregisterPostProcessor removes a previously registered post processor.
func UnregisterPostProcessor(name string) error {
	if name == "" {
		return newValidationError("post processor name cannot be empty", nil)
	}

	cName := C.CString(name)
	defer C.free(unsafe.Pointer(cName))

	if ok := C.kreuzberg_unregister_post_processor(cName); !bool(ok) {
		return lastError()
	}
	return nil
}

// RegisterValidator registers a Go-defined validator callback.
func RegisterValidator(name string, priority int32, callback C.ValidatorCallback) error {
	if name == "" {
		return newValidationError("validator name cannot be empty", nil)
	}
	if callback == nil {
		return newValidationError("validator callback cannot be nil", nil)
	}

	cName := C.CString(name)
	defer C.free(unsafe.Pointer(cName))

	if ok := C.kreuzberg_register_validator(cName, callback, C.int32_t(priority)); !bool(ok) {
		return lastError()
	}
	return nil
}

// UnregisterValidator deregisters a validator by name.
func UnregisterValidator(name string) error {
	if name == "" {
		return newValidationError("validator name cannot be empty", nil)
	}

	cName := C.CString(name)
	defer C.free(unsafe.Pointer(cName))

	if ok := C.kreuzberg_unregister_validator(cName); !bool(ok) {
		return lastError()
	}
	return nil
}

// ListValidators returns names of all registered validators.
func ListValidators() ([]string, error) {
	listPtr := C.kreuzberg_list_validators()
	if listPtr == nil {
		return []string{}, nil
	}
	defer C.kreuzberg_free_string(listPtr)

	jsonStr := C.GoString(listPtr)
	var validators []string
	if err := json.Unmarshal([]byte(jsonStr), &validators); err != nil {
		return nil, newSerializationError("failed to parse validators list", err)
	}
	return validators, nil
}

// ClearValidators removes all registered validators.
func ClearValidators() error {
	if ok := C.kreuzberg_clear_validators(); !bool(ok) {
		return lastError()
	}
	return nil
}

// ListPostProcessors returns names of all registered post-processors.
func ListPostProcessors() ([]string, error) {
	listPtr := C.kreuzberg_list_post_processors()
	if listPtr == nil {
		return []string{}, nil
	}
	defer C.kreuzberg_free_string(listPtr)

	jsonStr := C.GoString(listPtr)
	var processors []string
	if err := json.Unmarshal([]byte(jsonStr), &processors); err != nil {
		return nil, newSerializationError("failed to parse post-processors list", err)
	}
	return processors, nil
}

// ClearPostProcessors removes all registered post-processors.
func ClearPostProcessors() error {
	if ok := C.kreuzberg_clear_post_processors(); !bool(ok) {
		return lastError()
	}
	return nil
}

// UnregisterOCRBackend removes a registered OCR backend by name.
func UnregisterOCRBackend(name string) error {
	if name == "" {
		return newValidationError("ocr backend name cannot be empty", nil)
	}

	cName := C.CString(name)
	defer C.free(unsafe.Pointer(cName))

	if ok := C.kreuzberg_unregister_ocr_backend(cName); !bool(ok) {
		return lastError()
	}
	return nil
}

// ListOCRBackends returns names of all registered OCR backends.
func ListOCRBackends() ([]string, error) {
	listPtr := C.kreuzberg_list_ocr_backends()
	if listPtr == nil {
		return []string{}, nil
	}
	defer C.kreuzberg_free_string(listPtr)

	jsonStr := C.GoString(listPtr)
	var backends []string
	if err := json.Unmarshal([]byte(jsonStr), &backends); err != nil {
		return nil, newSerializationError("failed to parse OCR backends list", err)
	}
	return backends, nil
}

// ClearOCRBackends removes all registered OCR backends.
func ClearOCRBackends() error {
	if ok := C.kreuzberg_clear_ocr_backends(); !bool(ok) {
		return lastError()
	}
	return nil
}

// ListDocumentExtractors returns names of all registered document extractors.
func ListDocumentExtractors() ([]string, error) {
	listPtr := C.kreuzberg_list_document_extractors()
	if listPtr == nil {
		return []string{}, nil
	}
	defer C.kreuzberg_free_string(listPtr)

	jsonStr := C.GoString(listPtr)
	var extractors []string
	if err := json.Unmarshal([]byte(jsonStr), &extractors); err != nil {
		return nil, newSerializationError("failed to parse document extractors list", err)
	}
	return extractors, nil
}

// UnregisterDocumentExtractor removes a registered document extractor by name.
func UnregisterDocumentExtractor(name string) error {
	if name == "" {
		return newValidationError("document extractor name cannot be empty", nil)
	}

	cName := C.CString(name)
	defer C.free(unsafe.Pointer(cName))

	if ok := C.kreuzberg_unregister_document_extractor(cName); !bool(ok) {
		return lastError()
	}
	return nil
}

// ClearDocumentExtractors removes all registered document extractors.
func ClearDocumentExtractors() error {
	if ok := C.kreuzberg_clear_document_extractors(); !bool(ok) {
		return lastError()
	}
	return nil
}
