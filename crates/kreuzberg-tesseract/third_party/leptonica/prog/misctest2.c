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
 *   misctest2.c
 *        * Page cropping with light filtering
 *        * Page cropping with removal of fg on left and right sides
 *        * Demonstrate image cleaning function
 *        * Demonstrat page cropping for 2-column, where one column is
 *          Kanji, and removing lots of junk on left and right sides.
 *        * Demonstrate page cropping with edgeclean = -2, for a situation
 *          where a bad oversized mediabox confuses the pdftoppm renderer,
 *          which embeds the page image in a larger black image.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
         char **argv)
{
char   buf[256];
PIX   *pix1, *pix2, *pix3;
PIXA  *pixa1;

    setLeptDebugOK(1);
    lept_mkdir("lept/misc");

        /* Page cropping with light filtering */
    pix1 = pixRead("tel_3.tif");
    pix2 = pixCropImage(pix1, 30, 30, 4, 25, 25, 1.15, 0,
                        "/tmp/lept/misc/crop_tel3.pdf", NULL);
    pixDestroy(&pix1);
    pixDestroy(&pix2);

        /* Page cropping with removal of fg on left and right sides */
    pix1 = pixRead("boismort.1.tif");
    pix2 = pixCropImage(pix1, 50, 50, -1, 70, 70, 1.1, 0,
                        "/tmp/lept/misc/crop_bois1.pdf", NULL);
    pixDestroy(&pix1);
    pixDestroy(&pix2);

        /* Page cropping with removal of fg on left and right sides */
    pix1 = pixRead("boismort.15.tif");
    pix2 = pixCropImage(pix1, 50, 50, -1, 70, 70, 1.1, 0,
                        "/tmp/lept/misc/crop_bois15.pdf", NULL);
    pixDestroy(&pix1);
    pixDestroy(&pix2);

        /* Page cropping for 2 columns with junk on left and right sides,
         * This is the 2-column introductory material from Bruggen's
         * transcription of Bach's Cello Suites 1-3.  The right column
         * is kanji, which is relatively weak for coalescing into
         * connected blocks.  So it is necessary to include a
         * horizontal close/open of size 3, and in the algorithm, the
         * vertical close/open needs to be at least 70 at 4x reduction. */
    lept_mkdir("lept/2_column");
    lept_cp("2_column_crop_input.pdf", "lept/2_column", "input.pdf", NULL);
    snprintf(buf, sizeof(buf),
        "croppdf /tmp/lept/2_column 50 50 -1 70 70 1.12 0"
        " none /tmp/lept/misc/2_column_crop_result.pdf");
    lept_stderr("Writing /tmp/lept/misc/2_column_crop_result.pdf\n");
    callSystemDebug(buf);

        /* Page cropping for oversize media box that causes the renderer
         * to embed the page in a larger black image.  So we need to
         * extract the actual page.  This is now done with croppdf, using
         * edgeclean = -2.  The bad scan was encoded with jbig2.  It looks
         * OK when rendering with evince, but pdftoppm is tripped up by the
         * mediabox.  See the rendered images at the end of this file. */
    lept_mkdir("lept/bad_mediabox");
    lept_cp("bad_mediabox_input.pdf", "lept/bad_mediabox", "input.pdf", NULL);
    snprintf(buf, sizeof(buf),
        "croppdf /tmp/lept/bad_mediabox 50 50 -2 80 80 1.12 0"
        " none /tmp/lept/misc/bad_mediabox_crop_result.pdf");
    lept_stderr("Writing /tmp/lept/misc/bad_mediabox_crop_result.pdf\n");
    callSystemDebug(buf);

        /* Page cleaning */
    pixa1 = pixaCreate(3);
    pix1 = pixRead("tel_3.tif");
    pix2 = pixRotate(pix1, 0.02, L_ROTATE_SAMPLING, L_BRING_IN_WHITE, 0, 0);
    pix3 = pixCleanImage(pix2, 1, 0, 1, 0);
    pixaAddPix(pixa1, pix3, L_INSERT);
    pixDisplay(pix3, 800, 800);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pix1 = pixRead("w91frag.jpg");
    pixaAddPix(pixa1, pixScale(pix1, 2.5, 2.5), L_INSERT);
    pix2 = pixRotate(pix1, 0.02, L_ROTATE_AREA_MAP, L_BRING_IN_WHITE, 0, 0);
    pix3 = pixCleanImage(pix2, 1, 0, 1, 0);
    pixaAddPix(pixa1, pixScale(pix3, 2.5, 2.5), L_INSERT);
    pixDisplay(pix3, 1200, 800);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    lept_stderr("Writing /tmp/lept/misc/pageclean.pdf\n");
    pixaConvertToPdf(pixa1, 0, 1.0, L_DEFAULT_ENCODE, 50, NULL,
                     "/tmp/lept/misc/pageclean.pdf");
    pixaDestroy(&pixa1);
    
        /* Input images to bad mediabox example pages; delayed from
         * above to give system a chance to generate them. */
    snprintf(buf, sizeof(buf), "displaypix /tmp/lept/renderpdf/input-1.ppm");
    callSystemDebug(buf);
    snprintf(buf, sizeof(buf), "displaypix /tmp/lept/renderpdf/input-2.ppm");
    callSystemDebug(buf);

    return 0;
}
