#include "leptfuzz.h"

extern "C" int
LLVMFuzzerTestOneInput(const uint8_t* data, size_t size) { 
	if(size<3) return 0;
 
	leptSetStdNullHandler();

	PIX *pixs_payload = pixReadMemSpix(data, size);
	if(pixs_payload == NULL) return 0;
	
	PIX *pix_pointer_payload, *return_pix;

	pix_pointer_payload = pixCopy(NULL, pixs_payload);
	return_pix = pixCloseGray3(pix_pointer_payload,  3,  1);
	pixDestroy(&pix_pointer_payload);
	pixDestroy(&return_pix);

	pix_pointer_payload = pixCopy(NULL, pixs_payload);
	return_pix = pixDilateGray3(pix_pointer_payload,  3,  1);
	pixDestroy(&pix_pointer_payload);
	pixDestroy(&return_pix);

	pix_pointer_payload = pixCopy(NULL, pixs_payload);
	return_pix = pixErodeGray3(pix_pointer_payload,  3,  1);
	pixDestroy(&pix_pointer_payload);
	pixDestroy(&return_pix);

	pix_pointer_payload = pixCopy(NULL, pixs_payload);
	return_pix = pixOpenGray3(pix_pointer_payload,  3,  1);
	pixDestroy(&pix_pointer_payload);
	pixDestroy(&return_pix);

	pixDestroy(&pixs_payload);
	return 0;
}
