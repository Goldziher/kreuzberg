// The fuzzer takes as input a buffer of bytes. The buffer is read in as:
// <angle>, <x_center>, <y_center>, and the remaining bytes will be read
// in as a <pix>. The image is then rotated by angle around the center. All
// inputs should not result in undefined behavior.
#include <cmath>
#include <cstddef>
#include <cstdint>
#include <cstdlib>
#include <cstring>
#include "leptfuzz.h"

// Set to true only for debugging; always false for production
static const bool DebugOutput = false;

namespace {

// Reads the front bytes of a data buffer containing `size` bytes as an int16_t,
// and advances the buffer forward [if there is sufficient capacity]. If there
// is insufficient capacity, this returns 0 and does not modify size or data.
int16_t ReadInt16(const uint8_t** data, size_t* size) {
  int16_t result = 0;
  if (*size >= sizeof(result)) {
    memcpy(&result, *data, sizeof(result));
    *data += sizeof(result);
    *size -= sizeof(result);
  }
  return result;
}

}  // namespace

extern "C" int LLVMFuzzerTestOneInput(const uint8_t* data, size_t size) {
  const int16_t angle = ReadInt16(&data, &size);
  const int16_t x_center = ReadInt16(&data, &size);
  const int16_t y_center = ReadInt16(&data, &size);

  leptSetStdNullHandler();

  // Don't do pnm format (which can cause timeouts) or
  // jpeg format (which can have uninitialized variables).
  // tiff and png can also take too much memory in this test,
  // but not in applications because they abort quickly after
  // reading the header and computing the size of the data.
  // Note that the format checker requires at least 12 bytes.
  if (size < 12) return EXIT_SUCCESS;
  int format;
  findFileFormatBuffer(data, &format);
  if (format == IFF_PNM || format == IFF_JFIF_JPEG ||
      format == IFF_TIFF || format == IFF_PNG) return EXIT_SUCCESS;

  Pix* pix = pixReadMem(reinterpret_cast<const unsigned char*>(data), size);
  if (pix == nullptr) {
    return EXIT_SUCCESS;
  }

  // Never in production
  if (DebugOutput) {
    L_INFO("w = %d, h = %d, d = %d\n", "fuzzer",
           pixGetWidth(pix), pixGetHeight(pix), pixGetDepth(pix));
  }

  constexpr float deg2rad = M_PI / 180.;
  Pix* pix_rotated = pixRotateShear(pix, x_center, y_center, deg2rad * angle,
                                    L_BRING_IN_WHITE);
  if (pix_rotated) {
    pixDestroy(&pix_rotated);
  }

  pixDestroy(&pix);
  return EXIT_SUCCESS;
}
