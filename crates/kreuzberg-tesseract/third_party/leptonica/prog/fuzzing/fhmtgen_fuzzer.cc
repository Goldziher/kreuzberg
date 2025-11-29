#include "leptfuzz.h"

extern "C" int
LLVMFuzzerTestOneInput(const uint8_t* data, size_t size) { 
	if(size<3) return 0;
 
	leptSetStdNullHandler();

	PIX *pixs_payload = pixReadMemSpix(data, size);
	if(pixs_payload == NULL) return 0;

	l_int32 i, nsels;
	char *selname;
	SEL *sel;
	SELA *sela;
	PIX *pix_pointer_payload, *return_pix;

	sela = selaAddHitMiss(NULL);
	nsels = selaGetCount(sela);
	pix_pointer_payload = pixCopy(NULL, pixs_payload);

	for (i = 0; i < nsels; i++) {
	    sel = selaGetSel(sela, i);
	    selname = selGetName(sel);
	    return_pix = pixHMTDwa_1(NULL, pix_pointer_payload, selname);
	    pixDestroy(&return_pix);
	}

	pixDestroy(&pix_pointer_payload);
	pixDestroy(&pixs_payload);
	selaDestroy(&sela);
	return 0;
}
