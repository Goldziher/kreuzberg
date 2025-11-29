#include "leptfuzz.h"

extern "C" int
LLVMFuzzerTestOneInput(const uint8_t *data, size_t size)
{
    BOX *box;
    PIX *pixs;
    PIX *pix1, *pix2, *pix3, *pix4;
    PIX *pix5, *pix52, *pix53, *pix54;
    PIX *pix6, *pix7, *pix8, *pix9;
    PIX *pix10, *pix11, *pix12;
    PIX *return_pix;
    PIXCMAP *cmap;

    leptSetStdNullHandler();
    pixs = pixReadMemSpix(data, size);
    if(pixs==NULL) return 0;

    pix1 = pixThresholdTo4bpp(pixs, 6, 1);
    box = boxCreate(120, 30, 200, 200);
    pixColorGray(pix1, box, L_PAINT_DARK, 220, 0, 0, 255);
    boxDestroy(&box);

    pix2 = pixScale(pix1, 1.5, 1.5);
    cmap = pixGetColormap(pix1);
    pix3 = pixOctcubeQuantFromCmap(pix2, cmap, 4,
                                   3, L_EUCLIDEAN_DISTANCE);

    pix4 = pixConvertTo32(pix3);
    pix5 = pixMedianCutQuant(pix4, 0);
    pix52 = pixMedianCutQuant(pix4, 1);
    pix53 = pixMedianCutQuant(pixs, 0);
    pix54 = pixMedianCutQuant(pixs, 1);

    pix6 = pixFewColorsMedianCutQuantMixed(pix4, 30, 30, 100, 0, 0, 0);

    pix7 = pixDeskew(pixs, 0);
    pixWriteImpliedFormat("/tmp/fuzzfile1", pix7, 0, 0);

    pix8 = pixOctreeQuantByPopulation(pixs, 0, 0);
    pix9 = pixFewColorsOctcubeQuantMixed(pix4, 3, 20, 244, 20, 0.05, 15);
    pix10 = pixColorSegment(pixs, 50, 6, 6, 6, 0);

    for(int i=128; i<257; i++){
        pix11 = pixOctreeColorQuant(pixs, i, 0);
        pixDestroy(&pix11);
        pix11 = pixOctreeColorQuant(pixs, i, 1);
        pixDestroy(&pix11);
    }

    pix12 = pixFixedOctcubeQuant256(pixs, 0);
    pixDestroy(&pix12);
    pix12 = pixFixedOctcubeQuant256(pixs, 1);
    pixDestroy(&pix12);

    for(int i1=0; i1<10; i1++){
        for(int i2=0; i2<10; i2++){
            return_pix = pixQuantFromCmap(pixs, pixGetColormap(pixs),
                                          i1, i2, L_MANHATTAN_DISTANCE);
            pixDestroy(&return_pix);

            return_pix = pixQuantFromCmap(pixs, pixGetColormap(pixs),
                                          i1, i2, L_EUCLIDEAN_DISTANCE);
            pixDestroy(&return_pix);
        }
    }
    
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);
    pixDestroy(&pix5);
    pixDestroy(&pix52);
    pixDestroy(&pix53);
    pixDestroy(&pix54);
    pixDestroy(&pix6);
    pixDestroy(&pix7);
    pixDestroy(&pix8);
    pixDestroy(&pix9);
    pixDestroy(&pix10);
    pixDestroy(&pixs);
    return 0;
}
