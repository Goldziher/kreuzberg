#include "leptfuzz.h"

extern "C" int
LLVMFuzzerTestOneInput(const uint8_t* data, size_t size) { 
	if(size<3) return 0;
 
	leptSetStdNullHandler();

	PIX *pixs_payload = pixReadMemSpix(data, size);
	if(pixs_payload == NULL) return 0;
	
	l_float32 minupconf, minratio, conf1, upconf1, leftconf1;
	PIX *pix_pointer_payload, *return_pix;
	
	pix_pointer_payload = pixCopy(NULL, pixs_payload);
	pixMirrorDetect(pix_pointer_payload, &conf1, 0, 1);
	pixDestroy(&pix_pointer_payload);

	pix_pointer_payload = pixCopy(NULL, pixs_payload);
        minupconf = 0.0;
        minratio = 0.0;
	return_pix = pixOrientCorrect(pix_pointer_payload, minupconf,
                                      minratio, NULL, NULL, NULL, 1);
	pixDestroy(&pix_pointer_payload);	
	pixDestroy(&return_pix);

	pix_pointer_payload = pixCopy(NULL, pixs_payload);
	pixOrientDetect(pix_pointer_payload, &upconf1, &leftconf1, 0, 0);
	pixDestroy(&pix_pointer_payload);

	pixDestroy(&pixs_payload);
	return 0;
}
