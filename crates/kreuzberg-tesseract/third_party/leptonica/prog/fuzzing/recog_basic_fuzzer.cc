#include "leptfuzz.h"
#include <sys/types.h>
#include <unistd.h>

extern "C" int
LLVMFuzzerTestOneInput(const uint8_t *data, size_t size)
{
    leptSetStdNullHandler();

    L_RECOG  *recog;
    char filename[256];
    sprintf(filename, "/tmp/libfuzzer.%d", getppid());

    FILE *fp = fopen(filename, "wb");
    if (!fp)
                return 0;
    fwrite(data, size, 1, fp);
    fclose(fp);

    recog = recogRead(filename);

    recogDestroy(&recog);
    unlink(filename);
    return 0;
}
