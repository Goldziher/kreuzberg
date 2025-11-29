#include "leptfuzz.h"

extern "C" int
LLVMFuzzerTestOneInput(const uint8_t* data, size_t size) { 
	if(size<3) return 0;
 
	leptSetStdNullHandler();

	PIX *pixs_payload = pixReadMemSpix(data, size);
	if(pixs_payload == NULL) return 0;

	BOX *box1;
	PIX *pix_pointer_payload, *return_pix, *pix2;
	NUMA *return_numa;
	l_float32 l_f;
	l_int32 l_i;
	l_uint32 l_ui;
	BOXA *boxa1;

	box1 = boxCreate(150, 130, 1500, 355);
	pix_pointer_payload = pixCopy(NULL, pixs_payload);
	return_numa = pixAbsDiffByColumn(pix_pointer_payload, box1);
	pixDestroy(&pix_pointer_payload);
	boxDestroy(&box1);
        numaDestroy(&return_numa);

	box1 = boxCreate(150, 130, 1500, 355);
	pix_pointer_payload = pixCopy(NULL, pixs_payload);
	return_numa = pixAbsDiffByRow(pix_pointer_payload, box1);
	pixDestroy(&pix_pointer_payload);
	boxDestroy(&box1);
        numaDestroy(&return_numa);

	box1 = boxCreate(150, 130, 1500, 355);
	pix_pointer_payload = pixCopy(NULL, pixs_payload);
	pixAbsDiffInRect(pix_pointer_payload, box1, L_HORIZONTAL_LINE, &l_f);
	pixDestroy(&pix_pointer_payload);
	boxDestroy(&box1);

	pix_pointer_payload = pixCopy(NULL, pixs_payload);
	pixAbsDiffOnLine(pix_pointer_payload, 2, 2, 3, 3, &l_f);
	pixDestroy(&pix_pointer_payload);

	box1 = boxCreate(150, 130, 1500, 355);
	pix_pointer_payload = pixCopy(NULL, pixs_payload);
	return_numa = pixAverageByColumn(pix_pointer_payload, box1,
                                         L_BLACK_IS_MAX);
	boxDestroy(&box1);
	pixDestroy(&pix_pointer_payload);
	numaDestroy(&return_numa);
	
	box1 = boxCreate(150, 130, 1500, 355);
	return_numa = pixAverageByRow(pix_pointer_payload, box1,
                                      L_WHITE_IS_MAX);
	boxDestroy(&box1);
	pixDestroy(&pix_pointer_payload);
	numaDestroy(&return_numa);

	pix_pointer_payload = pixCopy(NULL, pixs_payload);
	pixAverageInRect(pix_pointer_payload, NULL, NULL, 0, 255, 1, &l_f);
	pixDestroy(&pix_pointer_payload);

	pix_pointer_payload = pixCopy(NULL, pixs_payload);
	pixAverageInRectRGB(pix_pointer_payload, NULL, NULL, 10, &l_ui);
	pixDestroy(&pix_pointer_payload);

	boxa1 = boxaCreate(0);
	pix_pointer_payload = pixCopy(NULL, pixs_payload);
	return_pix = pixCopyWithBoxa(pix_pointer_payload, boxa1, L_SET_WHITE);
	pixDestroy(&pix_pointer_payload);
	boxaDestroy(&boxa1);
	pixDestroy(&return_pix);
	
	for (int i = 0; i < 5; i++) {
            box1 = boxCreate(150, 130, 1500, 355);
	    pix_pointer_payload = pixCopy(NULL, pixs_payload);
	    pixCountArbInRect(pix_pointer_payload, box1, L_SET_WHITE, 2, &l_i);
	    pixDestroy(&pix_pointer_payload);
	    boxDestroy(&box1);
	}

	box1 = boxCreate(150, 130, 1500, 355);
	pix_pointer_payload = pixCopy(NULL, pixs_payload);
	return_numa = pixCountByRow(pix_pointer_payload, box1);
	pixDestroy(&pix_pointer_payload);
	boxDestroy(&box1);
	numaDestroy(&return_numa);
	
	box1 = boxCreate(150, 130, 1500, 355);
	pix_pointer_payload = pixCopy(NULL, pixs_payload);
	pixCountPixelsInRect(pix_pointer_payload, box1, &l_i, NULL);
	boxDestroy(&box1);
	pixDestroy(&pix_pointer_payload);

	pix_pointer_payload = pixCopy(NULL, pixs_payload);
	return_pix = pixMakeArbMaskFromRGB(pix_pointer_payload,
                                           -0.5, -0.5, 1.0, 20);
	pixDestroy(&pix_pointer_payload);
	pixDestroy(&return_pix);

	pix_pointer_payload = pixCopy(NULL, pixs_payload);
	for (int i = 0; i < 5; i++) {
            return_pix = pixMakeMaskFromVal(pix_pointer_payload, i);
	    pixDestroy(&pix_pointer_payload);
	    pixDestroy(&return_pix);
        }
    
	pix2 = pixRead("../test8.jpg");
	pix_pointer_payload = pixCopy(NULL, pixs_payload);
	pixPaintSelfThroughMask(pix_pointer_payload, pix2, 0, 0, L_HORIZ,
                                30, 50, 5, 10);
	pixDestroy(&pix2);
	pixDestroy(&pix_pointer_payload);
	
	pix_pointer_payload = pixCopy(NULL, pixs_payload);
	return_pix = pixSetUnderTransparency(pix_pointer_payload, 0, 0);
	pixDestroy(&pix_pointer_payload);
	pixDestroy(&return_pix);
	
	pix_pointer_payload = pixCopy(NULL, pixs_payload);
	return_numa = pixVarianceByColumn(pix2, NULL);
	pixDestroy(&pix_pointer_payload);
	numaDestroy(&return_numa);
	
	pix_pointer_payload = pixCopy(NULL, pixs_payload);
	return_numa = pixVarianceByRow(pix_pointer_payload, NULL);
	pixDestroy(&pix_pointer_payload);
	numaDestroy(&return_numa);

	box1 = boxCreate(150, 130, 1500, 355);
	pixVarianceInRect(pix_pointer_payload, box1, &l_f);
	boxDestroy(&box1);
	pixDestroy(&pix_pointer_payload);

	pixDestroy(&pixs_payload);
	return 0;
}
