#include "leptfuzz.h"

extern "C" int
LLVMFuzzerTestOneInput(const uint8_t* data, size_t size) { 
	if(size<3) return 0;

	leptSetStdNullHandler();

	PIX *pixs_payload = pixReadMemSpix(data, size);
	if(pixs_payload == NULL) return 0;
	PIX *return_pix1, *pix_copy;

	pix_copy = pixCopy(NULL, pixs_payload);
	return_pix1 = pixBilateral(pix_copy,  5.0,  10.0, 10,  1);
	pixDestroy(&pix_copy);
	pixDestroy(&return_pix1);

	pix_copy = pixCopy(NULL, pixs_payload);
	return_pix1 = pixBlockBilateralExact(pixs_payload,  10.0, 1.0);
	pixDestroy(&pix_copy);
	pixDestroy(&return_pix1);

	pixDestroy(&pixs_payload);
	return 0;
}
