/* Common include file for Leptonica fuzzers. */

#ifndef  LEPTFUZZ_H
#define  LEPTFUZZ_H

#include "allheaders.h"

static void send_to_devnull(const char *) {}

/* Suppress Leptonica error messages during fuzzing. */
static void leptSetStdNullHandler()
{
  leptSetStderrHandler(send_to_devnull);
}

#endif /* LEPTFUZZ_H */
