#include "leptfuzz.h"

extern "C" int
LLVMFuzzerTestOneInput(const uint8_t *data, size_t size)
{

    if(size<3) return 0;

    leptSetStdNullHandler();
    BOXA *boxa1, *boxa2;
    boxa1 = boxaReadMem(data, size);
    if(boxa1==NULL) return 0;

    boxa2 = boxaReconcileAllByMedian(boxa1, L_ADJUST_LEFT_AND_RIGHT,
                                     L_ADJUST_TOP_AND_BOT, 50, 0, NULL);
    if(boxa2!=NULL) boxaDestroy(&boxa2);

    boxa2 = boxaReconcileAllByMedian(boxa1, L_ADJUST_SKIP,
                                     L_ADJUST_TOP_AND_BOT, 50, 0, NULL);
    if(boxa2!=NULL) boxaDestroy(&boxa2);
    boxaDestroy(&boxa1);
    return 0;
}
