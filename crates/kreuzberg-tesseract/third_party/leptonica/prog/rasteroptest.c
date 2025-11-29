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
 * rasteroptest.c
 *
 *      This is in essence a fuzzing test for rasterop.
 *
 *      These timings are for 1000 iterations of the inner loop.
 *          rasterop:
 *              optimizing:    0.35 sec
 *              valgrind:      12 sec
 *          rasteropIP:
 *              optimizing:    0.18 sec  (two calls)
 *              valgrind:      13 sec  (two calls)
 *
 *      This has been tested with valgrind on:
 *      * all ops with niters = 10,000
 *      * op = PIX_SRC with niters = 100,000
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

void GeneralTest(PIX *pix1, BOX *box1, BOX *box2, l_int32 op, l_int32 niters);
void InplaceTest(PIX *pix1, BOX *box1, BOX *box2, l_int32 op, l_int32 niters);

int main(int    argc,
         char **argv)
{
BOX     *box1, *box2;
PIX     *pix1;
l_int32  niters, op, selectop;

    setLeptDebugOK(1);

    pix1 = pixRead("test24.jpg");
    box1 = boxCreate(243, 127, 513, 359);
    box2 = boxCreate(541, 312, 513, 359);
    niters = 10000;
    selectop = PIX_SRC;

#if 1
        /* Basic rasterop */
    for (op = 0; op < 16; op++)
        GeneralTest(pix1, box1, box2, op, niters);
#endif

#if 1
        /* In-place rasterop */
    for (op = 0; op < 16; op++)
        InplaceTest(pix1, box1, box2, op, niters);
#endif

#if 0
        /* Basic rasterop; single operation */
    GeneralTest(pix1, box1, box2, selectop, niters);
#endif

    pixDestroy(&pix1);
    boxDestroy(&box1);
    boxDestroy(&box2);
    return 0;
}

/* ------------------------------------------------------------------- */
void GeneralTest(PIX *pix1, BOX *box1, BOX *box2, l_int32 op, l_int32 niters)
{
PIX     *pix2, *pix3;
l_int32  i, val1, val2, val3, val4, val5, val6;

    startTimer();
    for (i = 0; i < niters; i++) {
        pix2 = pixClipRectangle(pix1, box1, NULL);
        pix3 = pixClipRectangle(pix1, box2, NULL);
        genRandomIntOnInterval(-42, 403, 0, &val1);
        genRandomIntOnInterval(-18, 289, 0, &val2);
        genRandomIntOnInterval(13, 289, 0, &val3);
        genRandomIntOnInterval(13, 403, 0, &val4);
        genRandomIntOnInterval(-34, 289, 0, &val5);
        genRandomIntOnInterval(-38, 403, 0, &val6);
        pixRasterop(pix3, val1, val2, val3, val4, op, pix2, val5, val6);
        if (i == op) {
            lept_stderr("Rasterop: op = %d    ", op);
            pixDisplay(pix3, 100 * i, 100);
        }
        pixDestroy(&pix2);
        pixDestroy(&pix3);
    }
    lept_stderr("Time = %7.3f sec\n", stopTimer());
}

/* ------------------------------------------------------------------- */
void InplaceTest(PIX *pix1, BOX *box1, BOX *box2, l_int32 op, l_int32 niters)
{
PIX     *pix2;
l_int32  i, val1, val2, val3, val4, val5, val6;

    startTimer();
    for (i = 0; i < niters; i++) {
        pix2 = pixClipRectangle(pix1, box1, NULL);
        genRandomIntOnInterval(-217, 113, 0, &val1);
        genRandomIntOnInterval(1, 211, 0, &val2);
        genRandomIntOnInterval(-217, 143, 0, &val3);
        genRandomIntOnInterval(-247, 113, 0, &val4);
        genRandomIntOnInterval(1, 241, 0, &val5);
        genRandomIntOnInterval(-113, 163, 0, &val6);
        pixRasteropHip(pix2, val1, val2, val3, L_BRING_IN_WHITE);
        pixRasteropVip(pix2, val4, val5, val6, L_BRING_IN_BLACK);
        if (i == op) {
            lept_stderr("Rasterop: op = %d    ", op);
            pixDisplay(pix2, 100 * i, 500);
        }
        pixDestroy(&pix2);
    }
    lept_stderr("Time = %7.3f sec\n", stopTimer());
}
