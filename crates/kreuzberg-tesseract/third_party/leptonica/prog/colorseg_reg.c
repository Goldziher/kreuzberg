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
 * colorseg_reg.c
 *
 *   This explores the space of the four parameters input for color
 *   segmentation.  Of the four, only two strongly determine the
 *   output result:
 *      maxdist (the maximum distance between pixels that get
 *               clustered: 20 is very small, 180 is very large)
 *      selsize (responsible for smoothing the result: 0 is no
 *               smoothing (fine texture), 8 is large smoothing)
 *
 *   For large selsize (>~ 6), large regions get the same color,
 *   and there are few colors in the final result.
 *
 *   The other two parameters, maxcolors and finalcolors, can be
 *   set small (~4) or large (~20).  When set large, @maxdist will
 *   be most influential in determining the actual number of colors.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

static const l_int32  MaxColors[] = {4, 8, 16};
static const l_int32  FinalColors[] = {4, 8, 16};

int main(int    argc,
         char **argv)
{
l_int32       i, j, k, maxdist, maxcolors, selsize, finalcolors;
l_int32       nc, rval, gval, bval;
PIX          *pixs, *pix1, *pix2;
PIXA         *pixa;
PIXCMAP      *cmap, *cmapr;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    pixs = pixRead("tetons.jpg");
    for (k = 0; k < 3; k++) {
        maxcolors = MaxColors[k];
        finalcolors = FinalColors[k];
        pixa = pixaCreate(0);
        for (i = 1; i <= 9; i++) {
            maxdist = 20 * i;
            for (j = 0; j <= 6; j++) {
                selsize = j;
                pix1 = pixColorSegment(pixs, maxdist, maxcolors, selsize,
                                       finalcolors, 0);
                pixaAddPix(pixa, pix1, L_INSERT);
            }
        }

        pix2 = pixaDisplayTiledInColumns(pixa, 7, 1.0, 15, 2);
        regTestWritePixAndCheck(rp, pix2, IFF_JFIF_JPEG);  /* 0, 1, 2 */
        pixDisplayWithTitle(pix2, 100, k * 300, "colorseg", rp->display);
        pixDestroy(&pix2);
        pixaDestroy(&pixa);
    }
    pixDestroy(&pixs);

    pixs = pixRead("wyom.jpg");
    pix1 = pixColorSegment(pixs, 50, 6, 6, 6, 0);
    cmap = pixGetColormap(pix1);
    nc = pixcmapGetCount(cmap);
    cmapr = pixcmapCreateRandom(8, 0, 0);
    for (i = 0; i < nc; i++) {
        pix2 = pixMakeMaskFromVal(pix1, i);
        pixcmapGetColor(cmapr, i, &rval, &gval, &bval);
        pixRenderHashMaskArb(pixs, pix2, 0, 0, 8, 3, i % 4, 0,
                             rval, gval, bval);
        pixDestroy(&pix2);
    }
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 3 */
    regTestWritePixAndCheck(rp, pixs, IFF_JFIF_JPEG);  /* 4 */
    pixDisplayWithTitle(pix1, 800, 0, NULL, rp->display);
    pixDisplayWithTitle(pixs, 800, 640, NULL, rp->display);
    pixDestroy(&pix1);
    pixDestroy(&pixs);
    pixcmapDestroy(&cmapr);

    return regTestCleanup(rp);
}

