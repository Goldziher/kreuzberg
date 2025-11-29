/*====================================================================*
 -  Copyright (C) 2001 Leptonica.  All rights reserved.
 -
 -  Redistribution and use in source and binary forms, with or without
 -  modification, are permitted provided that the following conditions
 -  are met:
 -  1. Redistributions of source code must retain the above copyright
 -     notice, this list of conditions and the following disclaimer.
 -  2. Redistributions in binary form must reproduce the above
 -     copyright notice, this list of conditions and the following
 -     disclaimer in the documentation and/or other materials
 -     provided with the distribution.
 -
 -  THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
 -  ``AS IS'' AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
 -  LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
 -  A PARTICULAR PURPOSE ARE DISCLAIMED.  IN NO EVENT SHALL ANY
 -  CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL,
 -  EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO,
 -  PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR
 -  PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY
 -  OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING
 -  NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
 -  SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 *====================================================================*/

/*
 * pixserial_reg.c
 *
 *    Tests the fast (uncompressed) serialization of pix to a string
 *    in memory and the deserialization back to a pix.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

    /* Use this set */
static l_int32  nfiles = 10;
static const char  *filename[] = {
                         "feyn.tif",         /* 1 bpp */
                         "dreyfus2.png",     /* 2 bpp cmapped */
                         "dreyfus4.png",     /* 4 bpp cmapped */
                         "weasel4.16c.png",  /* 4 bpp cmapped */
                         "dreyfus8.png",     /* 8 bpp cmapped */
                         "weasel8.240c.png", /* 8 bpp cmapped */
                         "karen8.jpg",       /* 8 bpp, not cmapped */
                         "test16.tif",       /* 8 bpp, not cmapped */
                         "marge.jpg",        /* rgb */
                         "test24.jpg"        /* rgb */
                            };

int main(int    argc,
         char **argv)
{
char          buf[256];
size_t        size;
l_int32       i, w, h;
l_int32       format, bps, spp, iscmap, format2, w2, h2, bps2, spp2, iscmap2;
l_uint8      *data;
l_uint32     *data32, *data32r;
BOX          *box;
PIX          *pixs, *pixt, *pixt2, *pixd;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

            /* Test basic serialization/deserialization */
    data32 = NULL;
    for (i = 0; i < nfiles; i++) {
        pixs = pixRead(filename[i]);
            /* Serialize to memory */
        pixSerializeToMemory(pixs, &data32, &size);
            /* Just for fun, write and read back from file */
        l_binaryWrite("/tmp/lept/regout/array", "w", data32, size);
        data32r = (l_uint32 *)l_binaryRead("/tmp/lept/regout/array", &size);
            /* Deserialize */
        pixd = pixDeserializeFromMemory(data32r, size);
        regTestComparePix(rp, pixs, pixd);  /* i */
        pixDestroy(&pixd);
        pixDestroy(&pixs);
        lept_free(data32);
        lept_free(data32r);
    }

            /* Test read/write fileio interface */
    for (i = 0; i < nfiles; i++) {
        pixs = pixRead(filename[i]);
        pixGetDimensions(pixs, &w, &h, NULL);
        box = boxCreate(0, 0, L_MIN(150, w), L_MIN(150, h));
        pixt = pixClipRectangle(pixs, box, NULL);
        boxDestroy(&box);
        snprintf(buf, sizeof(buf), "/tmp/lept/regout/pixs.%d.spix",
                 rp->index + 1);
        pixWrite(buf, pixt, IFF_SPIX);
        regTestCheckFile(rp, buf);  /* nfiles + 2 * i */
        pixt2 = pixRead(buf);
        regTestComparePix(rp, pixt, pixt2);  /* nfiles + 2 * i + 1 */
        pixDestroy(&pixs);
        pixDestroy(&pixt);
        pixDestroy(&pixt2);
    }

            /* Test read header.  Note that for rgb input, spp = 3,
             * but for 32 bpp spix, we set spp = 4. */
    data = NULL;
    for (i = 0; i < nfiles; i++) {
        pixs = pixRead(filename[i]);
        pixWriteMem(&data, &size, pixs, IFF_SPIX);
        pixReadHeader(filename[i], &format, &w, &h, &bps, &spp, &iscmap);
        pixReadHeaderMem(data, size, &format2, &w2, &h2, &bps2,
                         &spp2, &iscmap2);
        if (format2 != IFF_SPIX || w != w2 || h != h2 || bps != bps2 ||
            iscmap != iscmap2) {
            if (rp->fp)
                fprintf(rp->fp, "Failure comparing data");
            else
                lept_stderr("Failure comparing data");
        }
        pixDestroy(&pixs);
        lept_free(data);
    }

#if 0
        /* Do timing */
    for (i = 0; i < nfiles; i++) {
        pixs = pixRead(filename[i]);
        startTimer();
        pixSerializeToMemory(pixs, &data32, &size);
        pixd = pixDeserializeFromMemory(data32, size);
        lept_stderr("Time for %s: %7.3f sec\n", filename[i], stopTimer());
        lept_free(data32);
        pixDestroy(&pixs);
        pixDestroy(&pixd);
    }
#endif

    return regTestCleanup(rp);
}
