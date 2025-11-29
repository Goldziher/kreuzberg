#include "leptfuzz.h"

extern "C" int
LLVMFuzzerTestOneInput(const uint8_t *data, size_t size)
{
    if(size<5) return 0;
    PIX *pixs, *pix1, *pix2, *pix3;

    leptSetStdNullHandler();

    pixs = pixReadMemSpix(data, size);
    if(pixs==NULL) return 0;
    
    pix1 = pixConvertGrayToFalseColor(pixs, 1.0);
    pix2 = pixThreshold8(pixs, 1, 0, 0);
    pixQuantizeIfFewColors(pixs, 8, 0, 1, &pix3);

    pixDestroy(&pixs);
    if (pix1!=NULL) pixDestroy(&pix1);
    if (pix2!=NULL) pixDestroy(&pix2);
    if (pix3!=NULL) pixDestroy(&pix3);
    return 0;
}
