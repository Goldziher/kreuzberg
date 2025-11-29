#include "leptfuzz.h"

extern "C" int
LLVMFuzzerTestOneInput(const uint8_t* data, size_t size) { 
	if(size<3) return 0;

	leptSetStdNullHandler();

	PIX *pixs_payload = pixReadMemSpix(data, size);
	if(pixs_payload == NULL) return 0;

	PIX *pix1, *pix2, *return_pix, *pix_copy1;	
	l_int32 l_i;

	pix1 = pixRead("../test8.jpg");
	pix_copy1 = pixCopy(NULL, pixs_payload);
	return_pix = pixMaskedThreshOnBackgroundNorm(pix_copy1, pix1, 
						     100, 100, 10, 10, 
						     10, 10, 0.1, &l_i);
	pixDestroy(&pix1);
	pixDestroy(&pix_copy1);
	pixDestroy(&return_pix);

	pix1 = pixRead("../test8.jpg");
	pix_copy1 = pixCopy(NULL, pixs_payload);
	return_pix = pixOtsuThreshOnBackgroundNorm(pix_copy1, pix1, 
						   100, 100, 10, 10, 
						   130, 30, 30, 0.1, 
						   &l_i);
	pixDestroy(&pix1);
	pixDestroy(&pix_copy1);
	pixDestroy(&return_pix);

	pix_copy1 = pixCopy(NULL, pixs_payload);
	pixSauvolaBinarizeTiled(pix_copy1, 8, 0.34, 1, 1, NULL, &pix1);
	pixDestroy(&pix1);
	pixDestroy(&pix_copy1);

	pix1 = pixRead("../test8.jpg");
	pix_copy1 = pixCopy(NULL, pixs_payload);
	pixThresholdByConnComp(pix_copy1, pix1, 10, 10, 10, 5.5, 5.5, 
						   &l_i, &pix2, 1);
	pixDestroy(&pix1);
	pixDestroy(&pix2);
	pixDestroy(&pix_copy1);

	pix_copy1 = pixCopy(NULL, pixs_payload);
	NUMA *na1;
	l_int32 ival;
	pixThresholdByHisto(pix_copy1, 2, 0, 0, &ival, &pix1, &na1, &pix2);
	pixDestroy(&pix1);
	pixDestroy(&pix2);
	pixDestroy(&pix_copy1);
	numaDestroy(&na1);

	pixDestroy(&pixs_payload);
	return 0;
}
