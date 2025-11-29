#include "leptfuzz.h"

extern "C" int
LLVMFuzzerTestOneInput(const uint8_t* data, size_t size) { 
	if(size<3) return 0;

	leptSetStdNullHandler();

	PIX *pixs_payload = pixReadMemSpix(data, size);
	if(pixs_payload == NULL) return 0;

	PIX *pix1, *return_pix1, *pix_copy;
	PTA *ptas, *ptad;

	ptas = ptaCreate(0);
	ptad = ptaCreate(0);
	pix_copy = pixCopy(NULL, pixs_payload);
	return_pix1 = pixBilinearPta(pix_copy, ptad, ptas, L_BRING_IN_WHITE);
	pixDestroy(&pix_copy);
	pixDestroy(&return_pix1);
	ptaDestroy(&ptas);
	ptaDestroy(&ptad);

	pix1 = pixRead("../test8.jpg");
	ptas = ptaCreate(0);
	ptad = ptaCreate(0);
	pix_copy = pixCopy(NULL, pixs_payload);
	return_pix1 = pixBilinearPtaWithAlpha(pix_copy, ptad, ptas, pix1, 0.5, 2);
	pixDestroy(&pix_copy);
	pixDestroy(&pix1);
	pixDestroy(&return_pix1);
	ptaDestroy(&ptas);
	ptaDestroy(&ptad);

	pixDestroy(&pixs_payload);
	return 0;
}
