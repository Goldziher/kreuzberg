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
 * jbclass_reg.c
 *
 *   Regression test for
 *       jbCorrelation
 *       jbRankhaus
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

    /* Choose one of these */
#define  COMPONENTS  JB_CONN_COMPS
/* #define  COMPONENTS  JB_CHARACTERS */
/* #define  COMPONENTS  JB_WORDS */

static PIXA *PixaOutlineTemplates(PIXA *pixas, NUMA *na);


int main(int    argc,
         char **argv)
{
l_int32      i, w, h;
BOX         *box;
JBDATA      *data;
JBCLASSER   *classer;
NUMA        *na;
SARRAY      *sa;
PIX         *pix1, *pix2;
PIXA        *pixa1, *pixa2;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    lept_mkdir("lept/class");

        /* Set up the input data */
    pix1 = pixRead("pageseg1.tif");
    pixGetDimensions(pix1, &w, &h, NULL);
    box = boxCreate(0, 0, w, h / 2);
    pix2 = pixClipRectangle(pix1, box, NULL);
    pixWrite("/tmp/lept/class/pix1.tif", pix2, IFF_TIFF_G4);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pix1 = pixRead("pageseg4.tif");
    pix2 = pixClipRectangle(pix1, box, NULL);
    pixWrite("/tmp/lept/class/pix2.tif", pix2, IFF_TIFF_G4);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    boxDestroy(&box);
    sa = sarrayCreate(2);
    sarrayAddString(sa, "/tmp/lept/class/pix1.tif", L_COPY);
    sarrayAddString(sa, "/tmp/lept/class/pix2.tif", L_COPY);

    /*--------------------------------------------------------------*/

        /* Run the correlation-based classifier */
    classer = jbCorrelationInit(COMPONENTS, 0, 0, 0.8, 0.6);
    jbAddPages(classer, sa);

        /* Save and write out the result */
    data = jbDataSave(classer);
    jbDataWrite("/tmp/lept/class/corr", data);
    lept_stderr("Number of classes: %d\n", classer->nclass);

    pix1 = pixRead("/tmp/lept/class/corr.templates.png");
    regTestWritePixAndCheck(rp, pix1, IFF_TIFF_G4);  /* 0 */
    pixDisplayWithTitle(pix1, 0, 0, NULL, rp->display);
    pixDestroy(&pix1);

        /* Render the pages from the classifier data.
         * Use debugflag == FALSE to omit outlines of each component. */
    pixa1 = jbDataRender(data, FALSE);
    for (i = 0; i < 2; i++) {
        pix1 = pixaGetPix(pixa1, i, L_CLONE);
        regTestWritePixAndCheck(rp, pix1, IFF_TIFF_G4);  /* 1, 2 */
        pixDestroy(&pix1);
    }
    pixaDestroy(&pixa1);

        /* Display all instances, organized by template */
    pixa1 = pixaaFlattenToPixa(classer->pixaa, &na, L_CLONE);
    pixa2 = PixaOutlineTemplates(pixa1, na);
    pix1 = pixaDisplayTiledInColumns(pixa2, 40, 1.0, 10, 0);
    regTestWritePixAndCheck(rp, pix1, IFF_TIFF_G4);  /* 3 */
    pixDestroy(&pix1);
    pixaDestroy(&pixa1);
    pixaDestroy(&pixa2);
    numaDestroy(&na);
    jbClasserDestroy(&classer);
    jbDataDestroy(&data);

    /*--------------------------------------------------------------*/

    lept_mkdir("lept/class2");

        /* Run the rank hausdorff-based classifier */
    classer = jbRankHausInit(COMPONENTS, 0, 0, 2, 0.97);
    jbAddPages(classer, sa);

        /* Save and write out the result */
    data = jbDataSave(classer);
    jbDataWrite("/tmp/lept/class2/haus", data);
    lept_stderr("Number of classes: %d\n", classer->nclass);

    pix1 = pixRead("/tmp/lept/class2/haus.templates.png");
    regTestWritePixAndCheck(rp, pix1, IFF_TIFF_G4);  /* 4 */
    pixDisplayWithTitle(pix1, 200, 0, NULL, rp->display);
    pixDestroy(&pix1);

        /* Render the pages from the classifier data.
         * Use debugflag == FALSE to omit outlines of each component. */
    pixa1 = jbDataRender(data, FALSE);
    for (i = 0; i < 2; i++) {
        pix1 = pixaGetPix(pixa1, i, L_CLONE);
        regTestWritePixAndCheck(rp, pix1, IFF_TIFF_G4);  /* 5, 6 */
        pixDestroy(&pix1);
    }
    pixaDestroy(&pixa1);

        /* Display all instances, organized by template */
    pixa1 = pixaaFlattenToPixa(classer->pixaa, &na, L_CLONE);
    pixa2 = PixaOutlineTemplates(pixa1, na);
    pix1 = pixaDisplayTiledInColumns(pixa2, 40, 1.0, 10, 0);
    regTestWritePixAndCheck(rp, pix1, IFF_TIFF_G4);  /* 7 */
    pixDestroy(&pix1);
    pixaDestroy(&pixa1);
    pixaDestroy(&pixa2);
    numaDestroy(&na);
    jbClasserDestroy(&classer);
    jbDataDestroy(&data);

    /*--------------------------------------------------------------*/

    sarrayDestroy(&sa);
    return regTestCleanup(rp);
}


static PIXA *
PixaOutlineTemplates(PIXA  *pixas,
                     NUMA  *na)
{
l_int32  i, n, val, prev, curr;
NUMA    *nai;
PIX     *pix1, *pix2, *pix3;
PIXA    *pixad;

        /* Make an indicator array with a 1 for each template image */
    n = numaGetCount(na);
    nai = numaCreate(n);
    prev = -1;
    for (i = 0; i < n; i++) {
        numaGetIValue(na, i, &curr);
        if (curr != prev) {  /* index change */
            prev = curr;
            numaAddNumber(nai, 1);
        } else {
            numaAddNumber(nai, 0);
        }
    }

        /* Add a boundary of 3 white and 1 black pixels to templates */
    pixad = pixaCreate(n);
    for (i = 0; i < n; i++) {
        pix1 = pixaGetPix(pixas, i, L_CLONE);
        numaGetIValue(nai, i, &val);
        if (val == 0) {
            pixaAddPix(pixad, pix1, L_INSERT);
        } else {
            pix2 = pixAddBorder(pix1, 3, 0);
            pix3 = pixAddBorder(pix2, 1, 1);
            pixaAddPix(pixad, pix3, L_INSERT);
            pixDestroy(&pix1);
            pixDestroy(&pix2);
        }
    }

    numaDestroy(&nai);
    return pixad;
}
