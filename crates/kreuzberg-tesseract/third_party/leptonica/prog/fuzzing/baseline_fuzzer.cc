#include "leptfuzz.h"

extern "C" int
LLVMFuzzerTestOneInput(const uint8_t* data, size_t size) { 
    if(size<3) return 0;

    leptSetStdNullHandler();

    PIX *pixs_payload = pixReadMemSpix(data, size);
    if(pixs_payload == NULL) return 0;

    PIX *pix1 = pixDeskewLocal(pixs_payload, 10, 0, 0, 0.0, 0.0, 0.0);
    pixDestroy(&pix1);

    PTA *pta;
    PIXA *pixadb = pixaCreate(6);
    NUMA *na = pixFindBaselines(pixs_payload, &pta, pixadb);
    numaDestroy(&na);
    ptaDestroy(&pta);
    pixaDestroy(&pixadb);

    pixDestroy(&pixs_payload);
    return 0;
}
