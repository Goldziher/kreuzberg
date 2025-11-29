#include "leptfuzz.h"

PIX *
MakeReplacementMask(PIX  *pixs)
{
PIX  *pix1, *pix2, *pix3, *pix4;

    pix1 = pixMaskOverColorPixels(pixs, 95, 3);
    pix2 = pixMorphSequence(pix1, "o15.15", 0);
    pixSeedfillBinary(pix2, pix2, pix1, 8);
    pix3 = pixMorphSequence(pix2, "c15.15 + d61.31", 0);
    pix4 = pixRemoveBorderConnComps(pix3, 8);
    pixXor(pix4, pix4, pix3);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    return pix4;
}


extern "C" int
LLVMFuzzerTestOneInput(const uint8_t *data, size_t size)
{
    if(size<3) return 0;

    leptSetStdNullHandler();

    PIX *pixs, *pix1, *pix2;

    pixs = pixReadMemSpix(data, size);
    if(pixs==NULL) return 0;

    pix1 = MakeReplacementMask(pixs);
    pix2 = pixConvertTo8(pix1, FALSE);

    pixPaintSelfThroughMask(pix2, pix1, 0, 0, L_HORIZ, 30, 50, 5, 10);

    pixDestroy(&pixs);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    return 0;
}
