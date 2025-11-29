#include "leptfuzz.h"

extern "C" int
LLVMFuzzerTestOneInput(const uint8_t* data, size_t size) { 
    if(size<3) return 0;
 
    leptSetStdNullHandler();

    PIX *pixs_payload = pixReadMemSpix(data, size);
    if(pixs_payload == NULL) return 0;

    l_float32 l_f1;
    l_float32 l_f2;
    l_float32 l_f3;
    pixMeasureEdgeSmoothness(pixs_payload, 1, 1, 1, &l_f1, &l_f2, &l_f3, NULL);
    pixDestroy(&pixs_payload);
    return 0;
}
