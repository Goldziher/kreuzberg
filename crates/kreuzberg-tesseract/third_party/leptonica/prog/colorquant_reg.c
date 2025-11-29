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
 * colorquant_reg.c
 *
 *    Regression test for various color quantizers
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

static const l_int32 MAX_WIDTH = 350;
static const char *image[4] = {"marge.jpg",
                               "test24.jpg",
                               "juditharismax.jpg",
                               "hardlight2_2.jpg"};

static l_int32 TestImage(const char *filename, l_int32 i, L_REGPARAMS *rp);
static void PixSave32(PIXA *pixa, PIX *pixc, L_REGPARAMS *rp);


int main(int    argc,
         char **argv)
{
l_int32       i;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    for (i = 0; i < 4; i++) {
/*        if (i != 2) continue; */
        TestImage(image[i], i, rp);
    }

    return regTestCleanup(rp);
}


static l_int32
TestImage(const char   *filename,
          l_int32       i,
          L_REGPARAMS  *rp)
{
char       buf[256];
l_int32    w, h;
l_float32  factor;
PIX       *pix, *pixs, *pixc, *pix32, *pixt, *pixd;
PIXA      *pixa;

    if ((pix = pixRead(filename)) == NULL) {
        rp->success = FALSE;
        return ERROR_INT_1("pix not made", filename, __func__, 1);
    }
    pixGetDimensions(pix, &w, &h, NULL);
    if (w > MAX_WIDTH) {
        factor = (l_float32)MAX_WIDTH / (l_float32)w;
        pixs = pixScale(pix, factor, factor);
    }
    else
        pixs = pixClone(pix);
    pixDestroy(&pix);

    pixa = pixaCreate(0);

        /* Median cut quantizer (no dither; 5 sigbits) */
    pixaAddPix(pixa, pixs, L_COPY);
    pixc = pixMedianCutQuantGeneral(pixs, 0, 0, 16, 5, 1, 1);
    PixSave32(pixa, pixc, rp);
    pixc = pixMedianCutQuantGeneral(pixs, 0, 0, 128, 5, 1, 1);
    PixSave32(pixa, pixc, rp);
    pixc = pixMedianCutQuantGeneral(pixs, 0, 0, 256, 5, 1, 1);
    PixSave32(pixa, pixc, rp);

        /* Median cut quantizer (with dither; 5 sigbits) */
    pixaAddPix(pixa, pixs, L_COPY);
    pixc = pixMedianCutQuantGeneral(pixs, 1, 0, 16, 5, 1, 1);
    PixSave32(pixa, pixc, rp);
    pixc = pixMedianCutQuantGeneral(pixs, 1, 0, 128, 5, 1, 1);
    PixSave32(pixa, pixc, rp);
    pixc = pixMedianCutQuantGeneral(pixs, 1, 0, 256, 5, 1, 1);
    PixSave32(pixa, pixc, rp);

        /* Median cut quantizer (no dither; 6 sigbits) */
    pixaAddPix(pixa, pixs, L_COPY);
    pixc = pixMedianCutQuantGeneral(pixs, 0, 0, 16, 6, 1, 1);
    PixSave32(pixa, pixc, rp);
    pixc = pixMedianCutQuantGeneral(pixs, 0, 0, 128, 6, 1, 1);
    PixSave32(pixa, pixc, rp);
    pixc = pixMedianCutQuantGeneral(pixs, 0, 0, 256, 6, 1, 1);
    PixSave32(pixa, pixc, rp);

        /* Median cut quantizer (with dither; 6 sigbits) */
    pixaAddPix(pixa, pixs, L_COPY);
    pixc = pixMedianCutQuantGeneral(pixs, 1, 0, 16, 6, 1, 1);
    PixSave32(pixa, pixc, rp);
    pixc = pixMedianCutQuantGeneral(pixs, 1, 0, 128, 6, 1, 1);
    PixSave32(pixa, pixc, rp);
    pixc = pixMedianCutQuantGeneral(pixs, 1, 0, 256, 6, 10, 1);
    PixSave32(pixa, pixc, rp);

        /* Median cut quantizer (mixed color/gray) */
    pixaAddPix(pixa, pixs, L_COPY);
    pixc = pixMedianCutQuantMixed(pixs, 20, 10, 0, 0, 0);
    PixSave32(pixa, pixc, rp);
    pixc = pixMedianCutQuantMixed(pixs, 60, 20, 0, 0, 0);
    PixSave32(pixa, pixc, rp);
    pixc = pixMedianCutQuantMixed(pixs, 180, 40, 0, 0, 0);
    PixSave32(pixa, pixc, rp);

        /* Simple 256 cube octcube quantizer */
    pixaAddPix(pixa, pixs, L_COPY);
    pixc = pixFixedOctcubeQuant256(pixs, 0);  /* no dither */
    PixSave32(pixa, pixc, rp);
    pixc = pixFixedOctcubeQuant256(pixs, 1);  /* dither */
    PixSave32(pixa, pixc, rp);
    pixc = pixFixedOctcubeQuant256(pixs, 1);  /* dither */
    PixSave32(pixa, pixc, rp);

        /* 2-pass octree quantizer */
    pixc = pixOctreeColorQuant(pixs, 128, 0);  /* no dither */
    PixSave32(pixa, pixc, rp);
    pixc = pixOctreeColorQuant(pixs, 240, 0);  /* no dither */
    PixSave32(pixa, pixc, rp);
    pixc = pixOctreeColorQuant(pixs, 128, 1);  /* dither */
    PixSave32(pixa, pixc, rp);
    pixc = pixOctreeColorQuant(pixs, 240, 1);  /* dither */
    PixSave32(pixa, pixc, rp);

        /* Simple adaptive quantization to 4 or 8 bpp, specifying ncolors */
    pixc = pixOctreeQuantNumColors(pixs, 8, 0);    /* fixed: 8 colors */
    PixSave32(pixa, pixc, rp);
    pixc = pixOctreeQuantNumColors(pixs, 16, 0);   /* fixed: 16 colors */
    PixSave32(pixa, pixc, rp);
    pixc = pixOctreeQuantNumColors(pixs, 64, 0);   /* fixed: 64 colors */
    PixSave32(pixa, pixc, rp);
    pixc = pixOctreeQuantNumColors(pixs, 256, 0);   /* fixed: 256 colors */
    PixSave32(pixa, pixc, rp);

        /* Quantize to fully populated octree (RGB) at given level */
    pixc = pixFixedOctcubeQuantGenRGB(pixs, 2);  /* level 2 */
    PixSave32(pixa, pixc, rp);
    pixc = pixFixedOctcubeQuantGenRGB(pixs, 3);  /* level 3 */
    PixSave32(pixa, pixc, rp);
    pixc = pixFixedOctcubeQuantGenRGB(pixs, 4);  /* level 4 */
    PixSave32(pixa, pixc, rp);
    pixc = pixFixedOctcubeQuantGenRGB(pixs, 5);  /* level 5 */
    PixSave32(pixa, pixc, rp);

        /* Generate 32 bpp RGB image with num colors <= 256 */
    pixt = pixOctreeQuantNumColors(pixs, 256, 0);   /* cmapped version */
    pix32 = pixRemoveColormap(pixt, REMOVE_CMAP_BASED_ON_SRC);

        /* Quantize image with few colors at fixed octree leaf level */
    pixc = pixFewColorsOctcubeQuant1(pix32, 2);   /* level 2 */
    PixSave32(pixa, pixc, rp);
    pixc = pixFewColorsOctcubeQuant1(pix32, 3);   /* level 3 */
    PixSave32(pixa, pixc, rp);
    pixc = pixFewColorsOctcubeQuant1(pix32, 4);   /* level 4 */
    PixSave32(pixa, pixc, rp);
    pixc = pixFewColorsOctcubeQuant1(pix32, 5);   /* level 5 */
    PixSave32(pixa, pixc, rp);

        /* Quantize image by population */
    pixc = pixOctreeQuantByPopulation(pixs, 3, 0);  /* level 3, no dither */
    PixSave32(pixa, pixc, rp);
    pixc = pixOctreeQuantByPopulation(pixs, 3, 1);  /* level 3, dither */
    PixSave32(pixa, pixc, rp);
    pixc = pixOctreeQuantByPopulation(pixs, 4, 0);  /* level 4, no dither */
    PixSave32(pixa, pixc, rp);
    pixc = pixOctreeQuantByPopulation(pixs, 4, 1);  /* level 4, dither */
    PixSave32(pixa, pixc, rp);

        /* Mixed color/gray octree quantizer */
    pixaAddPix(pixa, pixt, L_COPY);
    pixc = pixOctcubeQuantMixedWithGray(pix32, 8, 64, 10);  /* max delta = 10 */
    PixSave32(pixa, pixc, rp);
    pixc = pixOctcubeQuantMixedWithGray(pix32, 8, 64, 30);  /* max delta = 30 */
    PixSave32(pixa, pixc, rp);
    pixc = pixOctcubeQuantMixedWithGray(pix32, 8, 64, 50);  /* max delta = 50 */
    PixSave32(pixa, pixc, rp);

        /* Run the high-level converter */
    pixaAddPix(pixa, pixt, L_COPY);
    pixc = pixConvertRGBToColormap(pix32, 1);
    PixSave32(pixa, pixc, rp);

    pixDestroy(&pix32);
    pixDestroy(&pixt);

    pixd = pixaDisplayTiledInColumns(pixa, 4, 1.0, 25, 2);
    pixDisplayWithTitle(pixd, 100, 100, NULL, rp->display);
    snprintf(buf, sizeof(buf), "/tmp/lept/regout/disp.%d.jpg", i);
    pixWrite(buf, pixd, IFF_JFIF_JPEG);

    pixDestroy(&pixs);
    pixDestroy(&pixd);
    pixaDestroy(&pixa);
    return 0;
}

static void
PixSave32(PIXA *pixa, PIX *pixc, L_REGPARAMS *rp)
{
PIX  *pix32;

    pix32 = pixConvertTo32(pixc);
    pixaAddPix(pixa, pix32, L_INSERT);
    regTestWritePixAndCheck(rp, pix32, IFF_JFIF_JPEG);
    pixDestroy(&pixc);
}
