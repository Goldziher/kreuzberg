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
 * modifyhuesat.c
 *
 *     modifyhuesat filein nhue dhue nsat dsat fileout
 *
 *         where nhue and nsat are odd
 *
 *     This gives a rectangle of nhue x nsat output images,
 *     where the center image is not modified.
 *
 *     Example: modifyhuesat test24.jpg 5 0.2 5 0.2 /tmp/junkout.jpg
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
         char **argv)
{
char      *filein, *fileout;
l_int32    i, j, w, d, nhue, nsat, tilewidth;
l_float32  scale, dhue, dsat, delhue, delsat;
PIX       *pixs, *pixt1, *pixt2, *pixd;
PIXA      *pixa;

    if (argc != 7)
        return ERROR_INT(
            " Syntax: modifyhuesat filein nhue dhue nsat dsat fileout",
            __func__, 1);
    filein = argv[1];
    nhue = atoi(argv[2]);
    dhue = atof(argv[3]);
    nsat = atoi(argv[4]);
    dsat = atof(argv[5]);
    fileout = argv[6];
    if (nhue % 2 == 0) {
        nhue++;
        lept_stderr("nhue must be odd; raised to %d\n", nhue);
    }
    if (nsat % 2 == 0) {
        nsat++;
        lept_stderr("nsat must be odd; raised to %d\n", nsat);
    }

    setLeptDebugOK(1);
    if ((pixt1 = pixRead(filein)) == NULL)
        return ERROR_INT("pixt1 not read", __func__, 1);
    pixGetDimensions(pixt1, &w, NULL, NULL);
    scale = 250.0 / (l_float32)w;
    pixt2 = pixScale(pixt1, scale, scale);
    pixs = pixConvertTo32(pixt2);
    pixDestroy(&pixt1);
    pixDestroy(&pixt2);

    pixGetDimensions(pixs, &w, NULL, &d);
    pixa = pixaCreate(nhue * nsat);
    for (i = 0; i < nsat; i++) {
        delsat = (i - nsat / 2.0) * dsat;
	pixt1 = pixModifySaturation(NULL, pixs, delsat);
        for (j = 0; j < nhue; j++) {
            delhue = (j - nhue / 2.0) * dhue;
            pixt2 = pixModifyHue(NULL, pixt1, delhue);
            pixaAddPix(pixa, pixt2, L_INSERT);
        }
        pixDestroy(&pixt1);
    }

    tilewidth = L_MIN(w, 1500 / nsat);
    pixd = pixaDisplayTiledAndScaled(pixa, d, tilewidth, nsat, 0, 25, 3);
    pixWrite(fileout, pixd, IFF_JFIF_JPEG);

    pixDestroy(&pixs);
    pixDestroy(&pixd);
    pixaDestroy(&pixa);
    return 0;
}

