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
 * seedfilltest.c
 *
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

#define  NTIMES             5
#define  CONNECTIVITY       8
#define  XS                 150
#define  YS                 150
#define  DFLAG              1

int main(int    argc,
         char **argv)
{
char      *filein, *fileout;
l_int32    i;
l_uint32   val;
l_float32  size;
PIX       *pixs, *pixd, *pixm, *pixmi, *pixt1, *pixt2, *pixt3;

    if (argc != 3)
        return ERROR_INT(" Syntax:  seedfilltest filein fileout", __func__, 1);
    filein = argv[1];
    fileout = argv[2];
    pixd = NULL;
    setLeptDebugOK(1);

    if ((pixm = pixRead(filein)) == NULL)
        return ERROR_INT("pixm not made", __func__, 1);
    pixmi = pixInvert(NULL, pixm);

    size = pixGetWidth(pixm) * pixGetHeight(pixm);
    pixs = pixCreateTemplate(pixm);
    for (i = 0; i < 100; i++) {
        pixGetPixel(pixm, XS + 5 * i, YS + 5 * i, &val);
        if (val == 0) break;
    }
    if (i == 100)
        return ERROR_INT("no seed pixel found", __func__, 1);
    pixSetPixel(pixs, XS + 5 * i, YS + 5 * i, 1);

#if 0
        /* hole filling; use "hole-filler.png" */
    pixt1 = pixHDome(pixmi, 100, 4);
    pixt2 = pixThresholdToBinary(pixt1, 10);
/*    pixInvert(pixt1, pixt1); */
    pixDisplay(pixt1, 100, 500);
    pixDisplay(pixt2, 600, 500);
    pixt3 = pixHolesByFilling(pixt2, 4);
    pixDilateBrick(pixt3, pixt3, 7, 7);
    pixd = pixConvertTo8(pixt3, FALSE);
    pixDisplay(pixd, 0, 100);
    pixSeedfillGray(pixd, pixmi, CONNECTIVITY);
    pixInvert(pixd, pixd);
    pixDisplay(pixmi, 500, 100);
    pixDisplay(pixd, 1000, 100);
    pixWrite("/tmp/junkpixm.png", pixmi, IFF_PNG);
    pixWrite("/tmp/junkpixd.png", pixd, IFF_PNG);
#endif

#if 0
        /* hole filling; use "hole-filler.png" */
    pixt1 = pixThresholdToBinary(pixm, 110);
    pixInvert(pixt1, pixt1);
    pixDisplay(pixt1, 100, 500);
    pixt2 = pixHolesByFilling(pixt1, 4);
    pixd = pixConvertTo8(pixt2, FALSE);
    pixDisplay(pixd, 0, 100);
    pixSeedfillGray(pixd, pixmi, CONNECTIVITY);
    pixInvert(pixd, pixd);
    pixDisplay(pixmi, 500, 100);
    pixDisplay(pixd, 1000, 100);
    pixWrite("/tmp/junkpixm.png", pixmi, IFF_PNG);
    pixWrite("/tmp/junkpixd.png", pixd, IFF_PNG);
#endif

#if 0
        /* hole filling; use "hole-filler.png" */
    pixd = pixInvert(NULL, pixm);
    pixAddConstantGray(pixd, -50);
    pixDisplay(pixd, 0, 100);
/*    pixt1 = pixThresholdToBinary(pixd, 20);
    pixDisplayWithTitle(pixt1, 600, 600, "pixt1", DFLAG); */
    pixSeedfillGray(pixd, pixmi, CONNECTIVITY);
/*    pixInvert(pixd, pixd); */
    pixDisplay(pixmi, 500, 100);
    pixDisplay(pixd, 1000, 100);
    pixWrite("/tmp/junkpixm.png", pixmi, IFF_PNG);
    pixWrite("/tmp/junkpixd.png", pixd, IFF_PNG);
#endif

#if 0
        /* test in-place seedfill for speed */
    pixd = pixClone(pixs);
    startTimer();
    pixSeedfillBinary(pixs, pixs, pixmi, CONNECTIVITY);
    lept_stderr("Filling rate: %7.4f Mpix/sec\n",
                (size/1000000.) / stopTimer());

    pixWrite(fileout, pixd, IFF_PNG);
    pixOr(pixd, pixd, pixm);
    pixWrite("/tmp/junkout1.png", pixd, IFF_PNG);
#endif

#if 0
        /* test seedfill to dest for speed */
    pixd = pixCreateTemplate(pixm);
    startTimer();
    for (i = 0; i < NTIMES; i++) {
        pixSeedfillBinary(pixd, pixs, pixmi, CONNECTIVITY);
    }
    lept_stderr("Filling rate: %7.4f Mpix/sec\n",
                (size/1000000.) * NTIMES / stopTimer());

    pixWrite(fileout, pixd, IFF_PNG);
    pixOr(pixd, pixd, pixm);
    pixWrite("/tmp/junkout1.png", pixd, IFF_PNG);
#endif

        /* use same connectivity to compare with the result of the
         * slow parallel operation */
#if 1
    pixDestroy(&pixd);
    pixd = pixSeedfillMorph(pixs, pixmi, 100, CONNECTIVITY);
    pixOr(pixd, pixd, pixm);
    pixWrite("/tmp/junkout2.png", pixd, IFF_PNG);
#endif

    pixDestroy(&pixs);
    pixDestroy(&pixm);
    pixDestroy(&pixmi);
    pixDestroy(&pixd);
    return 0;
}


