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
 * baseline_reg.c
 *
 *   This tests two things:
 *   (1) The ability to find a projective transform that will deskew
 *       textlines in an image with keystoning.
 *   (2) The ability to find baselines in a text image.
 *   (3) The ability to clean background to white in a dark and
 *       mottled text image.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
         char **argv)
{
NUMA         *na;
PIX          *pixs, *pix1, *pix2, *pix3, *pix4, *pix5;
PIXA         *pixadb;
PTA          *pta;
L_REGPARAMS  *rp;

#if !defined(HAVE_LIBPNG)
    L_ERROR("This test requires libpng to run.\n", "baseline_reg");
    exit(77);
#endif

    if (regTestSetup(argc, argv, &rp))
        return 1;

    pixs = pixRead("keystone.png");

        /* Test function for deskewing using projective transform
	 * on linear approximation for local skew angle */
    pix1 = pixDeskewLocal(pixs, 10, 0, 0, 0.0, 0.0, 0.0);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 0 */

        /* Test function for finding local skew angles */
    na = pixGetLocalSkewAngles(pixs, 10, 0, 0, 0.0, 0.0, 0.0, NULL, NULL, 1);
    gplotSimple1(na, GPLOT_PNG, "/tmp/lept/baseline/ang", "Angles in degrees");
    pix2 = pixRead("/tmp/lept/baseline/ang.png");
    pix3 = pixRead("/tmp/lept/baseline/skew.png");
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 1 */
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 2 */
    pixDisplayWithTitle(pix2, 0, 550, NULL, rp->display);
    pixDisplayWithTitle(pix3, 700, 550, NULL, rp->display);
    numaDestroy(&na);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pixs);

        /* Test baseline finder */
    pixadb = pixaCreate(6);
    na = pixFindBaselines(pix1, &pta, pixadb);
    regTestCompareValues(rp, 23, numaGetCount(na), 0);  /* 3 */
    pix2 = pixRead("/tmp/lept/baseline/diff.png");
    pix3 = pixRead("/tmp/lept/baseline/loc.png");
    pix4 = pixRead("/tmp/lept/baseline/baselines.png");
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 4 */
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 5 */
    regTestWritePixAndCheck(rp, pix4, IFF_PNG);  /* 6 */
    pixDisplayWithTitle(pix2, 0, 0, NULL, rp->display);
    pixDisplayWithTitle(pix3, 700, 0, NULL, rp->display);
    pixDisplayWithTitle(pix4, 1350, 0, NULL, rp->display);
    pix5 = pixaDisplayTiledInRows(pixadb, 32, 1500, 1.0, 0, 30, 2);
    pixDisplayWithTitle(pix5, 0, 500, NULL, rp->display);
    regTestWritePixAndCheck(rp, pix5, IFF_PNG);  /* 7 */
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);
    pixDestroy(&pix5);
    pixaDestroy(&pixadb);
    numaDestroy(&na);
    ptaDestroy(&pta);

        /* Another test for baselines, with dark image.
         * With minw = 60, the number at the top of the page is skipped. */
    pixadb = pixaCreate(6);
    pixs = pixRead("pedante.079.jpg");  /* 75 ppi */
    pix1 = pixRemoveBorder(pixs, 30);
    pixaAddPix(pixadb, pix1, L_COPY);
    pix2 = pixConvertRGBToGray(pix1, 0.33, 0.34, 0.33);
    pix3 = pixScale(pix2, 4.0, 4.0);   /* scale up to 300 ppi */
    pix4 = pixCleanBackgroundToWhite(pix3, NULL, NULL, 1.0, 70, 170);
    pix5 = pixThresholdToBinary(pix4, 170);
    regTestWritePixAndCheck(rp, pix5, IFF_PNG);  /* 8 */
    pixaAddPix(pixadb, pixScale(pix5, 0.25, 0.25), L_INSERT);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);
    pix1 = pixDeskew(pix5, 2);
    na = pixFindBaselinesGen(pix1, 50, &pta, pixadb);
    regTestCompareValues(rp, 35, numaGetCount(na), 0);  /* 9 */
    pix2 = pixaDisplayTiledInRows(pixadb, 32, 1500, 1.0, 0, 30, 2);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 10 */
    pixDisplayWithTitle(pix2, 800, 500, NULL, rp->display);
    pixaDestroy(&pixadb);
    pixDestroy(&pixs);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix5);
    numaDestroy(&na);
    ptaDestroy(&pta);

        /* Another test for baselines: very short textblock is removed */
    pixadb = pixaCreate(6);
    pix1 = pixRead("baseline1.png");
    na = pixFindBaselines(pix1, &pta, pixadb);
    regTestCompareValues(rp, 2, numaGetCount(na), 0);  /* 11 */
    pix2 = pixaDisplayTiledInRows(pixadb, 32, 1500, 1.0, 0, 30, 2);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 12 */
    pixDisplayWithTitle(pix2, 1400, 500, NULL, rp->display);
    pixaDestroy(&pixadb);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    numaDestroy(&na);
    ptaDestroy(&pta);

        /* Another test for baselines: some very short lines */
    pixadb = pixaCreate(6);
    pix1 = pixRead("baseline2.tif");
    na = pixFindBaselinesGen(pix1, 30, &pta, pixadb);
    regTestCompareValues(rp, 29, numaGetCount(na), 0);  /* 13 */
    pix2 = pixaDisplayTiledInRows(pixadb, 32, 1500, 1.0, 0, 30, 2);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 14 */
    pixDisplayWithTitle(pix2, 1400, 500, NULL, rp->display);
    pixaDestroy(&pixadb);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    numaDestroy(&na);
    ptaDestroy(&pta);

        /* Another test for baselines: more short lines' */
    pixadb = pixaCreate(6);
    pix1 = pixRead("baseline3.tif");
    na = pixFindBaselinesGen(pix1, 30, &pta, pixadb);
    regTestCompareValues(rp, 40, numaGetCount(na), 0);  /* 15 */
    pix2 = pixaDisplayTiledInRows(pixadb, 32, 1500, 1.0, 0, 30, 2);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 16 */
    pixDisplayWithTitle(pix2, 1400, 500, NULL, rp->display);
    pixaDestroy(&pixadb);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    numaDestroy(&na);
    ptaDestroy(&pta);

    return regTestCleanup(rp);
}
