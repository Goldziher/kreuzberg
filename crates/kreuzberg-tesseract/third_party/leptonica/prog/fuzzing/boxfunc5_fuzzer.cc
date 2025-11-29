#include "leptfuzz.h"

extern "C" int
LLVMFuzzerTestOneInput(const uint8_t* data, size_t size) { 
	if(size<3) return 0;
 
	leptSetStdNullHandler();

	BOXA *boxa_payload, *boxa1;
	boxa_payload = boxaReadMem(data, size);
	if(boxa_payload == NULL) return 0;

	l_float32  fvarp, fvarm, devw, devh;
	l_float32  del_evenodd, rms_even, rms_odd, rms_all;
	l_int32    isame;

	boxa1 = boxaReconcileAllByMedian(boxa_payload,
					 L_ADJUST_LEFT_AND_RIGHT,
					 L_ADJUST_TOP_AND_BOT, 50,
					 0, NULL);
	boxaDestroy(&boxa1);
	
	boxa1 = boxaReconcileSidesByMedian(boxa_payload, L_ADJUST_LEFT, 80,
					   40, NULL);
	boxaDestroy(&boxa1);

	boxa1 = boxaReconcilePairWidth(boxa_payload, 2,
				       L_ADJUST_CHOOSE_MIN,
				       0.5, NULL);
	boxaDestroy(&boxa1);

	boxaSizeConsistency(boxa_payload, L_CHECK_HEIGHT,
			    0.0, 0.0, &fvarp, &fvarm, &isame);

	boxaSizeVariation(boxa_payload, L_SELECT_WIDTH, &del_evenodd,
                  	  &rms_even, &rms_odd, &rms_all);

	boxa1 = boxaSmoothSequenceMedian(boxa_payload, 10,
					 L_SUB_ON_LOC_DIFF,
					 80, 20, 1);
	boxaDestroy(&boxa1);
	boxaDestroy(&boxa_payload);
	return 0;
}
