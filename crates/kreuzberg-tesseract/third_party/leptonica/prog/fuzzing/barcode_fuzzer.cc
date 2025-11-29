#include "leptfuzz.h"
#include "readbarcode.h"

extern "C" int
LLVMFuzzerTestOneInput(const uint8_t *data, size_t size)
{
    if(size<3) return 0;
    PIX *pixs;
    SARRAY *saw1, *sad1;

    leptSetStdNullHandler();

    pixs = pixReadMemSpix(data, size);
    if(pixs == NULL) return 0;

    sad1 = pixProcessBarcodes(pixs, L_BF_ANY, L_USE_WIDTHS, &saw1, 1);

    pixDestroy(&pixs);
    sarrayDestroy(&saw1);
    sarrayDestroy(&sad1);
    return 0;
}
