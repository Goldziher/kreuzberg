#include "leptfuzz.h"

extern "C" int
LLVMFuzzerTestOneInput(const uint8_t* data, size_t size) { 
	if(size<3) return 0;

	leptSetStdNullHandler();

	PIX *pixs_payload = pixReadMemSpix(data, size);
	if(pixs_payload == NULL) return 0;

	BOX *box1 = boxCreate(150, 130, 1500, 355);
	BOXA *boxa1;
	BOXAA *boxaa1;
	PIX *pix_copy = pixCopy(NULL, pixs_payload);
	pixFindWordAndCharacterBoxes(pix_copy, box1, 120, &boxa1, &boxaa1, NULL);
	boxDestroy(&box1);
	boxaDestroy(&boxa1);
	boxaaDestroy(&boxaa1);
	pixDestroy(&pix_copy);

	pixDestroy(&pixs_payload);
	return 0;
}
