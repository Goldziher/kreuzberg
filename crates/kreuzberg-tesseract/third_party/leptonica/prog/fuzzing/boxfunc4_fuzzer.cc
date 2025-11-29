#include "leptfuzz.h"

extern "C" int
LLVMFuzzerTestOneInput(const uint8_t* data, size_t size) { 
	if(size<3) return 0;
 
	leptSetStdNullHandler();

	BOXA *boxa_payload, *boxa1;
	boxa_payload = boxaReadMem(data, size);
	if(boxa_payload == NULL) return 0;

	PIX       *pix1;
	l_float32  fract;
	l_int32    minx, miny, maxx, maxy, changed;
	
	pix1 = boxaDisplayTiled(boxa_payload, NULL, 0, -1, 1500,
                                2, 1.0, 0, 3, 2);
	pixDestroy(&pix1);

	boxaGetCoverage(boxa_payload, 0, 0, 0, &fract);

	boxaLocationRange(boxa_payload, &minx, &miny, &maxx, &maxy);

	boxa1 = boxaPermutePseudorandom(boxa_payload);
	boxaDestroy(&boxa1);

	boxaPermuteRandom(boxa_payload, boxa_payload);

	boxa1 = boxaSelectByWHRatio(boxa_payload, 1, 
				    L_SELECT_IF_LT, &changed);
	boxaDestroy(&boxa1);

	boxa1 = boxaSelectRange(boxa_payload, 0, -1, L_COPY);
	boxaDestroy(&boxa1);

	boxaDestroy(&boxa_payload);
	return 0;
}
