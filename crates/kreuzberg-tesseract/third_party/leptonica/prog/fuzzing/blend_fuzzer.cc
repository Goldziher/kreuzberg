#include "leptfuzz.h"

extern "C" int
LLVMFuzzerTestOneInput(const uint8_t* data, size_t size) { 
    if(size<3) return 0;

    leptSetStdNullHandler();

    PIX *pixs_payload = pixReadMemSpix(data, size);
    if(pixs_payload == NULL) return 0;

    PIX *pix1, *pix2, *return_pix, *pix_copy;

    for(int i=0; i<10; i++) {
        pix1 = pixRead("../test8.jpg");
        pix_copy = pixCopy(NULL, pixs_payload);
        return_pix = pixBlend(pix_copy, pix1, i, i, i);
        pixDestroy(&pix_copy);
        pixDestroy(&pix1);
        pixDestroy(&return_pix);
                        
        pix_copy = pixCopy(NULL, pixs_payload);
        return_pix = pixBlend(pix_copy, pix_copy, i, i, i);
        pixDestroy(&pix_copy);
        pixDestroy(&return_pix);
    }

    pix_copy = pixCopy(NULL, pixs_payload);
    return_pix = pixAddAlphaToBlend(pix_copy, 1.2, 1);
    pixDestroy(&pix_copy);
    pixDestroy(&return_pix);

    pix1 = pixRead("../test8.jpg");
    BOX *box1 = boxCreate(150, 130, 1500, 355);
    pix_copy = pixCopy(NULL, pixs_payload);
    pixBlendBackgroundToColor(pix_copy, pix1, box1, 123, 1.0, 5, 12);
    pixDestroy(&pix1);
    boxDestroy(&box1);
    pixDestroy(&pix_copy);

    pix1 = pixRead("../test8.jpg");
    pix_copy = pixCopy(NULL, pixs_payload);
    pixBlendCmap(pix_copy, pix1, 2, 3, 4);
    pixDestroy(&pix1);
    pixDestroy(&pix_copy);

    pix1 = pixRead("../test8.jpg");
    pix_copy = pixCopy(NULL, pixs_payload);
    pixBlendColorByChannel(pix_copy, pix_copy, pix1, 200, 200, 0.7, 0.8, 0.9, 1, 5);
    pixDestroy(&pix1);
    pixDestroy(&pix_copy);

    pix1 = pixRead("../test8.jpg");
    pix_copy = pixCopy(NULL, pixs_payload);
    pixBlendGrayAdapt(pix_copy, pix_copy, pix1, 2, 3, 0.8, 1);
    pixDestroy(&pix1);
    pixDestroy(&pix_copy);

    pix1 = pixRead("../test8.jpg");
    pix_copy = pixCopy(NULL, pixs_payload);
    pixBlendGrayInverse(pix_copy, pix_copy, pix1, 1, 2, 0.7);
    pixDestroy(&pix1);
    pixDestroy(&pix_copy);

    pix1 = pixRead("../test8.jpg");
    pix_copy = pixCopy(NULL, pixs_payload);
    pixBlendHardLight(pix_copy, pix_copy, pix1, 1, 2, 0.8);
    pixDestroy(&pix1);
    pixDestroy(&pix_copy);

    pix1 = pixRead("../test8.jpg");
    pix_copy = pixCopy(NULL, pixs_payload);
    return_pix = pixFadeWithGray(pix_copy, pix1, 1.0, L_BLEND_TO_WHITE);
    pixDestroy(&pix1);
    pixDestroy(&pix_copy);
    pixDestroy(&return_pix);

    pix_copy = pixCopy(NULL, pixs_payload);
    pixLinearEdgeFade(pix_copy, L_FROM_LEFT, L_BLEND_TO_WHITE, 1.0, 0.8);
    pixDestroy(&pix_copy);

    pix_copy = pixCopy(NULL, pixs_payload);
    pixMultiplyByColor(pix_copy, pix_copy, NULL, 2);
    pixDestroy(&pix_copy);

    pix_copy = pixCopy(NULL, pixs_payload);
    return_pix = pixSetAlphaOverWhite(pix_copy);
    pixDestroy(&pix_copy);
    pixDestroy(&return_pix);

    pixDestroy(&pixs_payload);
    return 0;
}
