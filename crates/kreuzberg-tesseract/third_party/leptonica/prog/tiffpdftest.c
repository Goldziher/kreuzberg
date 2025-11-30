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
 * tiffpdftest.c
 *
 *   Generates pdf wrappers for tiff images, with both min_is_black
 *   and min_is_white.  Demonstrates that multiple cycles using
 *   pdftoppm preserve photometry.
 *
 *   Note: this test requires poppler pdf utilities, so it cannot
 *   be part of the alltests_reg regression test suite.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
         char **argv)
{
char       buf[256];
l_float32  fract;
PIX       *pix1, *pix2, *pix3, *pix4;
PIXA      *pixa1, *pixa2, *pixa3;

#if !defined(HAVE_LIBTIFF)
    L_ERROR("This test requires libtiff to run.\n", "tiffpdf_reg");
    exit(77);
#endif

    setLeptDebugOK(1);
    l_pdfSetDateAndVersion(0);
    lept_mkdir("lept/tiffpdf");

        /* Wrap min-is-white and min-is-black */
    pix1 = pixRead("miniswhite.tif");
    pix2 = pixRead("minisblack.tif");
    pixCompareBinary(pix1, pix2, L_COMPARE_XOR, &fract, NULL);
    lept_stderr("Compare input: %5.3f percent different\n", fract);
    pixa1 = pixaCreate(2);
    pixaAddPix(pixa1, pix1, L_INSERT);
    pixaAddPix(pixa1, pix2, L_INSERT);
    lept_stderr("Writing /tmp/lept/tiffpdf/set1.pdf\n");
    pixaConvertToPdf(pixa1, 100, 1.0, L_G4_ENCODE, 0, NULL,
        "/tmp/lept/tiffpdf/set1.pdf");

        /* Extract the images */
    lept_rmdir("lept/tmp");
    lept_mkdir("lept/tmp");
    snprintf(buf, sizeof(buf),
             "pdftoppm -r 300 /tmp/lept/tiffpdf/set1.pdf /tmp/lept/tmp/sevens");
    callSystemDebug(buf);

        /* Re-wrap them */
    pix1 = pixRead("/tmp/lept/tmp/sevens-1.ppm");
    pix2 = pixRead("/tmp/lept/tmp/sevens-2.ppm");
    pix3 = pixConvertTo1(pix1, 160);
    pix4 = pixConvertTo1(pix2, 160);
    pixCompareBinary(pix3, pix4, L_COMPARE_XOR, &fract, NULL);
    lept_stderr("Compare after first extraction: "
                "%5.3f percent different\n", fract);
    pixa2 = pixaCreate(2);
    pixaAddPix(pixa2, pix3, L_INSERT);
    pixaAddPix(pixa2, pix4, L_INSERT);
    lept_stderr("Writing /tmp/lept/tiffpdf/set2.pdf\n");
    pixaConvertToPdf(pixa2, 300, 1.0, L_G4_ENCODE, 0, NULL,
        "/tmp/lept/tiffpdf/set2.pdf");
    pixDestroy(&pix1);
    pixDestroy(&pix2);

        /* Extract the images again */
    lept_rmdir("lept/tmp");
    lept_mkdir("lept/tmp");
    snprintf(buf, sizeof(buf),
             "pdftoppm -r 300 /tmp/lept/tiffpdf/set2.pdf /tmp/lept/tmp/sevens");
    callSystemDebug(buf);

        /* And wrap them up again */
    pix1 = pixRead("/tmp/lept/tmp/sevens-1.ppm");
    pix2 = pixRead("/tmp/lept/tmp/sevens-2.ppm");
    pix3 = pixConvertTo1(pix1, 160);
    pix4 = pixConvertTo1(pix2, 160);
    pixCompareBinary(pix3, pix4, L_COMPARE_XOR, &fract, NULL);
    lept_stderr("Compare after second extraction: "
                "%5.3f percent different\n", fract);
    pixa3 = pixaCreate(2);
    pixaAddPix(pixa3, pix3, L_INSERT);
    pixaAddPix(pixa3, pix4, L_INSERT);
    lept_stderr("Writing /tmp/lept/tiffpdf/set3.pdf\n");
    pixaConvertToPdf(pixa3, 300, 1.0, L_G4_ENCODE, 0, NULL,
        "/tmp/lept/tiffpdf/set3.pdf");
    pixDestroy(&pix1);
    pixDestroy(&pix2);

    pix1 = pixaGetPix(pixa2, 0, L_COPY);
    pix2 = pixaGetPix(pixa3, 0, L_COPY);
    pixCompareBinary(pix1, pix2, L_COMPARE_XOR, &fract, NULL);
    lept_stderr("Compare between first and second extraction: "
                "%5.3f percent different\n", fract);
    pixDestroy(&pix1);
    pixDestroy(&pix2);

        /* Show the six images */
    pixaJoin(pixa1, pixa2, 0, -1);
    pixaJoin(pixa1, pixa3, 0, -1);
    pix1 = pixaDisplayTiledInColumns(pixa1, 6, 1.0, 30, 2);
    pixDisplay(pix1, 100, 100);
    pixDestroy(&pix1);
    pixaDestroy(&pixa1);
    pixaDestroy(&pixa2);
    pixaDestroy(&pixa3);
    return 0;
}
