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
 *  yuvtest.c
 *
 *    Test the yuv to rgb conversion.
 *
 *    Note that the yuv gamut is greater than rgb, so although any
 *    rgb image can be converted to yuv (and back), any possible
 *    yuv value does not necessarily represent a valid rgb value.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

void AddTransformsRGB(PIXA *pixa, L_BMF *bmf, l_int32 gval);
void AddTransformsYUV(PIXA *pixa, L_BMF *bmf, l_int32 yval);


l_int32 main(int    argc,
             char **argv)
{
l_int32     i, rval, gval, bval, yval, uval, vval;
l_float32  *a[3], b[3];
L_BMF      *bmf;
PIX        *pixd;
PIXA       *pixa;

    setLeptDebugOK(1);
    lept_mkdir("lept/yuv");

        /* Explore the range of rgb --> yuv transforms.  All rgb
         * values transform to a valid value of yuv, so when transforming
         * back we get the same rgb values that we started with. */
    pixa = pixaCreate(0);
    bmf = bmfCreate("fonts", 6);
    for (gval = 0; gval <= 255; gval += 20)
        AddTransformsRGB(pixa, bmf, gval);

    pixd = pixaDisplayTiledAndScaled(pixa, 32, 755, 1, 0, 20, 2);
    pixDisplay(pixd, 100, 0);
    pixWrite("/tmp/lept/yuv/yuv1.png", pixd, IFF_PNG);
    pixDestroy(&pixd);
    pixaDestroy(&pixa);

        /* Now start with all "valid" yuv values, not all of which are
         * related to a valid rgb value.  Our yuv --> rgb transform
         * clips the rgb components to [0 ... 255], so when transforming
         * back we get different values whenever the initial yuv
         * value is out of the rgb gamut. */
    pixa = pixaCreate(0);
    for (yval = 16; yval <= 235; yval += 16)
        AddTransformsYUV(pixa, bmf, yval);

    pixd = pixaDisplayTiledAndScaled(pixa, 32, 755, 1, 0, 20, 2);
    pixDisplay(pixd, 600, 0);
    pixWrite("/tmp/lept/yuv/yuv2.png", pixd, IFF_PNG);
    pixDestroy(&pixd);
    pixaDestroy(&pixa);
    bmfDestroy(&bmf);


    /* --------- Try out a special case by hand, and show that --------- *
     * ------- the transform matrices we are using are inverses ---------*/

        /* First, use our functions for the transform */
    lept_stderr("Start with: yval = 143, uval = 79, vval = 103\n");
    convertYUVToRGB(143, 79, 103, &rval, &gval, &bval);
    lept_stderr(" ==> rval = %d, gval = %d, bval = %d\n", rval, gval, bval);
    convertRGBToYUV(rval, gval, bval, &yval, &uval, &vval);
    lept_stderr(" ==> yval = %d, uval = %d, vval = %d\n", yval, uval, vval);

        /* Next, convert yuv --> rbg by solving for rgb --> yuv transform.
         *      [ a00   a01   a02 ]    r   =   b0           (y - 16)
         *      [ a10   a11   a12 ] *  g   =   b1           (u - 128)
         *      [ a20   a21   a22 ]    b   =   b2           (v - 128)
         */
    b[0] = 143.0 - 16.0;    /* y - 16 */
    b[1] = 79.0 - 128.0;   /* u - 128 */
    b[2] = 103.0 - 128.0;    /* v - 128 */
    for (i = 0; i < 3; i++)
        a[i] = (l_float32 *)lept_calloc(3, sizeof(l_float32));
    a[0][0] = 65.738 / 256.0;
    a[0][1] = 129.057 / 256.0;
    a[0][2] = 25.064 / 256.0;
    a[1][0] = -37.945 / 256.0;
    a[1][1] = -74.494 / 256.0;
    a[1][2] = 112.439 / 256.0;
    a[2][0] = 112.439 / 256.0;
    a[2][1] = -94.154 / 256.0;
    a[2][2] = -18.285 / 256.0;
    lept_stderr("Here's the original matrix: yuv --> rgb:\n");
    for (i = 0; i < 3; i++)
        lept_stderr("    %7.3f  %7.3f  %7.3f\n", 256.0 * a[i][0],
                    256.0 * a[i][1], 256.0 * a[i][2]);
    gaussjordan(a, b, 3);
    lept_stderr("\nInput (yuv) = (143,79,103); solve for rgb:\n"
                "rval = %7.3f, gval = %7.3f, bval = %7.3f\n", b[0], b[1], b[2]);
    lept_stderr("Here's the inverse matrix: rgb --> yuv:\n");
    for (i = 0; i < 3; i++)
        lept_stderr("    %7.3f  %7.3f  %7.3f\n", 256.0 * a[i][0],
                    256.0 * a[i][1], 256.0 * a[i][2]);

        /* Now, convert back: rgb --> yuv;
         * Do this by solving for yuv --> rgb transform.
         * Use the b[] found previously (the rgb values), and
         * the a[][] which now holds the rgb --> yuv transform.  */
    gaussjordan(a, b, 3);
    lept_stderr("\nInput rgb; solve for yuv:\n"
                "yval = %7.3f, uval = %7.3f, vval = %7.3f\n",
                b[0] + 16.0, b[1] + 128.0, b[2] + 128.0);
    lept_stderr("Inverting the matrix again: yuv --> rgb:\n");
    for (i = 0; i < 3; i++)
        lept_stderr("    %7.3f  %7.3f  %7.3f\n", 256.0 * a[i][0],
                    256.0 * a[i][1], 256.0 * a[i][2]);

    for (i = 0; i < 3; i++) lept_free(a[i]);
    return 0;
}


void
AddTransformsRGB(PIXA    *pixa,
                 L_BMF   *bmf,
                 l_int32  gval)
{
char       textbuf[256];
l_int32    i, j, wpls;
l_uint32  *datas, *lines;
PIX       *pixs, *pixt1, *pixt2, *pixt3, *pixt4;
PIXA      *pixat;

    pixs = pixCreate(255, 255, 32);
    wpls = pixGetWpl(pixs);
    datas = pixGetData(pixs);
    for (i = 0; i < 255; i++) {   /* r */
        lines = datas + i * wpls;
        for (j = 0; j < 255; j++)  /* b */
            composeRGBPixel(i, gval, j, lines + j);
    }

    pixat = pixaCreate(3);
    pixaAddPix(pixat, pixs, L_INSERT);
    pixt1 = pixConvertRGBToYUV(NULL, pixs);
    pixaAddPix(pixat, pixt1, L_INSERT);
    pixt2 = pixConvertYUVToRGB(NULL, pixt1);
    pixaAddPix(pixat, pixt2, L_INSERT);
    pixt3 = pixaDisplayTiledAndScaled(pixat, 32, 255, 3, 0, 20, 2);
    snprintf(textbuf, sizeof(textbuf), "gval = %d", gval);
    pixt4 = pixAddSingleTextblock(pixt3, bmf, textbuf, 0xff000000,
                                  L_ADD_BELOW, NULL);
    pixaAddPix(pixa, pixt4, L_INSERT);
    pixDestroy(&pixt3);
    pixaDestroy(&pixat);
    return;
}


void
AddTransformsYUV(PIXA    *pixa,
                 L_BMF   *bmf,
                 l_int32  yval)
{
char       textbuf[256];
l_int32    i, j, wpls;
l_uint32  *datas, *lines;
PIX       *pixs, *pixt1, *pixt2, *pixt3, *pixt4;
PIXA      *pixat;

    pixs = pixCreate(225, 225, 32);
    wpls = pixGetWpl(pixs);
    datas = pixGetData(pixs);
    for (i = 0; i < 225; i++) {   /* v */
        lines = datas + i * wpls;
        for (j = 0; j < 225; j++)  /* u */
            composeRGBPixel(yval + 16, j + 16, i + 16, lines + j);
    }

    pixat = pixaCreate(3);
    pixaAddPix(pixat, pixs, L_INSERT);
    pixt1 = pixConvertYUVToRGB(NULL, pixs);
    pixaAddPix(pixat, pixt1, L_INSERT);
    pixt2 = pixConvertRGBToYUV(NULL, pixt1);
    pixaAddPix(pixat, pixt2, L_INSERT);
    pixt3 = pixaDisplayTiledAndScaled(pixat, 32, 225, 3, 0, 20, 2);
    snprintf(textbuf, sizeof(textbuf), "yval = %d", yval);
    pixt4 = pixAddSingleTextblock(pixt3, bmf, textbuf, 0xff000000,
                                  L_ADD_BELOW, NULL);
    pixaAddPix(pixa, pixt4, L_INSERT);
    pixDestroy(&pixt3);
    pixaDestroy(&pixat);
    return;
}

