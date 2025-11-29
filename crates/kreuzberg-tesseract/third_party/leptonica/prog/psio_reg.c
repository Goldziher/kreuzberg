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
 * psio_reg.c
 *
 *   Tests writing of images in PS, with arbitrary scaling and
 *   translation, in the following formats:
 *
 *      - uncompressed
 *      - DCT compressed (jpeg for 8 bpp grayscale and RGB)
 *      - CCITT-G4 compressed (g4 fax compression for 1 bpp)
 *      - Flate compressed (gzip compression)
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

static const char *WeaselNames[] = {"weasel2.4c.png",
                                    "weasel2.4g.png",
                                    "weasel2.png",
                                    "weasel4.11c.png",
                                    "weasel4.8g.png",
                                    "weasel4.16g.png",
                                    "weasel8.16g.png",
                                    "weasel8.149g.png",
                                    "weasel8.240c.png",
                                    "weasel8.png",
                                    "weasel32.png"};
int main(int    argc,
         char **argv)
{
l_int32       i, w, h;
l_float32     factor, scale;
BOX          *box;
FILE         *fp1;
PIX          *pixs, *pixt;
PIXA         *pixa;
SARRAY       *sa;
L_REGPARAMS  *rp;

#if !defined(HAVE_LIBPNG)
    L_ERROR("This test requires libpng to run.\n", "psio_reg");
    exit(77);
#endif
#if !defined(HAVE_LIBJPEG)
    L_ERROR("This test requires libjpeg to run.\n", "psio_reg");
    exit(77);
#endif
#if !defined(HAVE_LIBTIFF)
    L_ERROR("This test requires libtiff to run.\n", "psio_reg");
    exit(77);
#endif

    if (regTestSetup(argc, argv, &rp))
        return 1;

#if !USE_PSIO
    lept_stderr("psio writing is not enabled\n"
                "See environ.h: #define USE_PSIO 1\n\n");
    regTestCleanup(rp);
    return 0;
#endif  /* abort */

    factor = 0.95;

        /* Uncompressed PS with scaling but centered on the page */
    pixs = pixRead("feyn-fract.tif");
    pixGetDimensions(pixs, &w, &h, NULL);
    scale = L_MIN(factor * 2550 / w, factor * 3300 / h);
    fp1 = lept_fopen("/tmp/lept/regout/psio0.ps", "wb+");
    pixWriteStreamPS(fp1, pixs, NULL, 300, scale);
    lept_fclose(fp1);
    regTestCheckFile(rp, "/tmp/lept/regout/psio0.ps");  /* 0 */
    pixDestroy(&pixs);

        /* Uncompressed PS with scaling, with LL corner at (1500, 1500) mils */
    pixs = pixRead("weasel4.11c.png");
    pixGetDimensions(pixs, &w, &h, NULL);
    scale = L_MIN(factor * 2550 / w, factor * 3300 / h);
    box = boxCreate(1500, 1500, (l_int32)(1000 * scale * w / 300),
                    (l_int32)(1000 * scale * h / 300));
    fp1 = lept_fopen("/tmp/lept/regout/psio1.ps", "wb+");
    pixWriteStreamPS(fp1, pixs, box, 300, 1.0);
    lept_fclose(fp1);
    regTestCheckFile(rp, "/tmp/lept/regout/psio1.ps");  /* 1 */
    boxDestroy(&box);
    pixDestroy(&pixs);

        /* DCT compressed PS with LL corner at (300, 1000) pixels */
    pixs = pixRead("marge.jpg");
    pixt = pixConvertTo32(pixs);
    pixWrite("/tmp/lept/regout/psio2.jpg", pixt, IFF_JFIF_JPEG);
    convertJpegToPS("/tmp/lept/regout/psio2.jpg", "/tmp/lept/regout/psio3.ps",
                    "w", 300, 1000, 0, 4.0, 1, 1);
    regTestCheckFile(rp, "/tmp/lept/regout/psio2.jpg");  /* 2 */
    regTestCheckFile(rp, "/tmp/lept/regout/psio3.ps");  /* 3 */
    pixDestroy(&pixt);
    pixDestroy(&pixs);

        /* For each page, apply tiff g4 image first; then jpeg or png over it */
    convertG4ToPS("feyn.tif", "/tmp/lept/regout/psio4.ps", "w",
                  0, 0, 0, 1.0, 1, 1, 0);
    convertJpegToPS("marge.jpg", "/tmp/lept/regout/psio4.ps",
                    "a", 500, 100, 300, 2.0, 1,  0);
    convertFlateToPS("weasel4.11c.png", "/tmp/lept/regout/psio4.ps",
                     "a", 300, 400, 300, 6.0, 1,  0);
    convertJpegToPS("marge.jpg", "/tmp/lept/regout/psio4.ps",
                    "a", 100, 800, 300, 1.5, 1, 1);

    convertG4ToPS("feyn.tif", "/tmp/lept/regout/psio4.ps",
                  "a", 0, 0, 0, 1.0, 2, 1, 0);
    convertJpegToPS("marge.jpg", "/tmp/lept/regout/psio4.ps",
                    "a", 1000, 700, 300, 2.0, 2, 0);
    convertJpegToPS("marge.jpg", "/tmp/lept/regout/psio4.ps",
                    "a", 100, 200, 300, 2.0, 2, 1);

    convertG4ToPS("feyn.tif", "/tmp/lept/regout/psio4.ps",
                  "a", 0, 0, 0, 1.0, 3, 1, 0);
    convertJpegToPS("marge.jpg", "/tmp/lept/regout/psio4.ps",
                    "a", 200, 200, 300, 2.0, 3, 0);
    convertJpegToPS("marge.jpg", "/tmp/lept/regout/psio4.ps",
                    "a", 200, 900, 300, 2.0, 3, 1);
    regTestCheckFile(rp, "/tmp/lept/regout/psio4.ps");  /* 4 */

        /* Now apply jpeg first; then paint through a g4 mask.
         * For gv, the first image with a b.b. determines the
         * window size for the canvas, so we put down the largest
         * image first.  If we had rendered a small image first,
         * gv and evince will not show the entire page.  However, after
         * conversion to pdf, everything works fine, regardless of the
         * order in which images are placed into the PS.  That is
         * because the pdf interpreter is robust to bad hints, ignoring
         * the page hints and computing the bounding box from the
         * set of images rendered on the page.
         *
         * Concatenate several pages, with colormapped png, color
         * jpeg and tiffg4 images (with the g4 image acting as a mask
         * that we're painting black through.  If the text layer
         * is painted first, the following images occlude it; otherwise,
         * the images remain in the background of the text. */
    pixs = pixRead("wyom.jpg");
    pixt = pixScaleToSize(pixs, 2528, 3300);
    pixWrite("/tmp/lept/regout/psio5.jpg", pixt, IFF_JFIF_JPEG);
    pixDestroy(&pixs);
    pixDestroy(&pixt);
    convertJpegToPS("/tmp/lept/regout/psio5.jpg", "/tmp/lept/regout/psio5.ps",
                      "w", 0, 0, 300, 1.0, 1, 0);
    convertFlateToPS("weasel8.240c.png", "/tmp/lept/regout/psio5.ps",
                     "a", 100, 100, 300, 5.0, 1, 0);
    convertFlateToPS("weasel8.149g.png", "/tmp/lept/regout/psio5.ps",
                     "a", 200, 300, 300, 5.0, 1, 0);
    convertFlateToPS("weasel4.11c.png", "/tmp/lept/regout/psio5.ps",
                     "a", 300, 500, 300, 5.0, 1, 0);
    convertG4ToPS("feyn.tif", "/tmp/lept/regout/psio5.ps",
                  "a", 0, 0, 0, 1.0, 1, 1, 1);

    convertJpegToPS("marge.jpg", "/tmp/lept/regout/psio5.ps",
                    "a", 500, 100, 300, 2.0, 2,  0);
    convertFlateToPS("weasel4.11c.png", "/tmp/lept/regout/psio5.ps",
                     "a", 300, 400, 300, 6.0, 2,  0);
    convertJpegToPS("marge.jpg", "/tmp/lept/regout/psio5.ps",
                    "a", 100, 800, 300, 1.5, 2, 0);
    convertG4ToPS("feyn.tif", "/tmp/lept/regout/psio5.ps",
                  "a", 0, 0, 0, 1.0, 2, 1, 1);

    convertJpegToPS("marge.jpg", "/tmp/lept/regout/psio5.ps",
                    "a", 500, 100, 300, 2.0, 3,  0);
    convertJpegToPS("marge.jpg", "/tmp/lept/regout/psio5.ps",
                    "a", 100, 800, 300, 2.0, 3, 0);
    convertG4ToPS("feyn.tif", "/tmp/lept/regout/psio5.ps",
                  "a", 0, 0, 0, 1.0, 3, 1, 1);

    convertJpegToPS("marge.jpg", "/tmp/lept/regout/psio5.ps",
                    "a", 700, 700, 300, 2.0, 4, 0);
    convertFlateToPS("weasel8.149g.png", "/tmp/lept/regout/psio5.ps",
                     "a", 400, 400, 300, 5.0, 4, 0);
    convertG4ToPS("feyn.tif", "/tmp/lept/regout/psio5.ps",
                  "a", 0, 0, 0, 1.0, 4, 1, 0);
    convertFlateToPS("weasel8.240c.png", "/tmp/lept/regout/psio5.ps",
                     "a", 100, 220, 300, 5.0, 4, 0);
    convertJpegToPS("marge.jpg", "/tmp/lept/regout/psio5.ps",
                    "a", 100, 200, 300, 2.0, 4, 1);

    convertJpegToPS("marge.jpg", "/tmp/lept/regout/psio5.ps",
                    "a", 200, 200, 300, 1.5, 5, 0);
    convertFlateToPS("weasel8.240c.png", "/tmp/lept/regout/psio5.ps",
                     "a", 140, 80, 300, 7.0, 5, 0);
    convertG4ToPS("feyn.tif", "/tmp/lept/regout/psio5.ps",
                  "a", 0, 0, 0, 1.0, 5, 1, 0);
    convertFlateToPS("weasel8.149g.png", "/tmp/lept/regout/psio5.ps",
                     "a", 280, 310, 300, 5.0, 4, 0);
    convertJpegToPS("marge.jpg", "/tmp/lept/regout/psio5.ps",
                    "a", 200, 900, 300, 2.0, 5, 1);
    regTestCheckFile(rp, "/tmp/lept/regout/psio5.ps");  /* 5 */

        /* Generation using segmentation masks */
    convertSegmentedPagesToPS(".", "lion-page", 10, ".", "lion-mask", 10,
                              0, 100, 2.0, 0.8, 190,
                              "/tmp/lept/regout/psio6.ps");
    regTestCheckFile(rp, "/tmp/lept/regout/psio6.ps");  /* 6 */

        /* PS generation for embeddding */
    convertJpegToPSEmbed("tetons.jpg", "/tmp/lept/regout/psio7.ps");
    regTestCheckFile(rp, "/tmp/lept/regout/psio7.ps");  /* 7 */

    convertG4ToPSEmbed("feyn-fract.tif", "/tmp/lept/regout/psio8.ps");
    regTestCheckFile(rp, "/tmp/lept/regout/psio8.ps");  /* 8 */

    convertFlateToPSEmbed("weasel8.240c.png", "/tmp/lept/regout/psio9.ps");
    regTestCheckFile(rp, "/tmp/lept/regout/psio9.ps");  /* 9 */

        /* Writing compressed from a pixa */
    sa = sarrayCreate(0);
    for (i = 0; i < 11; i++)
        sarrayAddString(sa, WeaselNames[i], L_COPY);
    pixa = pixaReadFilesSA(sa);
    pixaWriteCompressedToPS(pixa, "/tmp/lept/regout/psio10.ps", 0, 3);
    regTestCheckFile(rp, "/tmp/lept/regout/psio10.ps");  /* 10 */
    pixaDestroy(&pixa);
    sarrayDestroy(&sa);

    return regTestCleanup(rp);
}
