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
 *  insert_reg.c
 *
 *  This tests removal and insertion operations in numa, boxa and pixa.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include <math.h>
#include "allheaders.h"

int main(int    argc,
         char **argv)
{
l_int32       i, n;
l_float32     pi, angle, val;
BOX          *box;
BOXA         *boxa, *boxa1, *boxa2;
NUMA         *na1, *na2;
PIX          *pix, *pix1, *pix2;
PIXA         *pixa1, *pixa2, *pixa3, *pixa4;
L_REGPARAMS  *rp;

#if !defined(HAVE_LIBPNG)
    L_ERROR("This test requires libpng to run.\n", "insert_reg");
    exit(77);
#endif

    if (regTestSetup(argc, argv, &rp))
        return 1;

    lept_rmfile("/tmp/lept/regout/insert3.ba");
    lept_rmfile("/tmp/lept/regout/insert4.ba");
    lept_rmfile("/tmp/lept/regout/insert6.pa");
    lept_rmfile("/tmp/lept/regout/insert7.pa");
    lept_rmfile("/tmp/lept/regout/insert9.pa");
    lept_rmfile("/tmp/lept/regout/insert10.pa");

    /* ----------------- Test numa operations -------------------- */
    pi = 3.1415926535;
    na1 = numaCreate(500);
    for (i = 0; i < 500; i++) {
        angle = 0.02293 * i * pi;
        val = (l_float32)sin(angle);
        numaAddNumber(na1, val);
    }
    numaWrite("/tmp/lept/regout/insert0.na", na1);
    na2 = numaCopy(na1);
    n = numaGetCount(na2);
    for (i = 0; i < n; i++) {
        numaGetFValue(na2, i, &val);
        numaRemoveNumber(na2, i);
        numaInsertNumber(na2, i, val);
    }
    numaWrite("/tmp/lept/regout/insert1.na", na2);
    regTestCheckFile(rp, "/tmp/lept/regout/insert0.na");  /* 0 */
    regTestCheckFile(rp, "/tmp/lept/regout/insert1.na");  /* 1 */
    regTestCompareFiles(rp, 0, 1);  /* 2 */
    numaDestroy(&na1);
    numaDestroy(&na2);

    /* ----------------- Test boxa operations -------------------- */
    pix1 = pixRead("feyn.tif");
    box = boxCreate(1138, 1666, 1070, 380);
    pix2 = pixClipRectangle(pix1, box, NULL);
    boxDestroy(&box);
    boxa1 = pixConnComp(pix2, NULL, 8);
    boxaWrite("/tmp/lept/regout/insert3.ba", boxa1);
    boxa2 = boxaCopy(boxa1, L_COPY);
    n = boxaGetCount(boxa2);
    for (i = 0; i < n; i++) {
        boxaRemoveBoxAndSave(boxa2, i, &box);
        boxaInsertBox(boxa2, i, box);
    }
    boxaWrite("/tmp/lept/regout/insert4.ba", boxa2);
    regTestCheckFile(rp, "/tmp/lept/regout/insert3.ba");  /* 3 */
    regTestCheckFile(rp, "/tmp/lept/regout/insert4.ba");  /* 4 */
    regTestCompareFiles(rp, 3, 4);  /* 5 */
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    boxaDestroy(&boxa1);
    boxaDestroy(&boxa2);

    /* ----------------- Test pixa operations -------------------- */
    pix1 = pixRead("feyn.tif");
    box = boxCreate(1138, 1666, 1070, 380);
    pix2 = pixClipRectangle(pix1, box, NULL);
    boxDestroy(&box);
    boxa = pixConnComp(pix2, &pixa1, 8);
    boxaDestroy(&boxa);
    pixaWrite("/tmp/lept/regout/insert6.pa", pixa1);
    regTestCheckFile(rp, "/tmp/lept/regout/insert6.pa");  /* 6 */
    pixDestroy(&pix1);
    pixDestroy(&pix2);

        /* Remove and insert each one */
    pixa2 = pixaCopy(pixa1, L_COPY);
    n = pixaGetCount(pixa2);
    for (i = 0; i < n; i++) {
        pixaRemovePixAndSave(pixa2, i, &pix, &box);
        pixaInsertPix(pixa2, i, pix, box);
    }
    pixaWrite("/tmp/lept/regout/insert7.pa", pixa2);
    regTestCheckFile(rp, "/tmp/lept/regout/insert7.pa");  /* 7 */
    regTestCompareFiles(rp, 6, 7);  /* 8 */

        /* Move the last to the beginning; do it n times */
    pixa3 = pixaCopy(pixa2, L_COPY);
    for (i = 0; i < n; i++) {
        pix = pixaGetPix(pixa3, n - 1, L_CLONE);
        box = pixaGetBox(pixa3, n - 1, L_CLONE);
        pixaInsertPix(pixa3, 0, pix, box);
        pixaRemovePix(pixa3, n);
    }
    pixaWrite("/tmp/lept/regout/insert9.pa", pixa3);
    regTestCheckFile(rp, "/tmp/lept/regout/insert9.pa");  /* 9 */

        /* Move the first one to the end; do it n times */
    pixa4 = pixaCopy(pixa3, L_COPY);
    for (i = 0; i < n; i++) {
        pix = pixaGetPix(pixa4, 0, L_CLONE);
        box = pixaGetBox(pixa4, 0, L_CLONE);
        pixaInsertPix(pixa4, n, pix, box);  /* make sure insert works at end */
        pixaRemovePix(pixa4, 0);
    }
    pixaWrite("/tmp/lept/regout/insert10.pa", pixa4);
    regTestCheckFile(rp, "/tmp/lept/regout/insert10.pa");  /* 10 */
    regTestCompareFiles(rp, 9, 10);  /* 11 */
    pixaDestroy(&pixa1);
    pixaDestroy(&pixa2);
    pixaDestroy(&pixa3);
    pixaDestroy(&pixa4);

    return regTestCleanup(rp);
}
