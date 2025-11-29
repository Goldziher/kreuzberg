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
 * blendcmaptest.c
 *
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

static const l_int32  NX = 4;
static const l_int32  NY = 5;
static const l_float32  FADE_FRACTION = 0.75;

int main(int    argc,
         char **argv)
{
l_int32   i, j, sindex, wb, hb, ws, hs, delx, dely, x, y, y0;
PIX      *pixs, *pixb, *pix1, *pix2;
PIXA     *pixa;
PIXCMAP  *cmap;

    setLeptDebugOK(1);
    lept_mkdir("lept/blend");
    pixa = pixaCreate(0);

    pixs = pixRead("rabi.png");  /* blendee */
    pixb = pixRead("weasel4.11c.png");   /* blender */

        /* Fade the blender */
    pixcmapShiftIntensity(pixGetColormap(pixb), FADE_FRACTION);

        /* Downscale the input */
    wb = pixGetWidth(pixb);
    hb = pixGetHeight(pixb);
    pix1 = pixScaleToGray4(pixs);

        /* Threshold to 5 levels, 4 bpp */
    ws = pixGetWidth(pix1);
    hs = pixGetHeight(pix1);
    pix2 = pixThresholdTo4bpp(pix1, 5, 1);
    pixaAddPix(pixa, pix2, L_COPY);
    pixaAddPix(pixa, pixb, L_COPY);
    cmap = pixGetColormap(pix2);
    pixcmapWriteStream(stderr, cmap);

        /* Overwrite the white pixels (at sindex in pix2) */
    pixcmapGetIndex(cmap, 255, 255, 255, &sindex);

        /* Blend the weasel 20 times */
    delx = ws / NX;
    dely = hs / NY;
    for (i = 0; i < NY; i++) {
        y = 20 + i * dely;
        if (y >= hs + hb)
            continue;
        for (j = 0; j < NX; j++) {
            x = 30 + j * delx;
            y0 = y;
            if (j & 1) {
                y0 = y + dely / 2;
                if (y0 >= hs + hb)
                    continue;
            }
            if (x >= ws + wb)
                continue;
            pixBlendCmap(pix2, pixb, x, y0, sindex);
        }
    }

    pixaAddPix(pixa, pix2, L_COPY);
    cmap = pixGetColormap(pix2);
    pixcmapWriteStream(stderr, cmap);
    lept_stderr("Writing to: /tmp/lept/blend/blendcmap.pdf\n");
    pixaConvertToPdf(pixa, 0, 1.0, L_FLATE_ENCODE, 0, "cmap-blendtest",
                     "/tmp/lept/blend/blendcmap.pdf");

    pixDestroy(&pixs);
    pixDestroy(&pixb);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixaDestroy(&pixa);
    return 0;
}

