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
 * deskew_it.c
 *
 *    deskew_it filein threshold sweeprange tryboth fileout
 *
 *    where:
 *         threshold: for binarization, use 0 for default (130)
 *         sweeprange: half the sweep angle search range, in degrees;
 *                    use 0 for default (7.0 degrees)
 *         tryboth: 1 to test for skew both as input and with a 90 deg rotation;
 *                  0 to test for skew as input only
 *
 *    On failure to deskew, write the input image to the output (not rotated).
 *
 *    For further information on these and other defaulted parameters,
 *    see skew.c.

 *    For testing the deskew functions, see skewtest.c and the skew
 *    regression test skew_reg.c.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

    /* Default binarization threshold */
static const l_int32  DefaultThreshold = 130;

    /* Default half angle for searching */
static const l_float32  DefaultSweepRange = 7.0;    /* degrees */


int main(int    argc,
         char **argv)
{
char      *filein, *fileout;
l_int32    threshold, tryboth, format;
l_float32  sweeprange, angle, conf;
PIX       *pixs, *pix1, *pix2, *pixd;

    if (argc != 6)
        return ERROR_INT(
          "\n   Syntax: deskew_it filein threshold sweeprange tryboth fileout",
          __func__, 1);

    filein = argv[1];
    threshold = atoi(argv[2]);
    sweeprange = atof(argv[3]);
    tryboth = atoi(argv[4]);
    fileout = argv[5];

    setLeptDebugOK(1);
    pixd = NULL;

    if ((pixs = pixRead(filein)) == NULL)
        return ERROR_INT("pixs not made", __func__, 1);

    sweeprange = (sweeprange == 0) ? DefaultSweepRange : sweeprange;
    threshold = (threshold == 0) ? DefaultThreshold : threshold;
    format = pixGetInputFormat(pixs);
    if (format == IFF_UNKNOWN) format = IFF_PNG;

    pixd = pixDeskewGeneral(pixs, 0, sweeprange, 0.0, 0, threshold,
                            &angle, &conf);
    if (!pixd) {
        L_ERROR("deskew failed; pixd not made\n", __func__);
        pixWrite(fileout, pixs, format);
        pixDestroy(&pixs);
        return 1;
    }
    lept_stderr("skew angle = %.3f, conf = %.1f\n", angle, conf);

        /* Two situations were we're finished:
         * (1) conf >= 3.0 and it's good enough, so write out pixd
         * (2) conf < 3.0, so pixd is a clone of pixs, and we're
         * only trying once. */
    if (conf >= 3.0 || tryboth == 0) {
        pixWrite(fileout, pixd, format);
        pixDestroy(&pixs);
        pixDestroy(&pixd);
        return 0;
    }
    pixDestroy(&pixd);

        /* Confidence was less than the min acceptable, but we will
         * try again (tryboth == 1) after a 90 degree rotation. */
    pix1 = pixRotateOrth(pixs, 1);
    pix2 = pixDeskewGeneral(pix1, 0, sweeprange, 0.0, 0, threshold,
                            &angle, &conf);
    pixDestroy(&pix1);
    if (!pix2) {
        L_ERROR("deskew failed at 90 deg; pixd not made\n", __func__);
        pixWrite(fileout, pixs, format);
        pixDestroy(&pixs);
        return 1;
    }
    lept_stderr("90 rot: skew angle = %.3f, conf = %.1f\n", angle, conf);

    if (conf < 3.0) {
        pixWrite(fileout, pixs, format);
    } else {
        pixd = pixRotateOrth(pix2, 3);
        pixWrite(fileout, pixd, format);
        pixDestroy(&pixd);
    }
    pixDestroy(&pixs);
    pixDestroy(&pix2);
    return 0;
}
