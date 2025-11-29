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
 * smoothedge_reg.c
 *
 *   Analyzes edges of a 1 bpp (connected component) image for smoothness.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

static l_int32 MIN_JUMP = 2;
static l_int32 MIN_REVERSAL = 3;

void PixAddEdgeData(PIXA *pixa, PIX *pixs, l_int32 side, l_int32 minjump,
                    l_int32 minreversal);


int main(int    argc,
         char **argv)
{
l_int32  w;
PIX     *pixs, *pixt, *pixd;
PIXA    *pixa;

    pixs = pixRead("raggededge.png");
    w = pixGetWidth(pixs);
    pixa = pixaCreate(0);
    PixAddEdgeData(pixa, pixs, L_FROM_RIGHT, MIN_JUMP, MIN_REVERSAL);
    PixAddEdgeData(pixa, pixs, L_FROM_LEFT, MIN_JUMP, MIN_REVERSAL);
    pixt = pixRotateOrth(pixs, 1);
    PixAddEdgeData(pixa, pixt, L_FROM_BOT, MIN_JUMP, MIN_REVERSAL);
    PixAddEdgeData(pixa, pixt, L_FROM_TOP, MIN_JUMP, MIN_REVERSAL);
    pixDestroy(&pixt);
    pixt = pixRotateOrth(pixs, 2);
    PixAddEdgeData(pixa, pixt, L_FROM_LEFT, MIN_JUMP, MIN_REVERSAL);
    PixAddEdgeData(pixa, pixt, L_FROM_RIGHT, MIN_JUMP, MIN_REVERSAL);
    pixDestroy(&pixt);
    pixt = pixRotateOrth(pixs, 3);
    PixAddEdgeData(pixa, pixt, L_FROM_TOP, MIN_JUMP, MIN_REVERSAL);
    PixAddEdgeData(pixa, pixt, L_FROM_BOT, MIN_JUMP, MIN_REVERSAL);
    pixDestroy(&pixt);
    pixDestroy(&pixs);

        /* Display at 2x scaling */
    pixd = pixaDisplayTiledAndScaled(pixa, 32, 2 * (w + 10), 2, 0, 25, 2);
    pixWrite("/tmp/junkpixd.png", pixd, IFF_PNG);
    pixDestroy(&pixd);
    pixaDestroy(&pixa);
    return 0;
}


void PixAddEdgeData(PIXA    *pixa,
                    PIX     *pixs,
                    l_int32  side,
                    l_int32  minjump,
                    l_int32  minreversal)
{
l_float32  jpl, jspl, rpl;
PIX       *pixt1, *pixt2;

    pixMeasureEdgeSmoothness(pixs, side, minjump, minreversal, &jpl,
                             &jspl, &rpl, "/tmp/junkedge.png");
    lept_stderr("side = %d: jpl = %6.3f, jspl = %6.3f, rpl = %6.3f\n",
                side, jpl, jspl, rpl);
    pixt1 = pixRead("/tmp/junkedge.png");
    pixt2 = pixAddBorder(pixt1, 10, 0);  /* 10 pixel white border */
    pixaAddPix(pixa, pixt2, L_INSERT);
    pixDestroy(&pixt1);
    return;
}

