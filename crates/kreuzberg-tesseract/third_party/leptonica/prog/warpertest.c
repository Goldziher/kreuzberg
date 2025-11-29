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
 *  warpertest.c
 *
 *    Tests stereoscopic warp and associated shear and stretching functions.
 *
 *    Puts output to both a tiled image and pdf.  The pdf is useful for
 *    visualizing the difference between sampling and interpolation.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

static const char  *opstr[3] = {"", "interpolated", "sampled"};
static const char  *dirstr[3] = {"", "to left", "to right"};

#define  RUN_WARP                 1
#define  RUN_QUAD_VERT_SHEAR      1
#define  RUN_LIN_HORIZ_STRETCH    1
#define  RUN_QUAD_HORIZ_STRETCH   1
#define  RUN_HORIZ_SHEAR          1
#define  RUN_VERT_SHEAR           1

int main(int    argc,
         char **argv)
{
char       buf[256];
l_int32    w, h, i, j, k, index, op, dir, stretch;
l_float32  del, angle, angledeg;
BOX       *box;
L_BMF     *bmf;
PIX       *pixs, *pix1, *pix2, *pixd;
PIXA      *pixa;

    if (argc != 1)
        return ERROR_INT("syntax: warpertest", __func__, 1);

    setLeptDebugOK(1);
    lept_mkdir("lept/warp");
    bmf = bmfCreate(NULL, 6);

    /* --------   Stereoscopic warping --------------*/
#if RUN_WARP
    pixs = pixRead("german.png");
    pixGetDimensions(pixs, &w, &h, NULL);
    pixa = pixaCreate(50);
    for (i = 0; i < 50; i++) {  /* need to test > 2 widths ! */
        j = 7 * i;
        box = boxCreate(0, 0, w - j, h - j);
        pix1 = pixClipRectangle(pixs, box, NULL);
        pixd = pixWarpStereoscopic(pix1, 15, 22, 8, 30, -20, 1);
        pixSetChromaSampling(pixd, 0);
        pixaAddPix(pixa, pixd, L_INSERT);
        pixDestroy(&pix1);
        boxDestroy(&box);
    }
    pixDestroy(&pixs);

    pixaConvertToPdf(pixa, 100, 1.0, L_JPEG_ENCODE, 0, "warp.pdf",
                     "/tmp/lept/warp/warp.pdf");
    pixd = pixaDisplayTiledInRows(pixa, 32, 2000, 1.0, 0, 20, 2);
    pixWrite("/tmp/lept/warp/warp.jpg", pixd, IFF_JFIF_JPEG);
    pixaDestroy(&pixa);
    pixDestroy(&pixd);
#endif

    /* --------   Quadratic Vertical Shear  --------------*/
#if RUN_QUAD_VERT_SHEAR
    pixs = pixCreate(501, 501, 32);
    pixGetDimensions(pixs, &w, &h, NULL);
    pixSetAll(pixs);
    pixRenderLineArb(pixs, 0, 30, 500, 30, 5, 0, 0, 255);
    pixRenderLineArb(pixs, 0, 110, 500, 110, 5, 0, 255, 0);
    pixRenderLineArb(pixs, 0, 190, 500, 190, 5, 0, 255, 255);
    pixRenderLineArb(pixs, 0, 270, 500, 270, 5, 255, 0, 0);
    pixRenderLineArb(pixs, 0, 360, 500, 360, 5, 255, 0, 255);
    pixRenderLineArb(pixs, 0, 450, 500, 450, 5, 255, 255, 0);
    pixa = pixaCreate(50);
    for (i = 0; i < 50; i++) {
        j = 3 * i;
        dir = ((i / 2) & 1) ? L_WARP_TO_RIGHT : L_WARP_TO_LEFT;
        op = (i & 1) ? L_INTERPOLATED : L_SAMPLED;
        box = boxCreate(0, 0, w - j, h - j);
        pix1 = pixClipRectangle(pixs, box, NULL);
        pix2 = pixQuadraticVShear(pix1, dir, 60, -20, op, L_BRING_IN_WHITE);

        snprintf(buf, sizeof(buf), "%s, %s", dirstr[dir], opstr[op]);
        pixd = pixAddSingleTextblock(pix2, bmf, buf, 0xff000000,
                                     L_ADD_BELOW, 0);
        pixaAddPix(pixa, pixd, L_INSERT);
        pixDestroy(&pix1);
        pixDestroy(&pix2);
        boxDestroy(&box);
    }
    pixDestroy(&pixs);

    pixaConvertToPdf(pixa, 100, 1.0, L_FLATE_ENCODE, 0, "quad_vshear.pdf",
                     "/tmp/lept/warp/quad_vshear.pdf");
    pixd = pixaDisplayTiledInRows(pixa, 32, 2000, 1.0, 0, 20, 2);
    pixWrite("/tmp/lept/warp/quad_vshear.jpg", pixd, IFF_PNG);
    pixaDestroy(&pixa);
    pixDestroy(&pixd);
#endif

    /* --------  Linear Horizontal stretching  --------------*/
#if RUN_LIN_HORIZ_STRETCH
    pixs = pixRead("german.png");
    pixa = pixaCreate(50);
    for (k = 0; k < 2; k++) {
        for (i = 0; i < 25; i++) {
            index = 25 * k + i;
            stretch = 10 + 4 * i;
            if (k == 0) stretch = -stretch;
            dir = (k == 1) ? L_WARP_TO_RIGHT : L_WARP_TO_LEFT;
            op = (i & 1) ? L_INTERPOLATED : L_SAMPLED;
            pix1 = pixStretchHorizontal(pixs, dir, L_LINEAR_WARP,
                                        stretch, op, L_BRING_IN_WHITE);
            snprintf(buf, sizeof(buf), "%s, %s", dirstr[dir], opstr[op]);
            pixd = pixAddSingleTextblock(pix1, bmf, buf, 0xff000000,
                                         L_ADD_BELOW, 0);
            pixaAddPix(pixa, pixd, L_INSERT);
            pixDestroy(&pix1);
        }
    }
    pixDestroy(&pixs);

    pixaConvertToPdf(pixa, 100, 1.0, L_JPEG_ENCODE, 0, "linear_hstretch.pdf",
                     "/tmp/lept/warp/linear_hstretch.pdf");
    pixd = pixaDisplayTiledInRows(pixa, 32, 2500, 1.0, 0, 20, 2);
    pixWrite("/tmp/lept/warp/linear_hstretch.jpg", pixd, IFF_JFIF_JPEG);
    pixaDestroy(&pixa);
    pixDestroy(&pixd);
#endif

    /* --------  Quadratic Horizontal stretching  --------------*/
#if RUN_QUAD_HORIZ_STRETCH
    pixs = pixRead("german.png");
    pixa = pixaCreate(50);
    for (k = 0; k < 2; k++) {
        for (i = 0; i < 25; i++) {
            index = 25 * k + i;
            stretch = 10 + 4 * i;
            if (k == 0) stretch = -stretch;
            dir = (k == 1) ? L_WARP_TO_RIGHT : L_WARP_TO_LEFT;
            op = (i & 1) ? L_INTERPOLATED : L_SAMPLED;
            pix1 = pixStretchHorizontal(pixs, dir, L_QUADRATIC_WARP,
                                        stretch, op, L_BRING_IN_WHITE);
            snprintf(buf, sizeof(buf), "%s, %s", dirstr[dir], opstr[op]);
            pixd = pixAddSingleTextblock(pix1, bmf, buf, 0xff000000,
                                         L_ADD_BELOW, 0);
            pixaAddPix(pixa, pixd, L_INSERT);
            pixDestroy(&pix1);
        }
    }
    pixDestroy(&pixs);

    pixaConvertToPdf(pixa, 100, 1.0, L_JPEG_ENCODE, 0, "quad_hstretch.pdf",
                     "/tmp/lept/warp/quad_hstretch.pdf");
    pixd = pixaDisplayTiledInRows(pixa, 32, 2500, 1.0, 0, 20, 2);
    pixWrite("/tmp/lept/warp/quad_hstretch.jpg", pixd, IFF_JFIF_JPEG);
    pixaDestroy(&pixa);
    pixDestroy(&pixd);
#endif

    /* --------  Horizontal Shear --------------*/
#if RUN_HORIZ_SHEAR
    pixs = pixRead("german.png");
    pixGetDimensions(pixs, &w, &h, NULL);
    pixa = pixaCreate(50);
    for (i = 0; i < 25; i++) {
        del = 0.2 / 12.;
        angle = -0.2 + (i - (i & 1)) * del;
        angledeg = 180. * angle / 3.14159265;
        op = (i & 1) ? L_INTERPOLATED : L_SAMPLED;
        if (op == L_SAMPLED)
            pix1 = pixHShear(NULL, pixs, h / 2, angle, L_BRING_IN_WHITE);
        else
            pix1 = pixHShearLI(pixs, h / 2, angle, L_BRING_IN_WHITE);
        snprintf(buf, sizeof(buf), "%6.2f degree, %s", angledeg, opstr[op]);
        pixd = pixAddSingleTextblock(pix1, bmf, buf, 0xff000000,
                                     L_ADD_BELOW, 0);
        pixaAddPix(pixa, pixd, L_INSERT);
        pixDestroy(&pix1);
    }
    pixDestroy(&pixs);

    pixaConvertToPdf(pixa, 100, 1.0, L_JPEG_ENCODE, 0, "hshear.pdf",
                     "/tmp/lept/warp/hshear.pdf");
    pixd = pixaDisplayTiledInRows(pixa, 32, 2500, 1.0, 0, 20, 2);
    pixWrite("/tmp/lept/warp/hshear.jpg", pixd, IFF_JFIF_JPEG);
    pixaDestroy(&pixa);
    pixDestroy(&pixd);
#endif

    /* --------  Vertical Shear --------------*/
#if RUN_VERT_SHEAR
    pixs = pixRead("german.png");
    pixGetDimensions(pixs, &w, &h, NULL);
    pixa = pixaCreate(50);
    for (i = 0; i < 25; i++) {
        del = 0.2 / 12.;
        angle = -0.2 + (i - (i & 1)) * del;
        angledeg = 180. * angle / 3.14159265;
        op = (i & 1) ? L_INTERPOLATED : L_SAMPLED;
        if (op == L_SAMPLED)
            pix1 = pixVShear(NULL, pixs, w / 2, angle, L_BRING_IN_WHITE);
        else
            pix1 = pixVShearLI(pixs, w / 2, angle, L_BRING_IN_WHITE);
        snprintf(buf, sizeof(buf), "%6.2f degree, %s", angledeg, opstr[op]);
        pixd = pixAddSingleTextblock(pix1, bmf, buf, 0xff000000,
                                     L_ADD_BELOW, 0);
        pixaAddPix(pixa, pixd, L_INSERT);
        pixDestroy(&pix1);
    }

    pixDestroy(&pixs);
    pixaConvertToPdf(pixa, 100, 1.0, L_JPEG_ENCODE, 0, "vshear.pdf",
                     "/tmp/lept/warp/vshear.pdf");
    pixd = pixaDisplayTiledInRows(pixa, 32, 2500, 1.0, 0, 20, 2);
    pixWrite("/tmp/lept/warp/vshear.jpg", pixd, IFF_JFIF_JPEG);
    pixaDestroy(&pixa);
    pixDestroy(&pixd);
#endif

    bmfDestroy(&bmf);
    return 0;
}
