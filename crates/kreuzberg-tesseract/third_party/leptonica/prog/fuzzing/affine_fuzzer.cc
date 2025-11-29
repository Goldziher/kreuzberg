#include "leptfuzz.h"

extern "C" int
LLVMFuzzerTestOneInput(const uint8_t* data, size_t size) { 
	if(size<3) return 0;

	leptSetStdNullHandler();

	PIX *pixs_payload = pixReadMemSpix(data, size);
	if(pixs_payload == NULL) return 0;

	PIX *pix1, *return_pix1;
	PTA *ptas, *ptad;

	ptas = ptaCreate(0);
	ptad = ptaCreate(0);
	return_pix1 = pixAffinePta(pixs_payload, ptad, ptas, L_BRING_IN_WHITE);
	ptaDestroy(&ptas);
	ptaDestroy(&ptad);
	pixDestroy(&return_pix1);

	pix1 = pixRead("../test8.jpg");
	ptas = ptaCreate(0);
	ptad = ptaCreate(0);
	return_pix1 = pixAffinePtaWithAlpha(pixs_payload, ptad, ptas, pix1, 0.9, 1);
	pixDestroy(&pix1);
	ptaDestroy(&ptas);
	ptaDestroy(&ptad);
	pixDestroy(&return_pix1);

	ptas = ptaCreate(0);
	ptad = ptaCreate(0);
	return_pix1 = pixAffineSequential(pixs_payload, ptad, ptas, 3, 3);
	ptaDestroy(&ptas);
	ptaDestroy(&ptad);
	pixDestroy(&return_pix1);

	pixDestroy(&pixs_payload);
	return 0;
}
