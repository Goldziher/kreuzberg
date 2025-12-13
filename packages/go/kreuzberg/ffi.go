package kreuzberg

/*
#cgo !windows pkg-config: kreuzberg-ffi
#cgo !pkg-config CFLAGS: -I${SRCDIR}/internal/ffi
#cgo !pkg-config,!windows LDFLAGS: -lkreuzberg_ffi
#cgo !pkg-config,windows LDFLAGS: -lkreuzberg_ffi -lws2_32 -luserenv -lbcrypt -static-libgcc -static-libstdc++

#include "internal/ffi/kreuzberg.h"
#include <stdlib.h>
#include <stdint.h>
*/
import "C"
