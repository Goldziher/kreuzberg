#include "leptfuzz.h"

extern "C" int
LLVMFuzzerTestOneInput(const uint8_t* data, size_t size) { 
    if(size<3) return 0;
 
    leptSetStdNullHandler();

    PIX *pixs_payload = pixReadMemSpix(data, size);
    if(pixs_payload == NULL) return 0;

    L_COLORFILL *cf = l_colorfillCreate(pixs_payload, 1, 1);
    l_colorfillDestroy(&cf);

    pixDestroy(&pixs_payload);
    return 0;
}
