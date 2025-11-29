#include "leptfuzz.h"

extern "C" int
LLVMFuzzerTestOneInput(const uint8_t *data, size_t size)
{
    if(size<3) return 0;

    PIX *pixs, *pixd;
    L_DEWARPA *dewa1;
    PIXAC *pixac;
    SARRAY *sa;

    leptSetStdNullHandler();

    pixs = pixReadMemSpix(data, size);
    if(pixs==NULL) return 0;
    
    // Don't use debug, because it requires writing to /tmp
    dewarpSinglePage(pixs, 0, 1, 1, 0, &pixd, NULL, 0);
	
    pixac = pixacompReadMem(data, size);
    dewa1 = dewarpaCreateFromPixacomp(pixac, 1, 0, 10, -1);
    
    dewarpaDestroy(&dewa1);
    pixacompDestroy(&pixac);
    pixDestroy(&pixs);
    pixDestroy(&pixd);
    return 0;
}
