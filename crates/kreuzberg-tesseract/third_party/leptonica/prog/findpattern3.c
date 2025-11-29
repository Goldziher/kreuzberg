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
 * findpattern3.c
 *
 *    We use pixGenerateSelBoundary() to generate the sels.
 *
 *    This is set up with input parameters to work on feyn.tif.
 *
 *    (1) We extract an "e" bitmap, generate a hit-miss sel, and
 *    then produce several 4 bpp colormapped renditions,
 *    with the pattern either removed or highlighted.
 *
 *    (2) We do the same with the word "Caltech".
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

    /* for pixDisplayHitMissSel() */
static const l_uint32  HitColor = 0x33aa4400;
static const l_uint32  MissColor = 0xaa44bb00;

int main(int    argc,
         char **argv)
{
BOX  *box;
PIX  *pixs, *pixc, *pixp, *pixsel, *pixhmt;
PIX  *pixd1, *pixd2, *pixd3;
SEL  *selhm;

    if (argc != 1)
        return ERROR_INT(" Syntax:  findpattern3", __func__, 1);

    setLeptDebugOK(1);
    lept_mkdir("lept/hmt");

        /* -------------------------------------------- *
         * Extract the pattern for a single character   *
         * ---------------------------------------------*/
    pixs = pixRead("feyn.tif");
    box = boxCreate(599, 1055, 18, 23);
    pixc = pixClipRectangle(pixs, box, NULL);

        /* Make a hit-miss sel */
    selhm = pixGenerateSelBoundary(pixc, 1, 2, 2, 2, 1, 1, 0, 0, &pixp);

        /* Display the sel */
    pixsel = pixDisplayHitMissSel(pixp, selhm, 7, HitColor, MissColor);
    pixDisplay(pixsel, 200, 200);
    pixWrite("/tmp/lept/hmt/pixsel1.png", pixsel, IFF_PNG);

        /* Use the Sel to find all instances in the page */
    startTimer();
    pixhmt = pixHMT(NULL, pixs, selhm);
    lept_stderr("Time to find patterns = %7.3f\n", stopTimer());

        /* Color each instance at full res */
    pixd1 = pixDisplayMatchedPattern(pixs, pixp, pixhmt, selhm->cx,
                                     selhm->cy, 0x0000ff00, 1.0, 5);
    pixWrite("/tmp/lept/hmt/pixd11.png", pixd1, IFF_PNG);

        /* Color each instance at 0.3 scale */
    pixd2 = pixDisplayMatchedPattern(pixs, pixp, pixhmt, selhm->cx,
                                     selhm->cy, 0x0000ff00, 0.5, 5);
    pixWrite("/tmp/lept/hmt/pixd12.png", pixd2, IFF_PNG);

        /* Remove each instance from the input image */
    pixd3 = pixCopy(NULL, pixs);
    pixRemoveMatchedPattern(pixd3, pixp, pixhmt, selhm->cx,
                                    selhm->cy, 1);
    pixWrite("/tmp/lept/hmt/pixr1.png", pixd3, IFF_PNG);

    boxDestroy(&box);
    selDestroy(&selhm);
    pixDestroy(&pixc);
    pixDestroy(&pixp);
    pixDestroy(&pixsel);
    pixDestroy(&pixhmt);
    pixDestroy(&pixd1);
    pixDestroy(&pixd2);
    pixDestroy(&pixd3);


        /* -------------------------------------------- *
         *        Extract the pattern for a word        *
         * ---------------------------------------------*/
    box = boxCreate(208, 872, 130, 35);
    pixc = pixClipRectangle(pixs, box, NULL);

        /* Make a hit-miss sel */
    selhm = pixGenerateSelBoundary(pixc, 2, 2, 1, 4, 1, 1, 0, 0, &pixp);

        /* Display the sel */
    pixsel = pixDisplayHitMissSel(pixp, selhm, 7, HitColor, MissColor);
    pixDisplay(pixsel, 200, 200);
    pixWrite("/tmp/lept/hmt/pixsel2.png", pixsel, IFF_PNG);

        /* Use the Sel to find all instances in the page */
    startTimer();
    pixhmt = pixHMT(NULL, pixs, selhm);
    lept_stderr("Time to find word patterns = %7.3f\n", stopTimer());

        /* Color each instance at full res */
    pixd1 = pixDisplayMatchedPattern(pixs, pixp, pixhmt, selhm->cx,
                                     selhm->cy, 0x0000ff00, 1.0, 5);
    pixWrite("/tmp/lept/hmt/pixd21.png", pixd1, IFF_PNG);

        /* Color each instance at 0.3 scale */
    pixd2 = pixDisplayMatchedPattern(pixs, pixp, pixhmt, selhm->cx,
                                     selhm->cy, 0x0000ff00, 0.5, 5);
    pixWrite("/tmp/lept/hmt/pixd22.png", pixd2, IFF_PNG);

        /* Remove each instance from the input image */
    pixd3 = pixCopy(NULL, pixs);
    pixRemoveMatchedPattern(pixd3, pixp, pixhmt, selhm->cx,
                                    selhm->cy, 1);
    pixWrite("/tmp/lept/hmt/pixr2.png", pixd3, IFF_PNG);

    selDestroy(&selhm);
    boxDestroy(&box);
    pixDestroy(&pixc);
    pixDestroy(&pixp);
    pixDestroy(&pixsel);
    pixDestroy(&pixhmt);
    pixDestroy(&pixd1);
    pixDestroy(&pixd2);
    pixDestroy(&pixd3);
    pixDestroy(&pixs);
    return 0;
}

