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
 *  compare_reg.c
 *
 *   This tests comparison of images that are:
 *
 *   (1) translated with respect to each other
 *   (2) only slightly different in content
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "string.h"
#include "allheaders.h"

l_int32 main(int    argc,
             char **argv)
{
l_int32       delx, dely, etransx, etransy, w, h, area1, area2;
l_int32      *stab, *ctab;
l_float32     cx1, cy1, cx2, cy2, score, fract;
PIX          *pix0, *pix1, *pix2, *pix3, *pix4, *pix5;
L_REGPARAMS  *rp;

#if !defined(HAVE_LIBPNG)
    L_ERROR("This test requires libpng to run.\n", "compare_reg");
    exit(77);
#endif

    if (regTestSetup(argc, argv, &rp))
        return 1;

    /* ------------ Test of pixBestCorrelation() --------------- */
    pix0 = pixRead("harmoniam100-11.png");
    pix1 = pixConvertTo1(pix0, 160);
    pixGetDimensions(pix1, &w, &h, NULL);

        /* Now make a smaller image, translated by (-32, -12)
         * Except for the resizing, this is equivalent to
         *     pix2 = pixTranslate(NULL, pix1, -32, -12, L_BRING_IN_WHITE);  */
    pix2 = pixCreate(w - 10, h, 1);
    pixRasterop(pix2, 0, 0, w, h, PIX_SRC, pix1, 32, 12);

        /* Get the number of FG pixels and the centroid locations */
    stab = makePixelSumTab8();
    ctab = makePixelCentroidTab8();
    pixCountPixels(pix1, &area1, stab);
    pixCountPixels(pix2, &area2, stab);
    pixCentroid(pix1, ctab, stab, &cx1, &cy1);
    pixCentroid(pix2, ctab, stab, &cx2, &cy2);
    etransx = lept_roundftoi(cx1 - cx2);
    etransy = lept_roundftoi(cy1 - cy2);
    lept_stderr("delta cx = %d, delta cy = %d\n", etransx, etransy);

        /* Get the best correlation, searching around the translation
         * where the centroids coincide */
    pixBestCorrelation(pix1, pix2, area1, area2, etransx, etransy,
                       4, stab, &delx, &dely, &score, 5);
    lept_stderr("delx = %d, dely = %d, score = %7.4f\n", delx, dely, score);
    regTestCompareValues(rp, 32, delx, 0);   /* 0 */
    regTestCompareValues(rp, 12, dely, 0);   /* 1 */
    lept_mv("/tmp/lept/comp/correl_5.png", "lept/regout", NULL, NULL);
    regTestCheckFile(rp, "/tmp/lept/regout/correl_5.png");   /* 2 */
    lept_free(stab);
    lept_free(ctab);
    pixDestroy(&pix0);
    pixDestroy(&pix1);
    pixDestroy(&pix2);


    /* ------------ Test of pixCompareWithTranslation() ------------ */
        /* Now use the pyramid to get the result.  Do a translation
         * to remove pixels at the bottom from pix2, so that the
         * centroids are initially far apart. */
    pix1 = pixRead("harmoniam-11.tif");
    pix2 = pixTranslate(NULL, pix1, -45, 25, L_BRING_IN_WHITE);
    l_pdfSetDateAndVersion(0);
    pixCompareWithTranslation(pix1, pix2, 160, &delx, &dely, &score, 1);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    lept_stderr("delx = %d, dely = %d\n", delx, dely);
    regTestCompareValues(rp, 45, delx, 0);   /* 3 */
    regTestCompareValues(rp, -25, dely, 0);   /* 4 */
    lept_mv("/tmp/lept/comp/correl.pdf", "lept/regout", NULL, NULL);
    lept_mv("/tmp/lept/comp/compare.pdf", "lept/regout", NULL, NULL);
    regTestCheckFile(rp, "/tmp/lept/regout/compare.pdf");   /* 5 */
    regTestCheckFile(rp, "/tmp/lept/regout/correl.pdf");  /* 6 */

    /* ------------ Test of pixGetPerceptualDiff() --------------- */
    pix0 = pixRead("greencover.jpg");
    pix1 = pixRead("redcover.jpg");  /* pre-scaled to the same size */
        /* Apply directly to the color images */
    pixGetPerceptualDiff(pix0, pix1, 1, 3, 20, &fract, &pix2, &pix3);
    lept_stderr("Fraction of color pixels = %f\n", fract);
    regTestCompareValues(rp, 0.061252, fract, 0.01);  /* 7 */
    regTestWritePixAndCheck(rp, pix2, IFF_JFIF_JPEG);  /* 8 */
    regTestWritePixAndCheck(rp, pix3, IFF_TIFF_G4);  /* 9 */
    pixDestroy(&pix2);
    pixDestroy(&pix3);
        /* Apply to grayscale images */
    pix2 = pixConvertTo8(pix0, 0);
    pix3 = pixConvertTo8(pix1, 0);
    pixGetPerceptualDiff(pix2, pix3, 1, 3, 20, &fract, &pix4, &pix5);
    lept_stderr("Fraction of grayscale pixels = %f\n", fract);
    regTestCompareValues(rp, 0.046928, fract, 0.0002);  /* 10 */
    regTestWritePixAndCheck(rp, pix4, IFF_JFIF_JPEG);  /* 11 */
    regTestWritePixAndCheck(rp, pix5, IFF_TIFF_G4);  /* 12 */
    pixDestroy(&pix0);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);
    pixDestroy(&pix5);

    return regTestCleanup(rp);
}
