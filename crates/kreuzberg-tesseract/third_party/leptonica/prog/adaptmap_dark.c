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
 *  adaptmap_dark.c
 *
 *    Demonstrates the effect of the fg threshold on adaptive mapping
 *    and cleaning for images with dark and variable background.
 *
 *    The example pages are text and image.  For both, because the
 *    background is both dark and variable, using a lower threshold
 *    gives much better results.
 *
 *    For text, cleaning the background to white after adaptively
 *    remapping to make the background uniform is preferable.
 *    The final cleaning step uses pixGammaTRC() where the white value
 *    (here, 180) is set below the remapped gray value (here, 200).
 *
 *    For the image, however, it is best to stop after remapping
 *    the background.  Going further and moving pixels near the
 *    background color to white removes the details in the lighter
 *    regions of the image.  In practice, parts of a scanned page
 *    that are image (as opposed to text) don't necessarily have
 *    background pixels that should be white.  These regions can be
 *    protected by masks from operations, such as pixGammaTRC(),
 *    where the white value is less than 255.
 *
 *    This also tests some code useful for rendering:
 *     * NUp display from pixa to pixa
 *     * Interleaving of both pixa and pixacomp
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "string.h"
#include "allheaders.h"

void GenCleans(const char *fname, l_int32  *pindex, l_int32 thresh, L_BMF *bmf);

l_int32 main(int    argc,
             char **argv)
{
l_int32  index;
L_BMF   *bmf;
PIXA    *pixa1, *pixa2, *pixa3, *pixa4;
PIXAC   *pixac1, *pixac2, *pixac3;

    setLeptDebugOK(1);
    bmf = bmfCreate(NULL, 10);
    index = 0;
    lept_mkdir("lept/adapt");

        /* Using a variety of different thresholds */
    GenCleans("cavalerie.29.jpg", &index, 80, bmf);
    GenCleans("cavalerie.29.jpg", &index, 60, bmf);
    GenCleans("cavalerie.29.jpg", &index, 40, bmf);
    GenCleans("cavalerie.11.jpg", &index, 80, bmf);
    GenCleans("cavalerie.11.jpg", &index, 60, bmf);
    GenCleans("cavalerie.11.jpg", &index, 40, bmf);

        /* Read the images and convert to a 4-up pixa */
    pixa1 = convertToNUpPixa("/tmp/lept/adapt", "adapt_", 2, 2, 500,
                             6, 2, 0);

        /* Convert to pdf */
    L_INFO("Writing to /tmp/lept/adapt/cleaning.pdf\n", __func__);
    pixaConvertToPdf(pixa1, 100, 1.0, L_JPEG_ENCODE,
                     75, "Adaptive cleaning",
                     "/tmp/lept/adapt/cleaning.pdf");
    pixaDestroy(&pixa1);

        /* Test the pixac interleaving.  Make two copies,
         * and interleave them:
         *   (1) convert NUp 2 x 1
         *   (2) convert twice to pixac
         *   (3) interleave the two copies
         *   (4) convert back to pixa
         *   (5) convert NUp 1 x 2   (result now is 2 x 2)
         *   (6) output as pdf   */
    pixa1 = convertToNUpPixa("/tmp/lept/adapt", "adapt_", 2, 1, 500,
                             6, 2, 0);
    startTimer();
    pixac1 = pixacompCreateFromPixa(pixa1, IFF_DEFAULT, L_CLONE);
    pixac2 = pixacompCreateFromPixa(pixa1, IFF_DEFAULT, L_CLONE);
    pixac3 = pixacompInterleave(pixac1, pixac2);
    pixa2 = pixaCreateFromPixacomp(pixac3, L_CLONE);
    pixa3 = pixaConvertToNUpPixa(pixa2, NULL, 1, 2, 1000, 6, 2, 0);
    lept_stderr("Time with pixac interleaving = %7.3f sec\n", stopTimer());
    L_INFO("Writing to /tmp/lept/adapt/cleaning2.pdf\n", __func__);
    pixaConvertToPdf(pixa3, 100, 1.0, L_JPEG_ENCODE,
                     75, "Adaptive cleaning", "/tmp/lept/adapt/cleaning2.pdf");
    pixaDestroy(&pixa1);
    pixaDestroy(&pixa2);
    pixaDestroy(&pixa3);
    pixacompDestroy(&pixac1);
    pixacompDestroy(&pixac2);
    pixacompDestroy(&pixac3);

        /* Test the pixa interleaving.  Make two copies,
         * and interleave them:
         *   (1) convert NUp 2 x 1
         *   (2) copy and interleave
         *   (3) convert NUp 1 x 2   (result now is 2 x 2)
         *   (4) output as pdf   */
    pixa1 = convertToNUpPixa("/tmp/lept/adapt", "adapt_", 2, 1, 500,
                             6, 2, 0);
    startTimer();
    pixa2 = pixaCopy(pixa1, L_COPY_CLONE);
    pixa3 = pixaInterleave(pixa1, pixa2, L_CLONE);
    pixa4 = pixaConvertToNUpPixa(pixa3, NULL, 1, 2, 1000, 6, 2, 0);
    lept_stderr("Time with pixa interleaving = %7.3f sec\n", stopTimer());
    L_INFO("Writing to /tmp/lept/adapt/cleaning3.pdf\n", __func__);
    pixaConvertToPdf(pixa4, 100, 1.0, L_JPEG_ENCODE,
                     75, "Adaptive cleaning", "/tmp/lept/adapt/cleaning3.pdf");
    pixaDestroy(&pixa1);
    pixaDestroy(&pixa2);
    pixaDestroy(&pixa3);
    pixaDestroy(&pixa4);
    bmfDestroy(&bmf);
    return 0;
}

void
GenCleans(const char  *fname,
          l_int32     *pindex,
          l_int32      thresh,
          L_BMF       *bmf)
{
l_int32  index, blackval, whiteval;
char     buf[256];
PIX     *pix1, *pix2, *pix3, *pix4, *pix5;

    blackval = 70;
    whiteval = 180;
    index = *pindex;
    pix1 = pixRead(fname);
    snprintf(buf, sizeof(buf), "/tmp/lept/adapt/adapt_%03d.jpg", index++);
    pixWrite(buf, pix1, IFF_JFIF_JPEG);

    pix2 = pixBackgroundNorm(pix1, NULL, NULL, 10, 15, thresh, 25, 200, 2, 1);
    snprintf(buf, sizeof(buf), "Norm color: fg thresh = %d", thresh);
    lept_stderr("%s\n", buf);
    pix3 = pixAddTextlines(pix2, bmf, buf, 0x00ff0000, L_ADD_BELOW);
    snprintf(buf, sizeof(buf), "/tmp/lept/adapt/adapt_%03d.jpg", index++);
    pixWrite(buf, pix3, IFF_JFIF_JPEG);
    pixDestroy(&pix3);
    pix3 = pixGammaTRC(NULL, pix2, 1.0, blackval, whiteval);
    snprintf(buf, sizeof(buf), "Clean color: fg thresh = %d", thresh);
    pix4 = pixAddSingleTextblock(pix3, bmf, buf, 0x00ff0000, L_ADD_BELOW, NULL);
    snprintf(buf, sizeof(buf), "/tmp/lept/adapt/adapt_%03d.jpg", index++);
    pixWrite(buf, pix4, IFF_JFIF_JPEG);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);

    pix2 = pixConvertRGBToGray(pix1, 0.33, 0.34, 0.33);
    pix3 = pixBackgroundNorm(pix2, NULL, NULL, 10, 15, thresh, 25, 200, 2, 1);
    pix4 = pixGammaTRC(NULL, pix3, 1.0, blackval, whiteval);
    snprintf(buf, sizeof(buf), "Clean gray: fg thresh = %d", thresh);
    pix5 = pixAddSingleTextblock(pix4, bmf, buf, 0x00ff0000, L_ADD_BELOW, NULL);
    snprintf(buf, sizeof(buf), "/tmp/lept/adapt/adapt_%03d.jpg", index++);
    pixWrite(buf, pix5, IFF_JFIF_JPEG);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);
    pixDestroy(&pix5);

    pixDestroy(&pix1);
    *pindex = index;
    return;
}
