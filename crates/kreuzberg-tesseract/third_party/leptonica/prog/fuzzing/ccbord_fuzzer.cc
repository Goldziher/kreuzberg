#include "leptfuzz.h"

extern "C" int
LLVMFuzzerTestOneInput(const uint8_t *data, size_t size)
{

    if(size<3) return 0;

    leptSetStdNullHandler();

    PIX *pixs, *pixc;
    CCBORDA *ccba;

    pixs = pixReadMemSpix(data, size);
    if(pixs==NULL) return 0;

    ccba = pixGetAllCCBorders(pixs);
    
    ccbaStepChainsToPixCoords(ccba, CCB_GLOBAL_COORDS);
    ccbaGenerateSPGlobalLocs(ccba, CCB_SAVE_TURNING_PTS);
    pixc = ccbaDisplayImage2(ccba);

    pixDestroy(&pixs);
    pixDestroy(&pixc);
    ccbaDestroy(&ccba);
    return 0;
}
