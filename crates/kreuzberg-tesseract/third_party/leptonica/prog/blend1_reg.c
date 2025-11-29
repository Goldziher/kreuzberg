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
 * blend1_reg.c
 *
 *   Regression test for these functions:
 *       pixBlendGray()
 *       pixBlendGrayAdapt()
 *       pixBlendColor()
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

void GrayBlend(PIX  *pixs, PIX  *pixb, l_int32 op, l_float32 fract);
void AdaptiveGrayBlend(PIX  *pixs, PIX  *pixb, l_float32 fract);
void ColorBlend(PIX  *pixs, PIX  *pixb, l_float32 fract);
PIX *MakeGrayWash(l_int32 w, l_int32 h);
PIX *MakeColorWash(l_int32 w, l_int32 h, l_int32 color);


int main(int    argc,
         char **argv)
{
PIX          *pixs, *pixg, *pixc, *pix1;
PIXA         *pixa;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

        /* Set up blenders */
    pixg = pixRead("blender8.png");
    pix1 = pixRead("weasel4.11c.png");
    pixc = pixRemoveColormap(pix1, REMOVE_CMAP_TO_FULL_COLOR);
    pixDestroy(&pix1);
    pixa = pixaCreate(0);

        /* Gray blend (straight) */
    pixs = pixRead("test24.jpg");
    pix1 = pixScale(pixs, 0.4, 0.4);
    GrayBlend(pix1, pixg, L_BLEND_GRAY, 0.3);
    regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 0 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pixDisplayWithTitle(pix1, 0, 100, NULL, rp->display);
    pixDestroy(&pixs);

    pixs = pixRead("marge.jpg");
    GrayBlend(pixs, pixg, L_BLEND_GRAY, 0.2);
    regTestWritePixAndCheck(rp, pixs, IFF_JFIF_JPEG);  /* 1 */
    pixaAddPix(pixa, pixs, L_INSERT);
    pixDisplayWithTitle(pixs, 100, 100, NULL, rp->display);

    pixs = pixRead("marge.jpg");
    pix1 = pixConvertRGBToLuminance(pixs);
    GrayBlend(pix1, pixg, L_BLEND_GRAY, 0.2);
    regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 2 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pixDisplayWithTitle(pix1, 200, 100, NULL, rp->display);
    pixDestroy(&pixs);

        /* Gray blend (inverse) */
    pixs = pixRead("test24.jpg");
    pix1 = pixScale(pixs, 0.4, 0.4);
    GrayBlend(pix1, pixg, L_BLEND_GRAY_WITH_INVERSE, 0.6);
    regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 3 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pixDisplayWithTitle(pix1, 300, 100, NULL, rp->display);
    pixDestroy(&pixs);

    pixs = pixRead("marge.jpg");
    GrayBlend(pixs, pixg, L_BLEND_GRAY_WITH_INVERSE, 0.6);
    regTestWritePixAndCheck(rp, pixs, IFF_JFIF_JPEG);  /* 4 */
    pixaAddPix(pixa, pixs, L_INSERT);
    pixDisplayWithTitle(pixs, 400, 100, NULL, rp->display);

    pixs = pixRead("marge.jpg");
    pix1 = pixConvertRGBToLuminance(pixs);
    GrayBlend(pix1, pixg, L_BLEND_GRAY_WITH_INVERSE, 0.6);
    regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 5 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pixDisplayWithTitle(pix1, 500, 100, NULL, rp->display);
    pixDestroy(&pixs);

    pixs = MakeGrayWash(1000, 120);
    GrayBlend(pixs, pixg, L_BLEND_GRAY_WITH_INVERSE, 0.3);
    regTestWritePixAndCheck(rp, pixs, IFF_PNG);  /* 6 */
    pixaAddPix(pixa, pixs, L_INSERT);
    pixDisplayWithTitle(pixs, 0, 600, NULL, rp->display);

    pixs = MakeColorWash(1000, 120, COLOR_RED);
    GrayBlend(pixs, pixg, L_BLEND_GRAY_WITH_INVERSE, 1.0);
    regTestWritePixAndCheck(rp, pixs, IFF_PNG);  /* 7 */
    pixaAddPix(pixa, pixs, L_INSERT);
    pixDisplayWithTitle(pixs, 0, 750, NULL, rp->display);

        /* Adaptive gray blend */
    pixs = pixRead("test24.jpg");
    pix1 = pixScale(pixs, 0.4, 0.4);
    AdaptiveGrayBlend(pix1, pixg, 0.8);
    regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 8 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pixDisplayWithTitle(pix1, 600, 100, NULL, rp->display);
    pixDestroy(&pixs);

    pixs = pixRead("marge.jpg");
    AdaptiveGrayBlend(pixs, pixg, 0.8);
    regTestWritePixAndCheck(rp, pixs, IFF_JFIF_JPEG);  /* 9 */
    pixaAddPix(pixa, pixs, L_INSERT);
    pixDisplayWithTitle(pixs, 700, 100, NULL, rp->display);

    pix1 = pixConvertRGBToLuminance(pixs);
    AdaptiveGrayBlend(pix1, pixg, 0.1);
    regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 10 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pixDisplayWithTitle(pix1, 800, 100, NULL, rp->display);

    pixs = MakeGrayWash(1000, 120);
    AdaptiveGrayBlend(pixs, pixg, 0.3);
    regTestWritePixAndCheck(rp, pixs, IFF_PNG);  /* 11 */
    pixaAddPix(pixa, pixs, L_INSERT);
    pixDisplayWithTitle(pixs, 0, 900, NULL, rp->display);

    pixs = MakeColorWash(1000, 120, COLOR_RED);
    AdaptiveGrayBlend(pixs, pixg, 0.5);
    regTestWritePixAndCheck(rp, pixs, IFF_PNG);  /* 12 */
    pixaAddPix(pixa, pixs, L_INSERT);
    pixDisplayWithTitle(pixs, 0, 1050, NULL, rp->display);

        /* Color blend */
    pixs = pixRead("test24.jpg");
    pix1 = pixScale(pixs, 0.4, 0.4);
    ColorBlend(pix1, pixc, 0.3);
    regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 13 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pixDisplayWithTitle(pix1, 900, 100, NULL, rp->display);
    pixDestroy(&pixs);

    pixs = pixRead("marge.jpg");
    ColorBlend(pixs, pixc, 0.30);
    regTestWritePixAndCheck(rp, pixs, IFF_JFIF_JPEG);  /* 14 */
    pixaAddPix(pixa, pixs, L_INSERT);
    pixDisplayWithTitle(pixs, 1000, 100, NULL, rp->display);

    pixs = pixRead("marge.jpg");
    ColorBlend(pixs, pixc, 0.15);
    regTestWritePixAndCheck(rp, pixs, IFF_JFIF_JPEG);  /* 15 */
    pixaAddPix(pixa, pixs, L_INSERT);
    pixDisplayWithTitle(pixs, 1100, 100, NULL, rp->display);

        /* Mosaic all results */
    pix1 = pixaDisplayTiledInRows(pixa, 32, 1700, 1.0, 0, 20, 2);
    regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 16 */
    pixDisplayWithTitle(pix1, 0, 0, NULL, rp->display);
    pixaDestroy(&pixa);
    pixDestroy(&pix1);

    pixDestroy(&pixg);
    pixDestroy(&pixc);
    return regTestCleanup(rp);
}


void
GrayBlend(PIX       *pixs,
          PIX       *pixb,
          l_int32    op,
          l_float32  fract)
{
l_int32   i, j, wb, hb, ws, hs, delx, dely, x, y;

    pixGetDimensions(pixs, &ws, &hs, NULL);
    pixGetDimensions(pixb, &wb, &hb, NULL);
    delx = wb + 30;
    dely = hb + 25;
    x = 200;
    y = 300;
    for (i = 0; i < 20; i++) {
        y = 20 + i * dely;
        if (y >= hs - hb)
            continue;
        for (j = 0; j < 20; j++) {
            x = 30 + j * delx;
            if (x >= ws - wb)
                continue;
            pixBlendGray(pixs, pixs, pixb, x, y, fract, op, 1, 255);
        }
    }
}


void
AdaptiveGrayBlend(PIX       *pixs,
                  PIX       *pixb,
                  l_float32  fract)
{
l_int32   i, j, wb, hb, ws, hs, delx, dely, x, y;

    pixGetDimensions(pixs, &ws, &hs, NULL);
    pixGetDimensions(pixb, &wb, &hb, NULL);
    delx = wb + 30;
    dely = hb + 25;
    x = 200;
    y = 300;
    for (i = 0; i < 20; i++) {
        y = 20 + i * dely;
        if (y >= hs - hb)
            continue;
        for (j = 0; j < 20; j++) {
            x = 30 + j * delx;
            if (x >= ws - wb)
                continue;
            pixBlendGrayAdapt(pixs, pixs, pixb, x, y, fract, 80);
        }
    }
}


void
ColorBlend(PIX       *pixs,
           PIX       *pixb,
           l_float32  fract)
{
l_int32   i, j, wb, hb, ws, hs, delx, dely, x, y;

    pixGetDimensions(pixs, &ws, &hs, NULL);
    pixGetDimensions(pixb, &wb, &hb, NULL);
    delx = wb + 30;
    dely = hb + 25;
    x = 200;
    y = 300;
    for (i = 0; i < 20; i++) {
        y = 20 + i * dely;
        if (y >= hs - hb)
            continue;
        for (j = 0; j < 20; j++) {
            x = 30 + j * delx;
            if (x >= ws - wb)
                continue;
            pixBlendColor(pixs, pixs, pixb, x, y, fract, 1, 255);
        }
    }
}


PIX *
MakeGrayWash(l_int32  w,
             l_int32  h)
{
l_int32    i, j, wpl, val;
l_uint32  *data, *line;
PIX       *pixd;

    pixd = pixCreate(w, h, 8);
    data = pixGetData(pixd);
    wpl = pixGetWpl(pixd);
    for (i = 0; i < h; i++) {
        line = data + i * wpl;
        for (j = 0; j < w; j++) {
            val = (j * 255) / w;
            SET_DATA_BYTE(line, j, val);
        }
    }
    return pixd;
}


PIX *
MakeColorWash(l_int32  w,
              l_int32  h,
              l_int32  color)
{
l_int32    i, j, wpl;
l_uint32   val;
l_uint32  *data, *line;
PIX       *pixd;

    pixd = pixCreate(w, h, 32);
    data = pixGetData(pixd);
    wpl = pixGetWpl(pixd);
    for (i = 0; i < h; i++) {
        line = data + i * wpl;
        for (j = 0; j < w; j++) {
            if (color == COLOR_RED)
                val = ((j * 255) / w) << L_GREEN_SHIFT |
                      ((j * 255) / w) << L_BLUE_SHIFT |
                      255 << L_RED_SHIFT;
            else if (color == COLOR_GREEN)
                val = ((j * 255) / w) << L_RED_SHIFT |
                      ((j * 255) / w) << L_BLUE_SHIFT |
                      255 << L_GREEN_SHIFT;
            else
                val = ((j * 255) / w) << L_RED_SHIFT |
                      ((j * 255) / w) << L_GREEN_SHIFT |
                      255 << L_BLUE_SHIFT;
            line[j] = val;
        }
    }
    return pixd;
}

