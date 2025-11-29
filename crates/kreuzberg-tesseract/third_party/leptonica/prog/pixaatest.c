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
 *  pixaatest.c
 *
 *     Syntax:  pixaatest
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"
#include "pix_internal.h"

static const l_int32  nx = 10;
static const l_int32  ny = 12;
static const l_int32  ncols = 3;

int main(int    argc,
         char **argv)
{
l_int32  w, d, tilewidth;
PIX     *pixs;
PIXA    *pixa, *pixad1, *pixad2;
PIXAA   *pixaa1, *pixaa2;

    if (argc != 1)
        return ERROR_INT(" Syntax: pixaatest", __func__, 1);

    setLeptDebugOK(1);
    lept_mkdir("/lept/paa");

        /* Read in file, split it into a set of tiles, and generate a pdf.
         * Two things to note for these tiny images:
         *  (1) If you use dct format (jpeg) for each image instead of
         *      flate (lossless), the quantization will be apparent.
         *  (2) If the resolution in pixaConvertToPdf() is above 50, and
         *      you add a red boundary, you will see errors in the boundary
         *      width.
         */
    pixs = pixRead("test24.jpg");
    pixGetDimensions(pixs, &w, NULL, &d);
    pixa = pixaSplitPix(pixs, nx, ny, 0, 0);
/*    pixa = pixaSplitPix(pixs, nx, ny, 2, 0xff000000);  */ /* red border */
    pixWrite("/tmp/lept/paa/pix0", pixa->pix[0], IFF_PNG);
    pixWrite("/tmp/lept/paa/pix9", pixa->pix[9], IFF_PNG);
    pixaConvertToPdf(pixa, 50, 1.0, 0, 95, "individual",
                     "/tmp/lept/paa/out1.pdf");

        /* Generate two pixaa by sampling the pixa, and write them to file */
    pixaa1 = pixaaCreateFromPixa(pixa, nx, L_CHOOSE_CONSECUTIVE, L_CLONE);
    pixaa2 = pixaaCreateFromPixa(pixa, nx, L_CHOOSE_SKIP_BY, L_CLONE);
    pixaaWrite("/tmp/lept/paa/pts1.paa", pixaa1);
    pixaaWrite("/tmp/lept/paa/pts2.paa", pixaa2);
    pixaDestroy(&pixa);
    pixaaDestroy(&pixaa1);
    pixaaDestroy(&pixaa2);

        /* Read each pixaa from file; tile/scale into a pixa */
    pixaa1 = pixaaRead("/tmp/lept/paa/pts1.paa");
    pixaa2 = pixaaRead("/tmp/lept/paa/pts2.paa");
    tilewidth = w / nx;
    pixad1 = pixaaDisplayTiledAndScaled(pixaa1, d, tilewidth, ncols, 0, 10, 0);
    pixad2 = pixaaDisplayTiledAndScaled(pixaa2, d, tilewidth, ncols, 0, 10, 0);

        /* Generate a pdf from each pixa */
    pixaConvertToPdf(pixad1, 50, 1.0, 0, 75, "consecutive",
                     "/tmp/lept/paa/out2.pdf");
    pixaConvertToPdf(pixad2, 50, 1.0, 0, 75, "skip_by",
                     "/tmp/lept/paa/out3.pdf");

        /* Write each pixa to a set of files, and generate a PS */
    pixaWriteFiles("/tmp/lept/paa/split1.", pixad1, IFF_JFIF_JPEG);
    pixaWriteFiles("/tmp/lept/paa/split2.", pixad2, IFF_JFIF_JPEG);
    convertFilesToPS("/tmp/lept/paa", "split1", 40, "/tmp/lept/paa/out1out1.ps");
    convertFilesToPS("/tmp/lept/paa", "split2", 40, "/tmp/lept/paa/out1out2.ps");

    pixDestroy(&pixs);
    pixaaDestroy(&pixaa1);
    pixaaDestroy(&pixaa2);
    pixaDestroy(&pixad1);
    pixaDestroy(&pixad2);
    return 0;
}
