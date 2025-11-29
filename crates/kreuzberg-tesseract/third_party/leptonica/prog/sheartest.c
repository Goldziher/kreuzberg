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
 * sheartest.c
 *
 *     sheartest filein angle fileout
 *
 *     where angle is expressed in degrees
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

#define   NTIMES   10

int main(int    argc,
         char **argv)
{
char      *filein, *fileout;
l_int32    i, w, h, liney, linex, same;
l_float32  angle, deg2rad;
PIX       *pixt1, *pixt2, *pixs, *pixd;

    if (argc != 4)
        return ERROR_INT(" Syntax:  sheartest filein angle fileout",
                         __func__, 1);

    setLeptDebugOK(1);

        /* Compare in-place H shear with H shear to a new pix */
    pixt1 = pixRead("marge.jpg");
    pixGetDimensions(pixt1, &w, &h, NULL);
    pixt2 = pixHShear(NULL, pixt1, (l_int32)(0.3 * h), 0.17, L_BRING_IN_WHITE);
    pixHShearIP(pixt1, (l_int32)(0.3 * h), 0.17, L_BRING_IN_WHITE);
    pixEqual(pixt1, pixt2, &same);
    if (same)
        lept_stderr("Correct for H shear\n");
    else
        lept_stderr("Error for H shear\n");
    pixDestroy(&pixt1);
    pixDestroy(&pixt2);

        /* Compare in-place V shear with V shear to a new pix */
    pixt1 = pixRead("marge.jpg");
    pixGetDimensions(pixt1, &w, &h, NULL);
    pixt2 = pixVShear(NULL, pixt1, (l_int32)(0.3 * w), 0.17, L_BRING_IN_WHITE);
    pixVShearIP(pixt1, (l_int32)(0.3 * w), 0.17, L_BRING_IN_WHITE);
    pixEqual(pixt1, pixt2, &same);
    if (same)
        lept_stderr("Correct for V shear\n");
    else
        lept_stderr("Error for V shear\n");
    pixDestroy(&pixt1);
    pixDestroy(&pixt2);

    filein = argv[1];
    angle = atof(argv[2]);
    fileout = argv[3];
    deg2rad = 3.1415926535 / 180.;

    if ((pixs = pixRead(filein)) == NULL)
        return ERROR_INT("pix not made", __func__, 1);

    pixGetDimensions(pixs, &w, &h, NULL);

#if 0
        /* Select an operation from this list ...
         * ------------------------------------------
    pixd = pixHShear(NULL, pixs, liney, deg2rad * angle, L_BRING_IN_WHITE);
    pixd = pixVShear(NULL, pixs, linex, deg2rad * angle, L_BRING_IN_WHITE);
    pixd = pixHShearCorner(NULL, pixs, deg2rad * angle, L_BRING_IN_WHITE);
    pixd = pixVShearCorner(NULL, pixs, deg2rad * angle, L_BRING_IN_WHITE);
    pixd = pixHShearCenter(NULL, pixs, deg2rad * angle, L_BRING_IN_WHITE);
    pixd = pixVShearCenter(NULL, pixs, deg2rad * angle, L_BRING_IN_WHITE);
    pixHShearIP(pixs, liney, deg2rad * angle, L_BRING_IN_WHITE); pixd = pixs;
    pixVShearIP(pixs, linex, deg2rad * angle, L_BRING_IN_WHITE); pixd = pixs;
    pixRasteropHip(pixs, 0, h/3, -50, L_BRING_IN_WHITE); pixd = pixs;
    pixRasteropVip(pixs, 0, w/3, -50, L_BRING_IN_WHITE); pixd = pixs;
         * ------------------------------------------
         *  ... and use it in the following:         */
    pixd = pixHShear(NULL, pixs, liney, deg2rad * angle, L_BRING_IN_WHITE);
    pixWrite(fileout, pixd, IFF_PNG);
    pixDisplay(pixd, 50, 50);
    pixDestroy(&pixd);
#endif

#if 0
        /* Do a horizontal shear about a line */
    for (i = 0; i < NTIMES; i++) {
        liney = i * h / (NTIMES - 1);
        if (liney >= h)
            liney = h - 1;
        pixd = pixHShear(NULL, pixs, liney, deg2rad * angle, L_BRING_IN_WHITE);
        pixDisplay(pixd, 50 + 10 * i, 50 + 10 * i);
        pixDestroy(&pixd);
    }
#endif

#if 0
        /* Do a vertical shear about a line */
    for (i = 0; i < NTIMES; i++) {
        linex = i * w / (NTIMES - 1);
        if (linex >= w)
            linex = w - 1;
        pixd = pixVShear(NULL, pixs, linex, deg2rad * angle, L_BRING_IN_WHITE);
        pixDisplay(pixd, 50 + 10 * i, 50 + 10 * i);
        pixDestroy(&pixd);
    }
#endif

#if 0
        /* Do a horizontal in-place shear about a line */
    pixSetPadBits(pixs, 0);
    for (i = 0; i < NTIMES; i++) {
        pixd = pixCopy(NULL, pixs);
        liney = i * h / (NTIMES - 1);
        if (liney >= h)
            liney = h - 1;
        pixHShearIP(pixd, liney, deg2rad * angle, L_BRING_IN_WHITE);
        pixDisplay(pixd, 50 + 10 * i, 50 + 10 * i);
        pixDestroy(&pixd);
    }
#endif

#if 0
        /* Do a vertical in-place shear about a line */
    for (i = 0; i < NTIMES; i++) {
        pixd = pixCopy(NULL, pixs);
        linex = i * w / (NTIMES - 1);
        if (linex >= w)
            linex = w - 1;
        pixVShearIP(pixd, linex, deg2rad * angle, L_BRING_IN_WHITE);
        pixDisplay(pixd, 50 + 10 * i, 50 + 10 * i);
        pixDestroy(&pixd);
    }
#endif

    pixDestroy(&pixs);
    return 0;
}

