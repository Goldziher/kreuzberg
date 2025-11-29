#include "leptfuzz.h"

extern "C" int
LLVMFuzzerTestOneInput(const uint8_t* data, size_t size) { 
    if(size<3) return 0;
 
    leptSetStdNullHandler();

    PIX *pixs_payload = pixReadMemSpix(data, size);
    if(pixs_payload == NULL) return 0;

    PTA *pta1 = ptaCreate(0);
    PIX *pix_return1 = pixFillPolygon(pixs_payload, pta1, 2, 2);
    ptaDestroy(&pta1);
    pixDestroy(&pix_return1);
    
    PTA  *pta_return1 =pixGeneratePtaBoundary(pixs_payload, 1);
    ptaDestroy(&pta_return1);
 
    BOX *box1 = boxCreate(150, 130, 1500, 355);
    pixRenderBox(pixs_payload, box1,  3,  200);
    boxDestroy(&box1);

    BOXA *boxa1;
    boxa1 = boxaCreate(0);
    pixRenderBoxa(pixs_payload, boxa1,  17,  200);
    boxaDestroy(&boxa1);

    boxa1 = boxaCreate(0);
    pixRenderBoxaBlend(pixs_payload, boxa1,  17,  200, 1,  25,  0.4,  1);
    boxaDestroy(&boxa1);

    PIX *pix_return12 = pixRenderContours(pixs_payload, 2, 4, 1);
    pixDestroy(&pix_return12);
    
    pixRenderGridArb(pixs_payload, 1, 1, 1, 1, 1, 1);
 
    BOX *box2 = boxCreate(150, 130, 1500, 355);
    pixRenderHashBox(pixs_payload, box2, 2, 1, 1, 0, L_SET_PIXELS);
    boxDestroy(&box2);
 
    BOX *box3 = boxCreate(150, 130, 1500, 355);
    pixRenderHashBoxBlend(pixs_payload, box3, 2, 1, L_HORIZONTAL_LINE, 0, 1, 1, 1, 1.0);
    boxDestroy(&box3);

    BOXA *boxa2;
    boxa2 = boxaCreate(1);
    pixRenderHashBoxa(pixs_payload, boxa2, 2, 1, L_HORIZONTAL_LINE, 0, L_SET_PIXELS);
    boxaDestroy(&boxa2);

    boxa1 = boxaCreate(1);
    pixRenderHashBoxaArb(pixs_payload, boxa1, 2, 1, L_HORIZONTAL_LINE, 0, 1, 1, 1);
    boxaDestroy(&boxa1);

    PIX *pixs = pixRead("../test8.jpg");
    pixRenderHashMaskArb(pixs_payload, pixs, 2, 2, 2, 1, L_HORIZONTAL_LINE, 0, 1, 1, 1);
    pixDestroy(&pixs);

    pixRenderLineBlend(pixs_payload,  30,  60,  440,  70,  5,  115,  200,  120,  0.3);
 
    PIX *pixs_payload2 = pixCopy(NULL, pixs_payload);
    NUMA *na2 = numaGammaTRC(1.7, 150, 255);
    pixRenderPlotFromNumaGen(&pixs_payload2, na2, L_HORIZONTAL_LINE,  3, 1,  80,  1, 1);
    numaDestroy(&na2);
    pixDestroy(&pixs_payload2);

    PTA *pta2 = ptaCreate(0);
    pixRenderPolylineArb(pixs_payload, pta2, 1, 1, 1, 1, 0);
    ptaDestroy(&pta2);

    PTA *pta3 = ptaCreate(0);
    pixRenderPolylineBlend(pixs_payload, pta3,  17,  25,  200, 1,  0.5,  1, 1);
    ptaDestroy(&pta3);

    NUMA *na1 = numaGammaTRC(1.7, 150, 255);
    pixRenderPlotFromNuma(&pixs_payload, na1, L_HORIZONTAL_LINE,  3, 1,  80);
    numaDestroy(&na1);

    pixDestroy(&pixs_payload);
    return 0;
}
