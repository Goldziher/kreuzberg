#include "leptfuzz.h"

extern "C" int
LLVMFuzzerTestOneInput(const uint8_t* data, size_t size) { 
	if(size<3) return 0;
 
	leptSetStdNullHandler();

	PIX *pixs_payload = pixReadMemSpix(data, size);
	if(pixs_payload == NULL) return 0;
	
	BOX *box1;
	PIX *pix_pointer_payload;
	
	box1 = boxCreate(278, 35, 122, 50);
	pix_pointer_payload = pixCopy(NULL, pixs_payload);
	pixSetSelectCmap(pix_pointer_payload, box1, 2, 255, 255, 100);
	boxDestroy(&box1);
	pixDestroy(&pix_pointer_payload);
	
	pix_pointer_payload = pixCopy(NULL, pixs_payload);
	pixSetSelectMaskedCmap(pix_pointer_payload, NULL, 1,  50,  0,  250, 249, 248);
	pixDestroy(&pix_pointer_payload);

	pixDestroy(&pixs_payload);
	return 0;
}
