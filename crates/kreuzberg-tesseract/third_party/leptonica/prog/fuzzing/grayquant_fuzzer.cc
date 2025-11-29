#include "leptfuzz.h"

extern "C" int
LLVMFuzzerTestOneInput(const uint8_t* data, size_t size) { 
	if(size<3) return 0;
 
	leptSetStdNullHandler();

	PIX *pixs_payload = pixReadMemSpix(data, size);
	if(pixs_payload == NULL) return 0;
	
	PIX *pix_pointer_payload, *return_pix;

	pix_pointer_payload = pixCopy(NULL, pixs_payload);
	return_pix = pixDitherTo2bpp(pix_pointer_payload,  1);
	pixDestroy(&pix_pointer_payload);
	pixDestroy(&return_pix);

	pix_pointer_payload = pixCopy(NULL, pixs_payload);
	return_pix = pixDitherToBinary(pix_pointer_payload);
	pixDestroy(&pix_pointer_payload);
	pixDestroy(&return_pix);

	pix_pointer_payload = pixCopy(NULL, pixs_payload);
	return_pix = pixGenerateMaskByBand(pix_pointer_payload, 1, 2,  1, 1);
	pixDestroy(&pix_pointer_payload);
	pixDestroy(&return_pix);

	pix_pointer_payload = pixCopy(NULL, pixs_payload);
	return_pix = pixGenerateMaskByBand32(pix_pointer_payload, 1, 1,  1, 0.0, 0.0);
	pixDestroy(&pix_pointer_payload);
	pixDestroy(&return_pix);

	pix_pointer_payload = pixCopy(NULL, pixs_payload);
	return_pix = pixGenerateMaskByDiscr32(pix_pointer_payload, 10, 10, L_MANHATTAN_DISTANCE);
	pixDestroy(&pix_pointer_payload);
	pixDestroy(&return_pix);

	const char *str = "45 75 115 185";
	pix_pointer_payload = pixCopy(NULL, pixs_payload);
	return_pix = pixThresholdGrayArb(pix_pointer_payload, str,  8,  0, 0, 0);
	pixDestroy(&pix_pointer_payload);
	pixDestroy(&return_pix);

	pixDestroy(&pixs_payload);
	return 0;
}
