```go title="Go"
package main

/*
#cgo CFLAGS: -I${SRCDIR}/../../../crates/kreuzberg-ffi
#cgo LDFLAGS: -L${SRCDIR}/../../../target/release -lkreuzberg_ffi
#include "../../../crates/kreuzberg-ffi/kreuzberg.h"
#include <stdlib.h>
*/
import "C"
import (
	"log"
	"unsafe"

	"github.com/kreuzberg-dev/kreuzberg/packages/go/kreuzberg"
)

//export customValidator
func customValidator(resultJSON *C.char) *C.char {
	// Inspect resultJSON, return error message or NULL
	return nil
}

func main() {
	if err := kreuzberg.RegisterValidator("go-validator", 50, (C.ValidatorCallback)(C.customValidator)); err != nil {
		log.Fatalf("register validator failed: %v", err)
	}

	result, err := kreuzberg.ExtractFileSync("document.pdf", nil)
	if err != nil {
		log.Fatalf("extract failed: %v", err)
	}
	log.Printf("Content length: %d", len(result.Content))
}
```
