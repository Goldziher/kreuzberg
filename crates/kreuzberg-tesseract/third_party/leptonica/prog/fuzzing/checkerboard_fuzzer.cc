#include "leptfuzz.h"

extern "C" int
LLVMFuzzerTestOneInput(const uint8_t* data, size_t size) { 
	if(size<3) return 0;

	leptSetStdNullHandler();

	PIX *pixs_payload = pixReadMemSpix(data, size);
	if(pixs_payload == NULL) return 0;
	PIX     *pix1, *pix_copy;
	PIXA    *pixa1;
	PTA     *pta1;

	pixa1 = pixaCreate(0);
	pix_copy = pixCopy(NULL, pixs_payload);
	pixFindCheckerboardCorners(pix_copy, 15, 3, 2, &pix1, &pta1, pixa1);
	pixDestroy(&pix_copy);
	pixaDestroy(&pixa1);
	ptaDestroy(&pta1);
	pixDestroy(&pix1);

	pixDestroy(&pixs_payload);
	return 0;
}
