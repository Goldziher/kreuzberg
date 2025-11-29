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
 * pdfseg_reg.c
 *
 *   Generates segmented images and encodes them efficiently in pdf.
 *   The encoding is mixed-raster, with the image parts encoded as
 *   DCT at one resolution and the non-image parts encoded at (typically)
 *   a higher resolution.
 *
 *   Uses 6 images, all segmented and scaled to a fixed width
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

    /* All images scaled to this width  */
static const l_int32  WIDTH = 800;

int main(int    argc,
         char **argv)
{
l_int32      h;
l_float32    scalefactor;
BOX         *box;
BOXA        *boxa1, *boxa2;
BOXAA       *baa;
PIX         *pix1, *pix2, *pix3, *pix4, *pix5, *pix6, *pix7, *pix8, *pix9;
L_REGPARAMS  *rp;

#if !defined(HAVE_LIBPNG)
    L_ERROR("This test requires libpng to run.\n", "pdfseg_reg");
    exit(77);
#endif
#if !defined(HAVE_LIBJPEG)
    L_ERROR("This test requires libjpeg to run.\n", "pdfseg_reg");
    exit(77);
#endif
#if !defined(HAVE_LIBTIFF)
    L_ERROR("This test requires libtiff to run.\n", "pdfseg_reg");
    exit(77);
#endif

    if (regTestSetup(argc, argv, &rp))
        return 1;

    lept_rmdir("lept/pdfseg");
    lept_mkdir("lept/pdfseg");
    baa = boxaaCreate(5);

        /* Image region input.  */
    pix1 = pixRead("wet-day.jpg");
    pix2 = pixScaleToSize(pix1, WIDTH, 0);
    pixWrite("/tmp/lept/pdfseg/0.jpg", pix2, IFF_JFIF_JPEG);
    regTestCheckFile(rp, "/tmp/lept/pdfseg/0.jpg");   /* 0 */
    box = boxCreate(105, 161, 620, 872);   /* image region */
    boxa1 = boxaCreate(1);
    boxaAddBox(boxa1, box, L_INSERT);
    boxaaAddBoxa(baa, boxa1, L_INSERT);
    pixDestroy(&pix1);
    pixDestroy(&pix2);

        /* Compute image region at w = 2 * WIDTH */
    pix1 = pixRead("candelabrum.011.jpg");
    pix2 = pixScaleToSize(pix1, WIDTH, 0);
    pix3 = pixConvertTo1(pix2, 100);
    pix4 = pixExpandBinaryPower2(pix3, 2);  /* w = 2 * WIDTH */
    pix5 = pixGenerateHalftoneMask(pix4, NULL, NULL, NULL);
    pix6 = pixMorphSequence(pix5, "c20.1 + c1.20", 0);
    pix7 = pixMaskConnComp(pix6, 8, &boxa1);
    pix8 = pixReduceBinary2(pix7, NULL);  /* back to w = WIDTH */
    pix9 = pixBackgroundNormSimple(pix2, pix8, NULL);
    pixWrite("/tmp/lept/pdfseg/1.jpg", pix9, IFF_JFIF_JPEG);
    regTestCheckFile(rp, "/tmp/lept/pdfseg/1.jpg");   /* 1 */
    boxa2 = boxaTransform(boxa1, 0, 0, 0.5, 0.5);  /* back to w = WIDTH */
    boxaaAddBoxa(baa, boxa2, L_INSERT);
    boxaDestroy(&boxa1);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);
    pixDestroy(&pix5);
    pixDestroy(&pix6);
    pixDestroy(&pix7);
    pixDestroy(&pix8);
    pixDestroy(&pix9);

        /* Use mask to find image region */
    pix1 = pixRead("lion-page.00016.jpg");
    pix2 = pixScaleToSize(pix1, WIDTH, 0);
    pixWrite("/tmp/lept/pdfseg/2.jpg", pix2, IFF_JFIF_JPEG);
    regTestCheckFile(rp, "/tmp/lept/pdfseg/2.jpg");   /* 2 */
    pix3 = pixRead("lion-mask.00016.tif");
    pix4 = pixScaleToSize(pix3, WIDTH, 0);
    boxa1 = pixConnComp(pix4, NULL, 8);
    boxaaAddBoxa(baa, boxa1, L_INSERT);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);

        /* Compute image region at full res */
    pix1 = pixRead("rabi.png");
    scalefactor = (l_float32)WIDTH / (l_float32)pixGetWidth(pix1);
    pix2 = pixScaleToGray(pix1, scalefactor);
    pixWrite("/tmp/lept/pdfseg/3.jpg", pix2, IFF_JFIF_JPEG);
    regTestCheckFile(rp, "/tmp/lept/pdfseg/3.jpg");   /* 3 */
    pix3 = pixGenerateHalftoneMask(pix1, NULL, NULL, NULL);
    pix4 = pixMorphSequence(pix3, "c20.1 + c1.20", 0);
    boxa1 = pixConnComp(pix4, NULL, 8);
    boxa2 = boxaTransform(boxa1, 0, 0, scalefactor, scalefactor);
    boxaaAddBoxa(baa, boxa2, L_INSERT);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);
    boxaDestroy(&boxa1);

        /* Page with no image regions */
    pix1 = pixRead("lucasta.047.jpg");
    pix2 = pixScaleToSize(pix1, WIDTH, 0);
    boxa1 = boxaCreate(1);
    pixWrite("/tmp/lept/pdfseg/4.jpg", pix2, IFF_JFIF_JPEG);
    regTestCheckFile(rp, "/tmp/lept/pdfseg/4.jpg");   /* 4 */
    boxaaAddBoxa(baa, boxa1, L_INSERT);
    pixDestroy(&pix1);
    pixDestroy(&pix2);

        /* Page that is all image */
    pix1 = pixRead("map1.jpg");
    pix2 = pixScaleToSize(pix1, WIDTH, 0);
    pixWrite("/tmp/lept/pdfseg/5.jpg", pix2, IFF_JFIF_JPEG);
    regTestCheckFile(rp, "/tmp/lept/pdfseg/5.jpg");   /* 5 */
    h = pixGetHeight(pix2);
    box = boxCreate(0, 0, WIDTH, h);
    boxa1 = boxaCreate(1);
    boxaAddBox(boxa1, box, L_INSERT);
    boxaaAddBoxa(baa, boxa1, L_INSERT);
    pixDestroy(&pix1);
    pixDestroy(&pix2);

        /* Save the boxaa file */
    boxaaWrite("/tmp/lept/pdfseg/images.baa", baa);
    regTestCheckFile(rp, "/tmp/lept/pdfseg/images.baa");   /* 6 */

        /* Do the conversion */
    l_pdfSetDateAndVersion(FALSE);
    convertSegmentedFilesToPdf("/tmp/lept/pdfseg", "jpg", 100, L_G4_ENCODE,
                               140, baa, 75, 0.6, "Segmentation Test",
                               "/tmp/lept/regout/pdfseg.7.pdf");
    L_INFO("Generated pdf file: /tmp/lept/regout/pdfseg.7.pdf\n", rp->testname);
    regTestCheckFile(rp, "/tmp/lept/regout/pdfseg.7.pdf");   /* 7 */

    boxaaDestroy(&baa);
    return regTestCleanup(rp);
}
