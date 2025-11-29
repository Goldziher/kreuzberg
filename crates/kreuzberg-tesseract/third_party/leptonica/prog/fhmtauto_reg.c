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
 * fhmtauto_reg.c
 *
 *    Basic regression test for hit-miss transform: rasterops & dwa.
 *
 *    Tests hmt from a set of hmt structuring elements
 *    by comparing the full image rasterop results with the
 *    automatically generated dwa results.
 *
 *    Results must be identical for all operations.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
         char **argv)
{
l_int32       i, nsels, same1, same2;
char         *selname;
PIX          *pixs, *pixref, *pix1, *pix2, *pix3, *pix4;
SEL          *sel;
SELA         *sela;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    pixs = pixRead("feyn.tif");
    sela = selaAddHitMiss(NULL);
    nsels = selaGetCount(sela);

    for (i = 0; i < nsels; i++)
    {
        sel = selaGetSel(sela, i);
        selname = selGetName(sel);

        pixref = pixHMT(NULL, pixs, sel);
        pix1 = pixAddBorder(pixs, 32, 0);
        pix2 = pixFHMTGen_1(NULL, pix1, selname);
        pix3 = pixRemoveBorder(pix2, 32);
        pix4 = pixHMTDwa_1(NULL, pixs, selname);
        regTestComparePix(rp, pixref, pix3);  /* 0, 2, ... 18 */
        regTestComparePix(rp, pixref, pix4);  /* 1, 3, ... 19 */
        pixEqual(pixref, pix3, &same1);
        pixEqual(pixref, pix4, &same2);
        if (!same1 || !same2)
            lept_stderr("hmt differ for sel %d (%s)\n", i, selname);
        if (rp->display && same1 && same2)
            lept_stderr("hmt are identical for sel %d (%s)\n", i, selname);
        pixDestroy(&pixref);
        pixDestroy(&pix1);
        pixDestroy(&pix2);
        pixDestroy(&pix3);
        pixDestroy(&pix4);
    }

    pixDestroy(&pixs);
    selaDestroy(&sela);
    return regTestCleanup(rp);
}
