#include "leptfuzz.h"

extern "C" int
LLVMFuzzerTestOneInput(const uint8_t* data, size_t size) { 
    if(size<3) return 0;

    leptSetStdNullHandler();

    PIX *pixs_payload = pixReadMemSpix(data, size);
    if(pixs_payload == NULL) return 0;

    PIX *ppixd;
    
    PTA *pta = pixSearchBinaryMaze(pixs_payload, 1, 2, 3, 4, &ppixd);
    pixDestroy(&ppixd);
    ptaDestroy(&pta);
    
    PTA *pta2 = pixSearchGrayMaze(pixs_payload, 1, 2, 3, 4, &ppixd);
    pixDestroy(&ppixd);
    ptaDestroy(&pta2);

    pixDestroy(&pixs_payload);
    return 0;
}
