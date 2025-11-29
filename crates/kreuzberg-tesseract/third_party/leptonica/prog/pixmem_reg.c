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
 *  pixmem_reg.c
 *
 *  Tests low-level pix data accessors, and functions that call them.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
         char **argv)
{
l_uint32     *data;
PIX          *pix1, *pix2, *pix3, *pix4, *pix5;
PIXA         *pixa;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    pixa = pixaCreate(0);

        /* Copy with internal resizing: onto a cmapped image */
    pix1 = pixRead("weasel4.16c.png");
    pix2 = pixRead("feyn-fract.tif");
    pix3 = pixRead("lucasta.150.jpg");
    pixCopy(pix3, pix2);
    regTestComparePix(rp, pix2, pix3);  /* 0 */
    pixaAddPix(pixa, pix3, L_INSERT);
    pixCopy(pix1, pix3);
    regTestComparePix(rp, pix1, pix2);  /* 1 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pixDestroy(&pix2);

        /* Copy with internal resizing: from a cmapped image */
    pix1 = pixRead("weasel4.16c.png");
    pix2 = pixRead("feyn-fract.tif");
    pix3 = pixRead("lucasta.150.jpg");
    pixCopy(pix2, pix1);
    regTestComparePix(rp, pix1, pix2);  /* 2 */
    pixaAddPix(pixa, pix2, L_INSERT);
    pixCopy(pix3, pix2);
    regTestComparePix(rp, pix2, pix3);  /* 3 */
    pixaAddPix(pixa, pix3, L_INSERT);
    pixDestroy(&pix1);

        /* Transfer of data pixs --> pixd, when pixs is not cloned.
         * pixs is destroyed.  */
    pix1 = pixRead("weasel4.16c.png");
    pix2 = pixRead("feyn-fract.tif");
    pix3 = pixRead("lucasta.150.jpg");
    pix4 = pixCopy(NULL, pix1);
    pixTransferAllData(pix2, &pix1, 0, 0);
    regTestComparePix(rp, pix2, pix4);  /* 4 */
    pixaAddPix(pixa, pix2, L_COPY);
    pixTransferAllData(pix3, &pix2, 0, 0);
    regTestComparePix(rp, pix3, pix4);  /* 5 */
    pixaAddPix(pixa, pix3, L_INSERT);
    pixDestroy(&pix4);

        /* Another transfer of data pixs --> pixd, when pixs is not cloned.
         * pixs is destroyed. */
    pix1 = pixRead("weasel4.16c.png");
    pix2 = pixRead("feyn-fract.tif");
    pix3 = pixRead("lucasta.150.jpg");
    pix4 = pixCopy(NULL, pix1);
    pixTransferAllData(pix2, &pix4, 0, 0);
    regTestComparePix(rp, pix1, pix2);  /* 6 */
    pixaAddPix(pixa, pix2, L_COPY);
    pixTransferAllData(pix3, &pix2, 0, 0);
    regTestComparePix(rp, pix1, pix3);  /* 7 */
    pixaAddPix(pixa, pix3, L_INSERT);
    pixDestroy(&pix1);

        /* Transfer of data pixs --> pixd, when pixs is cloned.
         * pixs has its refcount reduced by 1. */
    pix1 = pixRead("weasel4.16c.png");
    pix2 = pixRead("feyn-fract.tif");
    pix3 = pixRead("lucasta.150.jpg");
    pix4 = pixClone(pix1);
    pix5 = pixClone(pix2);
    pixTransferAllData(pix2, &pix4, 0, 0);
    regTestComparePix(rp, pix1, pix2);  /* 8 */
    pixaAddPix(pixa, pix2, L_INSERT);
    pixTransferAllData(pix3, &pix5, 0, 0);
    regTestComparePix(rp, pix1, pix3);  /* 9 */
    pixaAddPix(pixa, pix3, L_INSERT);
    pixDestroy(&pix1);

        /* Extraction of data when pixs is not cloned, putting
         * the data into a new template of pixs. */
    pix2 = pixRead("feyn-fract.tif");
    pix3 = pixCopy(NULL, pix2);  /* for later reference */
    data = pixExtractData(pix2);
    pix4 = pixCreateTemplateNoInit(pix2);
    pixFreeAndSetData(pix4, data);
    regTestComparePix(rp, pix3, pix4);  /* 10 */
    pixaAddPix(pixa, pix4, L_INSERT);
    pixDestroy(&pix2);
    pixDestroy(&pix3);

        /* Extraction of data when pixs is cloned, putting
         * a copy of the data into a new template of pixs. */
    pix1 = pixRead("weasel4.16c.png");
    pix2 = pixClone(pix1);  /* bump refcount of pix1 to 2 */
    data = pixExtractData(pix1);  /* should make a copy of data */
    pix3 = pixCreateTemplateNoInit(pix1);
    pixFreeAndSetData(pix3, data);
    regTestComparePix(rp, pix2, pix3);  /* 11 */
    pixaAddPix(pixa, pix3, L_INSERT);
    pixDestroy(&pix1);
    pixDestroy(&pix2);

    pix1 = pixaDisplayTiledInRows(pixa, 32, 1500, 0.5, 0, 30, 2);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 12 */
    pixDisplayWithTitle(pix1, 100, 100, NULL, rp->display);
    pixaDestroy(&pixa);
    pixDestroy(&pix1);

    return regTestCleanup(rp);
}
