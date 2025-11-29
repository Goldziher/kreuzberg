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
 * pixalloc_reg.c
 *
 *   Tests custom pix allocator.
 *
 *   The custom allocator is intended for situations where a number of large
 *   pix will be repeatedly allocated and freed over the lifetime of a program.
 *   If those pix are large, relying on malloc and free can result in
 *   fragmentation, even if there are no small memory leaks in the program.
 *
 *   Here we test the allocator in two situations:
 *     * a small number of relatively large pix
 *     * a large number of very small pix
 *
 *   For the second case, timing shows that the custom allocator does
 *   about as well as (malloc, free), even for thousands of very small pix.
 *   (Turn off logging to get a fair comparison).
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include <math.h>
#include "allheaders.h"

static const l_int32 logging = FALSE;

static const l_int32 ncopies = 2;
static const l_int32 nlevels = 4;
static const l_int32 ntimes = 30;


PIXA *GenerateSetOfMargePix(void);
void CopyStoreClean(PIXA *pixas, l_int32 nlevels, l_int32 ncopies);


int main(int    argc,
         char **argv)
{
l_int32  i;
BOXA    *boxa;
NUMA    *nas, *nab;
PIX     *pixs;
PIXA    *pixa, *pixas;

    setLeptDebugOK(1);
    lept_mkdir("lept/alloc");

    /* ----------------- Custom with a few large pix -----------------*/
        /* Set up pms */
    nas = numaCreate(4);  /* small */
    numaAddNumber(nas, 5);
    numaAddNumber(nas, 4);
    numaAddNumber(nas, 3);
    numaAddNumber(nas, 2);
    setPixMemoryManager(pmsCustomAlloc, pmsCustomDealloc);
    pmsCreate(200000, 400000, nas, "/tmp/lept/alloc/file1.log");

        /* Make the pix and do successive copies and removals of the copies */
    pixas = GenerateSetOfMargePix();
    startTimer();
    for (i = 0; i < ntimes; i++)
        CopyStoreClean(pixas, nlevels, ncopies);
    lept_stderr("Time (big pix; custom) = %7.3f sec\n", stopTimer());

        /* Clean up */
    numaDestroy(&nas);
    pixaDestroy(&pixas);
    pmsDestroy();


    /* ----------------- Standard with a few large pix -----------------*/
    setPixMemoryManager(malloc, free);

        /* Make the pix and do successive copies and removals of the copies */
    startTimer();
    pixas = GenerateSetOfMargePix();
    for (i = 0; i < ntimes; i++)
        CopyStoreClean(pixas, nlevels, ncopies);
    lept_stderr("Time (big pix; standard) = %7.3f sec\n", stopTimer());
    pixaDestroy(&pixas);


    /* ----------------- Custom with many small pix -----------------*/
        /* Set up pms */
    nab = numaCreate(10);
    numaAddNumber(nab, 2000);
    numaAddNumber(nab, 2000);
    numaAddNumber(nab, 2000);
    numaAddNumber(nab, 500);
    numaAddNumber(nab, 100);
    numaAddNumber(nab, 100);
    numaAddNumber(nab, 100);
    setPixMemoryManager(pmsCustomAlloc, pmsCustomDealloc);
    if (logging)   /* use logging == 0 for speed comparison */
        pmsCreate(20, 40, nab, "/tmp/lept/alloc/file2.log");
    else
        pmsCreate(20, 40, nab, NULL);
    pixs = pixRead("feyn.tif");

    startTimer();
    for (i = 0; i < 5; i++) {
        boxa = pixConnComp(pixs, &pixa, 8);
        boxaDestroy(&boxa);
        pixaDestroy(&pixa);
    }

    numaDestroy(&nab);
    pixDestroy(&pixs);
    pmsDestroy();
    lept_stderr("Time (custom) = %7.3f sec\n", stopTimer());


    /* ----------------- Standard with many small pix -----------------*/
    setPixMemoryManager(malloc, free);
    pixs = pixRead("feyn.tif");

    startTimer();
    for (i = 0; i < 5; i++) {
        boxa = pixConnComp(pixs, &pixa, 8);
        boxaDestroy(&boxa);
        pixaDestroy(&pixa);
    }
    pixDestroy(&pixs);
    lept_stderr("Time (standard) = %7.3f sec\n", stopTimer());
    return 0;
}


PIXA *
GenerateSetOfMargePix(void)
{
l_float32  factor;
BOX   *box;
PIX   *pixs, *pixt1, *pixt2, *pixt3, *pixt4;
PIXA  *pixa;

    pixs = pixRead("marge.jpg");
    box = boxCreate(130, 93, 263, 253);
    factor = sqrt(2.0);
    pixt1 = pixClipRectangle(pixs, box, NULL);  /* 266 KB */
    pixt2 = pixScale(pixt1, factor, factor);    /* 532 KB */
    pixt3 = pixScale(pixt2, factor, factor);    /* 1064 KB */
    pixt4 = pixScale(pixt3, factor, factor);    /* 2128 KB */
    pixa = pixaCreate(4);
    pixaAddPix(pixa, pixt1, L_INSERT);
    pixaAddPix(pixa, pixt2, L_INSERT);
    pixaAddPix(pixa, pixt3, L_INSERT);
    pixaAddPix(pixa, pixt4, L_INSERT);
    boxDestroy(&box);
    pixDestroy(&pixs);
    return pixa;
}


void
CopyStoreClean(PIXA    *pixas,
               l_int32  nlevels,
               l_int32  ncopies)
{
l_int32  i, j;
PIX     *pix, *pixt;
PIXA    *pixa;
PIXAA   *paa;

    paa = pixaaCreate(0);
    for (i = 0; i < nlevels ; i++) {
        pixa = pixaCreate(0);
        pixaaAddPixa(paa, pixa, L_INSERT);
        pix = pixaGetPix(pixas, i, L_CLONE);
        for (j = 0; j < ncopies; j++) {
            pixt = pixCopy(NULL, pix);
            pixaAddPix(pixa, pixt, L_INSERT);
        }
        pixDestroy(&pix);
    }
    pixaaDestroy(&paa);

    return;
}

