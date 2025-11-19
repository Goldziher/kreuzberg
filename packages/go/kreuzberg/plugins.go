package kreuzberg

/*
#cgo CFLAGS: -I${SRCDIR}/../../../crates/kreuzberg-ffi
#cgo LDFLAGS: -L${SRCDIR}/../../../target/release -L${SRCDIR}/../../../target/debug -lkreuzberg_ffi
#include "../../../crates/kreuzberg-ffi/kreuzberg.h"
#include <stdlib.h>
*/
import "C"

import "unsafe"

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
