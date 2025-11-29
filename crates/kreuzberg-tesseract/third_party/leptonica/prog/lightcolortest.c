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
 * lightcolortest.c
 *
 *   Determines if there are light colors on the image.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

static const l_int32  nbins = 10;

int main(int    argc,
         char **argv)
{
char      *name, *tail;
l_int32    i, j, n, minval, maxval, rdiff, gdiff, bdiff, maxdiff;
l_uint32  *rau32, *gau32, *bau32, *carray, *darray;
PIX       *pixs, *pix1, *pix2, *pix3, *pix4;
PIXA      *pixa, *pixa1;
SARRAY    *sa;

    if (argc != 1)
        return ERROR_INT(" Syntax:  lightcolortest", __func__, 1);

    setLeptDebugOK(1);
    sa = getSortedPathnamesInDirectory( ".", "comap.", 0, 0);
    sarrayWriteStderr(sa);
    n = sarrayGetCount(sa);
    lept_stderr("n = %d\n", n);
    pixa = pixaCreate(n);
    for (i = 0; i < n; i++) {
        pixa1 = pixaCreate(2);
        name = sarrayGetString(sa, i, L_NOCOPY);
        splitPathAtDirectory(name, NULL, &tail);
        pixs = pixRead(name);
        lept_stderr("%s:\n", tail);
        pix1 = pixScaleBySampling(pixs, 0.2, 0.2);

        pixGetBinnedComponentRange(pix1, nbins, 2, L_SELECT_RED,
                                   &minval, &maxval, &rau32, 0);
        lept_stderr("  Red: max = %d, min = %d\n", maxval, minval);
        rdiff = maxval - minval;
        pixGetBinnedComponentRange(pix1, nbins, 2, L_SELECT_GREEN,
                                   &minval, &maxval, &gau32, 0);
        lept_stderr("  Green: max = %d, min = %d\n", maxval, minval);
        gdiff = maxval - minval;
        pixGetBinnedComponentRange(pix1, nbins, 2, L_SELECT_BLUE,
                                   &minval, &maxval, &bau32, 0);
        lept_stderr("  Blue: max = %d, min = %d\n", maxval, minval);
        bdiff = maxval - minval;
        lept_stderr("rdiff = %d, gdiff = %d, bdiff = %d\n\n",
                    rdiff, gdiff, bdiff);
        maxdiff = L_MAX(rdiff, gdiff);
        maxdiff = L_MAX(maxdiff, bdiff);
        if (maxdiff == rdiff) {
            carray = rau32;
            lept_free(gau32);
            lept_free(bau32);
        } else if (maxdiff == gdiff) {
            carray = gau32;
            lept_free(bau32);
            lept_free(rau32);
        } else {   /* maxdiff == bdiff */
            carray = bau32;
            lept_free(rau32);
            lept_free(gau32);
        }

        pix2 = pixDisplayColorArray(carray, nbins, 200, 5, 6);
        pixaAddPix(pixa1, pix2, L_INSERT);

        darray = (l_uint32 *)lept_calloc(nbins, sizeof(l_uint32));
        for (j = 0; j < nbins; j++) {
            pixelLinearMapToTargetColor(carray[j], carray[nbins - 1],
                           0xffffff00, &darray[j]);
        }
        pix3 = pixDisplayColorArray(darray, nbins, 200, 5, 6);
        pixaAddPix(pixa1, pix3, L_INSERT);
        pix4 = pixaDisplayLinearly(pixa1, L_VERT, 1.0, 0, 30, 3, NULL);
        pixaAddPix(pixa, pix4, L_INSERT);

        pixaDestroy(&pixa1);
        pixDestroy(&pixs);
        pixDestroy(&pix1);
        lept_free(tail);
        lept_free(carray);
        lept_free(darray);
    }

    lept_mkdir("lept/color");
    pixaConvertToPdf(pixa, 100, 1.0, L_FLATE_ENCODE, 0, "lightcolortest",
                     "/tmp/lept/color/lightcolortest.pdf");
    L_INFO("Generated pdf file: /tmp/lept/color/lightcolortest.pdf",
           __func__);
    pixaDestroy(&pixa);
    sarrayDestroy(&sa);
    return 0;
}
