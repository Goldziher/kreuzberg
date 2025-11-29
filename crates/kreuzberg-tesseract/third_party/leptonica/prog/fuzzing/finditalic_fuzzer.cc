#include "leptfuzz.h"

extern "C" int
LLVMFuzzerTestOneInput(const uint8_t* data, size_t size) { 
	if(size<3) return 0;
 
	leptSetStdNullHandler();

	PIX *pixs_payload = pixReadMemSpix(data, size);
	if(pixs_payload == NULL) return 0;

	BOXA *boxa1;
	pixItalicWords(pixs_payload, NULL, NULL, &boxa1, 1);

	pixDestroy(&pixs_payload);
	boxaDestroy(&boxa1);
	return 0;
}
