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
 * morphtest1.c
 *
 *   - Timing test for rasterop-based morphological operations
 *   - Example repository of binary morph operations
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

#define   NTIMES         100
#define   IMAGE_SIZE     8.     /* megapixels */
#define   SEL_SIZE       9
#define   BASIC_OPS      1.     /* 1 for erosion/dilation; 2 for open/close */
#define   CPU_SPEED      866.   /* MHz: set it for the machine you're using */

int main(int    argc,
         char **argv)
{
l_int32    i, index;
l_float32  cputime, epo;
char      *filein, *fileout;
PIX       *pixs, *pixd;
SEL       *sel;
SELA      *sela;

    if (argc != 3)
        return ERROR_INT(" Syntax:  morphtest1 filein fileout", __func__, 1);
    filein = argv[1];
    fileout = argv[2];
    setLeptDebugOK(1);

    if ((pixs = pixRead(filein)) == NULL)
        return ERROR_INT("pix not made", __func__, 1);
    sela = selaAddBasic(NULL);

    /* ------------------------   Timing  -------------------------------*/
#if 1
    selaFindSelByName(sela, "sel_9h", &index, &sel);
    selWriteStream(stderr, sel);
    pixd = pixCreateTemplate(pixs);

    startTimer();
    for (i = 0; i < NTIMES; i++)  {
        pixDilate(pixd, pixs, sel);
/*        if ((i % 10) == 0) lept_stderr("%d iters\n", i); */
    }
    cputime = stopTimer();
        /* Get the elementary pixel operations/sec */
    epo = BASIC_OPS * SEL_SIZE * NTIMES * IMAGE_SIZE /(cputime * CPU_SPEED);

    lept_stderr("Time: %7.3f sec\n", cputime);
    lept_stderr("Speed: %7.3f epo/cycle\n", epo);
    pixWrite(fileout, pixd, IFF_PNG);
    pixDestroy(&pixd);
#endif

    /* ------------------  Example operation from repository --------------*/
#if 1
        /* Select a structuring element */
    selaFindSelByName(sela, "sel_50h", &index, &sel);
    selWriteStream(stderr, sel);

        /* Do these operations.  See below for other ops
         * that can be substituted here. */
    pixd = pixOpen(NULL, pixs, sel);
    pixXor(pixd, pixd, pixs);
    pixWrite(fileout, pixd, IFF_PNG);
    pixDestroy(&pixd);
#endif

    pixDestroy(&pixs);
    return 0;
}


/* ==================================================================== */

/* -------------------------------------------------------------------- *
 *                 Repository for selecting various operations          *
 *                              that might be used                      *
 * -------------------------------------------------------------------- */
#if 0
    pixd = pixCreateTemplate(pixs);

    pixd = pixDilate(NULL, pixs, sel);
    pixd = pixErode(NULL, pixs, sel);
    pixd = pixOpen(NULL, pixs, sel);
    pixd = pixClose(NULL, pixs, sel);

    pixDilate(pixd, pixs, sel);
    pixErode(pixd, pixs, sel);
    pixOpen(pixd, pixs, sel);
    pixClose(pixd, pixs, sel);

    pixAnd(pixd, pixd, pixs);
    pixOr(pixd, pixd, pixs);
    pixXor(pixd, pixd, pixs);
    pixSubtract(pixd, pixd, pixs);
    pixInvert(pixd, pixs);

    pixd = pixAnd(NULL, pixd, pixs);
    pixd = pixOr(NULL, pixd, pixs);
    pixd = pixXor(NULL, pixd, pixs);
    pixd = pixSubtract(NULL, pixd, pixs);
    pixd = pixInvert(NULL, pixs);

    pixInvert(pixs, pixs);
#endif  /* 0 */

