#include "leptfuzz.h"

extern "C" int
LLVMFuzzerTestOneInput(const uint8_t* data, size_t size) { 
    if(size<3) return 0;
 
    leptSetStdNullHandler();

    PIX *pixs_payload = pixReadMemSpix(data, size);
    if(pixs_payload == NULL) return 0;

    PIX *pix, *pix1, *pix_copy1, *pix_copy2, *pix_copy3, *pix_copy4;
    BOX *box1;
    SEL *sel;

    pix = pixRead("../feyn-fract.tif");
    box1 = boxCreate(507, 65, 60, 36);
    pix1 = pixClipRectangle(pix, box1, NULL);
    sel = selCreateFromPix(pix1, 6, 6, "plus_sign");
    pix_copy1 = pixCopy(NULL, pixs_payload);
    pixCloseGeneralized(pix_copy1, pix, sel);
    boxDestroy(&box1);
    pixDestroy(&pix_copy1);
    pixDestroy(&pix1);

    pix_copy2 = pixCopy(NULL, pixs_payload);
    pixCloseSafe(pix_copy2, pix, sel);
    pixDestroy(&pix_copy2);

    pix_copy3 = pixCopy(NULL, pixs_payload);
    pixOpenGeneralized(pix_copy3, pix, sel);
    pixDestroy(&pix_copy3);
    pixDestroy(&pix);
    selDestroy(&sel);

    for (l_int32 i = 0; i < 5; i++) {
        if ((sel = selCreate (i, i, "sel_5dp")) == NULL)
            continue;
        char *selname = selGetName(sel);
        pix_copy4 = pixCopy(NULL, pixs_payload);
        pixMorphDwa_1(pix_copy4, pix_copy4, i, selname);
        pixDestroy(&pix_copy4);
        selDestroy(&sel);
    }

    pixDestroy(&pixs_payload);
    return 0;
}
