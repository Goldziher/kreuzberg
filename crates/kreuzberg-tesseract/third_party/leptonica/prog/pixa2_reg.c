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
 * pixa2_reg.c
 *
 *    Tests various replacement functions on pixa.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
         char **argv)
{
const char   *name;
l_int32       i, n;
BOX          *box;
PIX          *pix0, *pix1, *pixd;
PIXA         *pixa;
SARRAY       *sa1, *sa2, *sa3, *sa4;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    /* ----------------  Find all the jpg and tif images --------------- */
    sa1 = getSortedPathnamesInDirectory(".", ".jpg", 0, 0);
    sa2 = getSortedPathnamesInDirectory(".", ".tif", 0, 0);
    sa3 = sarraySelectRange(sa1, 10, 19);
    sa4 = sarraySelectRange(sa2, 10, 19);
    sarrayJoin(sa3, sa4);
    n =sarrayGetCount(sa3);
    sarrayDestroy(&sa1);
    sarrayDestroy(&sa2);
    sarrayDestroy(&sa4);
    if (rp->display) sarrayWriteStderr(sa3);

    /* ---------------- Use replace to fill up a pixa -------------------*/
    pixa = pixaCreate(1);
    pixaExtendArrayToSize(pixa, n);
    if ((pix0 = pixRead("marge.jpg")) == NULL)
        rp->success = FALSE;
    pix1 = pixScaleToSize(pix0, 144, 108);  /* scale 0.25 */
    pixDestroy(&pix0);
    pixaInitFull(pixa, pix1, NULL);  /* fill it up */
    pixd = pixaDisplayTiledInRows(pixa, 32, 1000, 1.0, 0, 25, 2);
    pixDisplayWithTitle(pixd, 100, 100, NULL, rp->display);
    pixWrite("/tmp/lept/regout/pixa2-1.jpg", pixd, IFF_JFIF_JPEG);
    pixDestroy(&pix1);
    pixDestroy(&pixd);

    /* ---------------- And again with jpgs and tifs -------------------*/
    for (i = 0; i < n; i++) {
        name = sarrayGetString(sa3, i, L_NOCOPY);
        if ((pix0 = pixRead(name)) == NULL) {
            lept_stderr("Error in %s_reg: failed to read %s\n",
                        rp->testname, name);
            rp->success = FALSE;
            continue;
        }
        pix1 = pixScaleToSize(pix0, 144, 108);
        pixaReplacePix(pixa, i, pix1, NULL);
        pixDestroy(&pix0);
    }
    pixd = pixaDisplayTiledInRows(pixa, 32, 1000, 1.0, 0, 25, 2);
    pixDisplayWithTitle(pixd, 400, 100, NULL, rp->display);
    pixWrite("/tmp/lept/regout/pixa2-2.jpg", pixd, IFF_JFIF_JPEG);
    pixDestroy(&pixd);

    /* ---------------- And again, reversing the order ------------------*/
    box = boxCreate(0, 0, 0, 0);
    pixaInitFull(pixa, NULL, box);
    boxDestroy(&box);
    for (i = 0; i < n; i++) {
        name = sarrayGetString(sa3, i, L_NOCOPY);
        if ((pix0 = pixRead(name)) == NULL) {
            lept_stderr("Error in %s_reg: failed to read %s\n",
                        rp->testname, name);
            rp->success = FALSE;
            continue;
        }
        pix1 = pixScaleToSize(pix0, 144, 108);
        pixaReplacePix(pixa, n - 1 - i, pix1, NULL);
        pixDestroy(&pix0);
    }
    pixd = pixaDisplayTiledInRows(pixa, 32, 1000, 1.0, 0, 25, 2);
    pixDisplayWithTitle(pixd, 700, 100, NULL, rp->display);
    pixWrite("/tmp/lept/regout/pixa2-3.jpg", pixd, IFF_JFIF_JPEG);
    pixDestroy(&pixd);
    sarrayDestroy(&sa3);

    pixaDestroy(&pixa);
    return regTestCleanup(rp);
}
