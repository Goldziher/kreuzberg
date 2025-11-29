#include "leptfuzz.h"

extern "C" int
LLVMFuzzerTestOneInput(const uint8_t* data, size_t size) { 
    if(size<3) return 0;

    leptSetStdNullHandler();

    PIX *pixs_payload = pixReadMemSpix(data, size);
    if(pixs_payload == NULL) return 0;

    L_KERNEL *kel1 = kernelCreateFromPix(pixs_payload, 2, 2);
    pixDestroy(&pixs_payload);
    kernelDestroy(&kel1);
    return 0;
}
