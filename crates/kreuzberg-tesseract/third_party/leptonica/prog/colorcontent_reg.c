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
 *  colorcontent_reg.c
 *
 *   This tests various color content functions, including a simple
 *   color quantization method.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "string.h"
#include "allheaders.h"

l_int32 main(int    argc,
             char **argv)
{
//char         *fname[64];
char          fname[] = "/tmp/lept/colorcontent/maskgen.pdf";
l_uint32     *colors;
l_int32       ncolors, w, h;
l_float32     fcolor;
PIX          *pix1, *pix2, *pix3, *pix4, *pix5, *pix6, *pix7, *pix8;
PIXA         *pixa1;
L_REGPARAMS  *rp;

#if !defined(HAVE_LIBPNG)
    L_ERROR("This test requires libpng to run.\n", "colorcontent_reg");
    exit(77);
#endif

    if (regTestSetup(argc, argv, &rp))
        return 1;

        /* Find the most populated colors */
    pix1 = pixRead("fish24.jpg");
    pixGetMostPopulatedColors(pix1, 2, 3, 10, &colors, NULL);
    pix2 = pixDisplayColorArray(colors, 10, 200, 5, 6);
    pixDisplayWithTitle(pix2, 0, 0, NULL, rp->display);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 0 */
    lept_free(colors);
    pixDestroy(&pix2);

        /* Do a simple color quantization with sigbits = 2 */
    pix2 = pixSimpleColorQuantize(pix1, 2, 3, 10);
    pixDisplayWithTitle(pix2, 0, 400, NULL, rp->display);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 1 */
    pix3 = pixRemoveColormap(pix2, REMOVE_CMAP_TO_FULL_COLOR);
    regTestComparePix(rp, pix2, pix3);  /* 2 */
    pixNumColors(pix3, 1, &ncolors);
    regTestCompareValues(rp, ncolors, 10, 0.0);  /* 3 */
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);

        /* Do a simple color quantization with sigbits = 3.
         * Roundoff from different jpeg decompression algorithms can
         * result in differing numbers of colors by a few percent.  */
    pix1 = pixRead("wyom.jpg");
    pixNumColors(pix1, 1, &ncolors);  /* >255, so should give 0 */
    regTestCompareValues(rp, ncolors, 132165, 10000.0);  /* 4 */
    pix2 = pixSimpleColorQuantize(pix1, 3, 3, 20);
    pixDisplayWithTitle(pix2, 1000, 0, NULL, rp->display);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 5 */
    ncolors = pixcmapGetCount(pixGetColormap(pix2));
    regTestCompareValues(rp, ncolors, 20, 0.0);  /* 6 */
    pixDestroy(&pix1);
    pixDestroy(&pix2);

        /* Find the number of perceptually significant gray intensities */
    pix1 = pixRead("marge.jpg");
    pix2 = pixConvertTo8(pix1, 0);
    pixNumSignificantGrayColors(pix2, 20, 236, 0.0001, 1, &ncolors);
    regTestCompareValues(rp, ncolors, 219, 0.0);  /* 7 */
    pixDestroy(&pix1);
    pixDestroy(&pix2);

        /* Find background color in image with light color regions */
    pix1 = pixRead("map.057.jpg");
    pixa1 = pixaCreate(0);
    pixFindColorRegions(pix1, NULL, 4, 200, 70, 10, 90, 0.05,
                          &fcolor, &pix2, NULL, pixa1);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 8 */
    pix3 = pixaDisplayTiledInColumns(pixa1, 5, 0.3, 20, 2);
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 9 */
    pixDisplayWithTitle(pix3, 1000, 500, NULL, rp->display);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixaDestroy(&pixa1);

        /* Show binary classification of RGB colors using a plane */
    pix1 = pixMakeGamutRGB(3);
    pix2 = pixMakeArbMaskFromRGB(pix1, -0.5, -0.5, 1.0, 20);
    pixGetDimensions(pix1, &w, &h, NULL);
    pix3 = pixCreate(w, h, 32);
    pixSetAll(pix3);
    pixCombineMasked(pix3, pix1, pix2);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 10 */
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 11 */
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 12 */
    pixDisplayWithTitle(pix3, 0, 1300, NULL, rp->display);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);

        /* Show use of more than one plane to further restrict the
           allowed region of RGB color space */
    pixa1 = pixaCreate(0);
    pix1 = pixMakeGamutRGB(3);
    pix2 = pixMakeArbMaskFromRGB(pix1, -0.5, -0.5, 1.0, 20);
    pix3 = pixMakeArbMaskFromRGB(pix1, 1.5, -0.5, -1.0, 0);
    pix4 = pixMakeArbMaskFromRGB(pix1, 0.4, 0.3, 0.3, 60);
    pixInvert(pix4, pix4);
    pix5 = pixSubtract(NULL, pix2, pix3);
    pix6 = pixSubtract(NULL, pix5, pix4);
    pixGetDimensions(pix1, &w, &h, NULL);
    pix7 = pixCreate(w, h, 32);
    pixSetAll(pix7);
    pixCombineMasked(pix7, pix1, pix6);
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 13 */
    regTestWritePixAndCheck(rp, pix4, IFF_PNG);  /* 14 */
    regTestWritePixAndCheck(rp, pix5, IFF_PNG);  /* 15 */
    regTestWritePixAndCheck(rp, pix6, IFF_PNG);  /* 16 */
    regTestWritePixAndCheck(rp, pix7, IFF_PNG);  /* 17 */
    pixaAddPix(pixa1, pix1, L_INSERT);
    pixaAddPix(pixa1, pix2, L_INSERT);
    pixaAddPix(pixa1, pix3, L_INSERT);
    pixaAddPix(pixa1, pix4, L_INSERT);
    pixaAddPix(pixa1, pix5, L_INSERT);
    pixaAddPix(pixa1, pix6, L_INSERT);
    pixaAddPix(pixa1, pix7, L_INSERT);
    lept_mkdir("lept/colorcontent");
    l_pdfSetDateAndVersion(FALSE);
    pixaConvertToPdf(pixa1, 0, 0.5, L_FLATE_ENCODE, 0, NULL, fname);
    regTestCheckFile(rp, fname);  /* 18 */
    lept_stderr("Wrote %s\n", fname);
    if (rp->display) {
        pix8 = pixaDisplayTiledInColumns(pixa1, 2, 0.5, 15, 2);
        pixDisplay(pix8, 800, 1300);
        pixDestroy(&pix8);
    }
    pixaDestroy(&pixa1);

    return regTestCleanup(rp);
}
