#include "leptfuzz.h"

extern "C" int
LLVMFuzzerTestOneInput(const uint8_t *data, size_t size)
{
    leptSetStdNullHandler();

    l_int32      score;
    PIX          *pixs;
    PIX          *pix1, *pix2, *pix3, *pix4;
    PIXA         *pixa1, *pixa2;
    PIXAC        *pixac;
    BOX          *box;
    BOXA         *boxa;

    pixs = pixReadMemSpix(data, size);
    if(pixs==NULL) return 0;

    pixa1 = pixaCreate(0);
    pixDecideIfTable(pixs, NULL, L_PORTRAIT_MODE, &score, pixa1);
    pixaDestroy(&pixa1);

    pixa1 = pixaCreate(0);
    pixGetRegionsBinary(pixs, &pix1, &pix2, &pix3, pixa1);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixaDestroy(&pixa1);

    pixac = pixacompReadMem(data, size);
    box = pixFindPageForeground(pixs, 170, 70, 30, 0, pixac);
    boxDestroy(&box);
    pixacompDestroy(&pixac);

    pixSplitIntoCharacters(pixs, 4, 4, &boxa, &pixa2, &pix4);
    boxaDestroy(&boxa);
    pixaDestroy(&pixa2);
    pixDestroy(&pix4);

    pixDestroy(&pixs);
    return 0;
}
