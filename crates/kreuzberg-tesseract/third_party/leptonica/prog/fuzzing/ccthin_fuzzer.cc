#include "leptfuzz.h"

extern "C" int
LLVMFuzzerTestOneInput(const uint8_t *data, size_t size)
{
    PIXA *pixa, *return_pixa;
    PIX *pixs;

    leptSetStdNullHandler();
    pixs = pixReadMemSpix(data, size);
    if(pixs==NULL) return 0;

    for(int i=0; i<10; i++) {
        pixa = pixaReadMem(data, size);
        return_pixa = pixaThinConnected(pixa, L_THIN_FG, i, i);
        pixaDestroy(&pixa);
        pixaDestroy(&return_pixa);

        pixa = pixaReadMem(data, size);
        return_pixa = pixaThinConnected(pixa, L_THIN_BG, i, i);
        pixaDestroy(&pixa);
        pixaDestroy(&return_pixa);       
    }

    pixaDestroy(&return_pixa);
    pixDestroy(&pixs);
    return 0;
}