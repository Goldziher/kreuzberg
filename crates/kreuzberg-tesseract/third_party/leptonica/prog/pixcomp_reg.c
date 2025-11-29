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
 * pixcomp_reg.c
 *
 *    Regression test for compressed pix and compressed pix arrays
 *    in memory.
 *
 *    We also show some other ways to accumulate and display pixa.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include <math.h>
#include "allheaders.h"
#include "pix_internal.h"

static const char *fnames[] = {"weasel32.png", "weasel2.4c.png",
                               "weasel4.16c.png", "weasel4.8g.png",
                               "weasel8.149g.png", "weasel8.16g.png"};

LEPT_DLL extern const char *ImageFileFormatExtensions[];
static void get_format_data(l_int32 i, l_uint8 *data, size_t size);

int main(int    argc,
         char **argv)
{
l_uint8      *data1, *data2;
l_int32       i;
size_t        size1, size2;
BOX          *box;
PIX          *pix, *pix1, *pix2, *pix3;
PIXA         *pixa, *pixa1;
PIXC         *pixc, *pixc1, *pixc2;
PIXAC        *pixac, *pixac1, *pixac2;
L_REGPARAMS  *rp;
SARRAY       *sa;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    lept_mkdir("lept/comp");

    pixac = pixacompCreate(1);
    pixa = pixaCreate(0);

        /* --- Read in the images --- */
    pix1 = pixRead("marge.jpg");
    pixc1 = pixcompCreateFromPix(pix1, IFF_JFIF_JPEG);
    pix2 = pixCreateFromPixcomp(pixc1);
    pixc2 = pixcompCreateFromPix(pix2, IFF_JFIF_JPEG);
    pix3 = pixCreateFromPixcomp(pixc2);
    regTestWritePixAndCheck(rp, pix3, IFF_JFIF_JPEG);  /* 0 */
    pixaAddPix(pixa, pix3, L_INSERT);
    pixacompAddPix(pixac, pix1, IFF_DEFAULT);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixcompDestroy(&pixc1);
    pixcompDestroy(&pixc2);

    pix = pixRead("feyn.tif");
    pix1 = pixScaleToGray6(pix);
    pixc1 = pixcompCreateFromPix(pix1, IFF_JFIF_JPEG);
    pix2 = pixCreateFromPixcomp(pixc1);
    pixc2 = pixcompCreateFromPix(pix2, IFF_JFIF_JPEG);
    pix3 = pixCreateFromPixcomp(pixc2);
    regTestWritePixAndCheck(rp, pix3, IFF_JFIF_JPEG);  /* 1 */
    pixaAddPix(pixa, pix3, L_INSERT);
    pixacompAddPix(pixac, pix1, IFF_DEFAULT);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixcompDestroy(&pixc1);
    pixcompDestroy(&pixc2);

    box = boxCreate(1144, 611, 690, 180);
    pix1 = pixClipRectangle(pix, box, NULL);
    pixc1 = pixcompCreateFromPix(pix1, IFF_TIFF_G4);
    pix2 = pixCreateFromPixcomp(pixc1);
    pixc2 = pixcompCreateFromPix(pix2, IFF_TIFF_G4);
    pix3 = pixCreateFromPixcomp(pixc2);
    regTestWritePixAndCheck(rp, pix3, IFF_TIFF_G4);  /* 2 */
    pixaAddPix(pixa, pix3, L_INSERT);
    pixacompAddPix(pixac, pix1, IFF_DEFAULT);
    boxDestroy(&box);
    pixDestroy(&pix);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixcompDestroy(&pixc1);
    pixcompDestroy(&pixc2);

    pix1 = pixRead("weasel4.11c.png");
    pixc1 = pixcompCreateFromPix(pix1, IFF_PNG);
    pix2 = pixCreateFromPixcomp(pixc1);
    pixc2 = pixcompCreateFromPix(pix2, IFF_PNG);
    pix3 = pixCreateFromPixcomp(pixc2);
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 3 */
    pixaAddPix(pixa, pix3, L_INSERT);
    pixacompAddPix(pixac, pix1, IFF_DEFAULT);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixcompDestroy(&pixc1);
    pixcompDestroy(&pixc2);

        /* --- Extract formatting info from compressed strings --- */
    for (i = 0; i < 4; i++) {
        pixc = pixacompGetPixcomp(pixac, i, L_NOCOPY);
        get_format_data(i, pixc->data, pixc->size);
    }

        /* Save a tiled composite from the pixa */
    pix1 = pixaDisplayTiledAndScaled(pixa, 32, 400, 4, 0, 20, 2);
    regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 4 */
    pixaDestroy(&pixa);
    pixDestroy(&pix1);

        /* Convert the pixac --> pixa and save a tiled composite */
    pixa1 = pixaCreateFromPixacomp(pixac, L_COPY);
    pix1 = pixaDisplayTiledAndScaled(pixa1, 32, 400, 4, 0, 20, 2);
    regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 5 */
    pixaDestroy(&pixa1);
    pixDestroy(&pix1);

        /* Make a pixacomp from files, and join */
    sa = sarrayCreate(0);
    for (i = 0; i < 6; i++)
        sarrayAddString(sa, fnames[i], L_COPY);
    pixac1 = pixacompCreateFromSA(sa, IFF_DEFAULT);
    pixacompJoin(pixac1, pixac, 0, -1);
    pixa1 = pixaCreateFromPixacomp(pixac1, L_COPY);
    pix1 = pixaDisplayTiledAndScaled(pixa1, 32, 250, 10, 0, 20, 2);
    regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 6 */
    pixacompDestroy(&pixac1);
    pixaDestroy(&pixa1);
    pixDestroy(&pix1);
    sarrayDestroy(&sa);

        /* Test serialized I/O */
    pixacompWrite("/tmp/lept/comp/file1.pac", pixac);
    regTestCheckFile(rp, "/tmp/lept/comp/file1.pac");  /* 7 */
    pixac1 = pixacompRead("/tmp/lept/comp/file1.pac");
    pixacompWrite("/tmp/lept/comp/file2.pac", pixac1);
    regTestCheckFile(rp, "/tmp/lept/comp/file2.pac");  /* 8 */
    regTestCompareFiles(rp, 7, 8);  /* 9 */
    pixac2 = pixacompRead("/tmp/lept/comp/file2.pac");
    pixa1 = pixaCreateFromPixacomp(pixac2, L_COPY);
    pix1 = pixaDisplayTiledAndScaled(pixa1, 32, 250, 4, 0, 20, 2);
    regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 10 */
    pixacompDestroy(&pixac1);
    pixacompDestroy(&pixac2);
    pixaDestroy(&pixa1);
    pixDestroy(&pix1);

        /* Test serialized pixacomp I/O to and from memory */
    pixacompWriteMem(&data1, &size1, pixac);
    pixac1 = pixacompReadMem(data1, size1);
    pixacompWriteMem(&data2, &size2, pixac1);
    pixac2 = pixacompReadMem(data2, size2);
    pixacompWrite("/tmp/lept/comp/file3.pac", pixac1);
    regTestCheckFile(rp, "/tmp/lept/comp/file3.pac");  /* 11 */
    pixacompWrite("/tmp/lept/comp/file4.pac", pixac2);
    regTestCheckFile(rp, "/tmp/lept/comp/file4.pac");  /* 12 */
    regTestCompareFiles(rp, 11, 12);  /* 13 */
    pixacompDestroy(&pixac1);
    pixacompDestroy(&pixac2);
    lept_free(data1);
    lept_free(data2);

        /* Test pdf generation (both with and without transcoding */
    pixacompDestroy(&pixac);
    pix1 = pixRead("test24.jpg");
    pix2 = pixRead("marge.jpg");
    pixac = pixacompCreate(2);
    pixacompAddPix(pixac, pix1, IFF_JFIF_JPEG);
    pixacompAddPix(pixac, pix2, IFF_JFIF_JPEG);
    l_pdfSetDateAndVersion(0);
    pixacompConvertToPdfData(pixac, 0, 1.0, L_DEFAULT_ENCODE, 0, "test1",
                             &data1, &size1);
    regTestWriteDataAndCheck(rp, data1, size1, "pdf");  /* 13 */
    pixacompFastConvertToPdfData(pixac, "test2", &data2, &size2);
    regTestWriteDataAndCheck(rp, data2, size2, "pdf");  /* 14 */
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixacompDestroy(&pixac);
    lept_free(data1);
    lept_free(data2);

    return regTestCleanup(rp);
}


static void
get_format_data(l_int32   i,
                l_uint8  *data,
                size_t    size)
{
l_int32  ret, format, w, h, d, bps, spp, iscmap;

    ret = pixReadHeaderMem(data, size, &format, &w, &h, &bps, &spp, &iscmap);
    d = bps * spp;
    if (d == 24) d = 32;
    if (ret)
        lept_stderr("Error: couldn't read data: size = %d\n", (l_int32)size);
    else
        lept_stderr("Format data for image %d:\n"
                    "  format: %s, size (w, h, d) = (%d, %d, %d)\n"
                    "  bps = %d, spp = %d, iscmap = %d\n",
                    i, ImageFileFormatExtensions[format], w, h, d,
                    bps, spp, iscmap);
    return;
}

