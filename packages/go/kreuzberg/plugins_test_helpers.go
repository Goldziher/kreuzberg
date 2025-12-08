//go:build cgo

package kreuzberg

/*
#cgo CFLAGS: -I${SRCDIR}/../../../crates/kreuzberg-ffi
#cgo LDFLAGS: -L${SRCDIR}/../../../target/release -L${SRCDIR}/../../../target/debug
#include "../../../crates/kreuzberg-ffi/kreuzberg.h"
#include <stdlib.h>
#include <string.h>

char* kreuzberg_go_test_post_processor(const char* result_json) {
	if (result_json == NULL) {
		const char* fallback = "{\"content\":\"\"}";
		char* copy = (char*)malloc(strlen(fallback) + 1);
		strcpy(copy, fallback);
		return copy;
	}
	size_t len = strlen(result_json);
	char* copy = (char*)malloc(len + 1);
	memcpy(copy, result_json, len + 1);
	return copy;
}

char* kreuzberg_go_test_validator(const char* result_json) {
	(void)result_json;
	return NULL;
}

char* kreuzberg_go_test_ocr_backend(const uint8_t* image_bytes, uintptr_t image_length, const char* config_json) {
	(void)image_bytes;
	(void)image_length;
	(void)config_json;
	const char* response = "ocr text";
	char* copy = (char*)malloc(strlen(response) + 1);
	strcpy(copy, response);
	return copy;
}
*/
import "C"

var (
	testPostProcessorCallback = (C.PostProcessorCallback)(C.kreuzberg_go_test_post_processor)
	testValidatorCallback     = (C.ValidatorCallback)(C.kreuzberg_go_test_validator)
	testOcrBackendCallback    = (C.OcrBackendCallback)(C.kreuzberg_go_test_ocr_backend)
)
