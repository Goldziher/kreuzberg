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
 * alphaops_reg.c
 *
 *   Tests various functions that use the alpha layer:
 *
 *     (1) Remove and add alpha layers.
 *         Removing is done by blending with a uniform image.
 *         Adding is done by setting all white pixels to transparent,
 *         and grading the alpha layer to opaque depending on
 *         the distance from the nearest transparent pixel.
 *
 *     (2) Tests transparency and cleaning under alpha.
 *
 *     (3) Blending with a uniform color.  Also tests an alternative
 *         way to "blend" to a color: component-wise multiplication by
 *         the color.
 *
 *     (4) Testing RGB and colormapped images with alpha, including
 *         binary and ascii colormap serialization.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"
#include "pix_internal.h"

static PIX *DoBlendTest(PIX *pix, BOX *box, l_uint32 val, l_float32 gamma,
                        l_int32 minval, l_int32 maxval, l_int32 which);

int main(int    argc,
         char **argv)
{
l_uint8      *data;
l_int32       w, h, n1, n2, n, i, minval, maxval;
l_int32       ncolors, rval, gval, bval, equal;
l_int32      *rmap, *gmap, *bmap;
l_uint32      color;
l_float32     gamma;
BOX          *box;
FILE         *fp;
PIX          *pix1, *pix2, *pix3, *pix4, *pix5, *pix6;
PIX          *pixs, *pixb, *pixg, *pixc, *pixd;
PIX          *pixg2, *pixcs1, *pixcs2, *pixd1, *pixd2;
PIXA         *pixa, *pixa2, *pixa3;
PIXCMAP      *cmap, *cmap2;
RGBA_QUAD    *cta;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    /* ------------------------ (1) ----------------------------*/
        /* Blend with a white background */
    pix1 = pixRead("books_logo.png");
    pixDisplayWithTitle(pix1, 100, 0, NULL, rp->display);
    pix2 = pixAlphaBlendUniform(pix1, 0xffffff00);
    pixDisplayWithTitle(pix2, 100, 150, NULL, rp->display);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 0 */
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 1 */

        /* Generate an alpha layer based on the white background */
    pix3 = pixSetAlphaOverWhite(pix2);
    pixSetSpp(pix3, 3);
            /* without alpha */
    pixWrite("/tmp/lept/regout/alphaops.2.png", pix3, IFF_PNG);
    regTestCheckFile(rp, "/tmp/lept/regout/alphaops.2.png");   /* 2 */
    pixSetSpp(pix3, 4);
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 3, with alpha */
    pixDisplayWithTitle(pix3, 100, 300, NULL, rp->display);

        /* Render on a light yellow background */
    pix4 = pixAlphaBlendUniform(pix3, 0xffffe000);
    regTestWritePixAndCheck(rp, pix4, IFF_PNG);  /* 4 */
    pixDisplayWithTitle(pix4, 100, 450, NULL, rp->display);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);

    /* ------------------------ (2) ----------------------------*/
    lept_mkdir("lept/alpha");
        /* Make the transparency (alpha) layer.
         * pixs is the mask.  We turn it into a transparency (alpha)
         * layer by converting to 8 bpp.  A small convolution fuzzes
         * the mask edges so that you don't see the pixels. */
    pixs = pixRead("feyn-fract.tif");
    pixGetDimensions(pixs, &w, &h, NULL);
    pixg = pixConvert1To8(NULL, pixs, 0, 255);
    pixg2 = pixBlockconvGray(pixg, NULL, 1, 1);
    regTestWritePixAndCheck(rp, pixg2, IFF_JFIF_JPEG);  /* 5 */
    pixDisplayWithTitle(pixg2, 0, 0, "alpha", rp->display);

        /* Make the viewable image.
         * pixc is the image that we see where the alpha layer is
         * opaque -- i.e., greater than 0.  Scale it to the same
         * size as the mask.  To visualize what this will look like
         * when displayed over a black background, create the black
         * background image, pixb, and do the blending with pixcs1
         * explicitly using the alpha layer pixg2. */
    pixc = pixRead("tetons.jpg");
    pixcs1 = pixScaleToSize(pixc, w, h);
    regTestWritePixAndCheck(rp, pixcs1, IFF_JFIF_JPEG);  /* 6 */
    pixDisplayWithTitle(pixcs1, 300, 0, "viewable", rp->display);
    pixb = pixCreateTemplate(pixcs1);  /* black */
    pixd1 = pixBlendWithGrayMask(pixb, pixcs1, pixg2, 0, 0);
    regTestWritePixAndCheck(rp, pixd1, IFF_JFIF_JPEG);  /* 7 */
    pixDisplayWithTitle(pixd1, 600, 0, "alpha-blended 1", rp->display);

        /* Embed the alpha layer pixg2 into the color image pixc.
         * Write it out as is.  Then clean pixcs1 (to 0) under the fully
         * transparent part of the alpha layer, and write that result
         * out as well. */
    pixSetRGBComponent(pixcs1, pixg2, L_ALPHA_CHANNEL);
    pixWrite("/tmp/lept/alpha/cs1.png", pixcs1, IFF_PNG);
    pixcs2 = pixSetUnderTransparency(pixcs1, 0, 0);
    pixWrite("/tmp/lept/alpha/cs2.png", pixcs2, IFF_PNG);

        /* What will this look like over a black background?
         * Do the blending explicitly and display.  It should
         * look identical to the blended result pixd1 before cleaning. */
    pixd2 = pixBlendWithGrayMask(pixb, pixcs2, pixg2, 0, 0);
    regTestWritePixAndCheck(rp, pixd2, IFF_JFIF_JPEG);  /* 8 */
    pixDisplayWithTitle(pixd2, 0, 400, "alpha blended 2", rp->display);

        /* Read the two images back, ignoring the transparency layer.
         * The uncleaned image will come back identical to pixcs1.
         * However, the cleaned image will be black wherever
         * the alpha layer was fully transparent.  It will
         * look the same when viewed through the alpha layer,
         * but have much better compression. */
    pix1 = pixRead("/tmp/lept/alpha/cs1.png");  /* just pixcs1 */
    pix2 = pixRead("/tmp/lept/alpha/cs2.png");  /* cleaned under transparent */
    n1 = nbytesInFile("/tmp/lept/alpha/cs1.png");
    n2 = nbytesInFile("/tmp/lept/alpha/cs2.png");
    lept_stderr(" Original: %d bytes\n Cleaned: %d bytes\n", n1, n2);
    regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 9 */
    regTestWritePixAndCheck(rp, pix2, IFF_JFIF_JPEG);  /* 10 */
    pixDisplayWithTitle(pix1, 300, 400, "without alpha", rp->display);
    pixDisplayWithTitle(pix2, 600, 400, "cleaned under transparent",
                        rp->display);

    pixa = pixaCreate(0);
    pixaAddPix(pixa, pixg2, L_INSERT);
    pixaAddPix(pixa, pixcs1, L_INSERT);
    pixaAddPix(pixa, pix1, L_INSERT);
    pixaAddPix(pixa, pixd1, L_INSERT);
    pixaAddPix(pixa, pixd2, L_INSERT);
    pixaAddPix(pixa, pix2, L_INSERT);
    pixd = pixaDisplayTiledInColumns(pixa, 1, 1.0, 20, 2);
    regTestWritePixAndCheck(rp, pixd, IFF_JFIF_JPEG);  /* 11 */
    pixDisplayWithTitle(pixd, 200, 200, "composite", rp->display);
    pixWrite("/tmp/lept/alpha/composite.png", pixd, IFF_JFIF_JPEG);
    pixDestroy(&pixd);
    pixaDestroy(&pixa);
    pixDestroy(&pixs);
    pixDestroy(&pixb);
    pixDestroy(&pixg);
    pixDestroy(&pixc);
    pixDestroy(&pixcs2);
    pixDestroy(&pixd);

    /* ------------------------ (3) ----------------------------*/
    color = 0xffffa000;
    gamma = 1.0;
    minval = 0;
    maxval = 200;
    box = boxCreate(0, 85, 600, 100);
    pixa = pixaCreate(6);
    pix1 = pixRead("blend-green1.jpg");
    pixaAddPix(pixa, pix1, L_INSERT);
    pix1 = pixRead("blend-green2.png");
    pixaAddPix(pixa, pix1, L_INSERT);
    pix1 = pixRead("blend-green3.png");
    pixaAddPix(pixa, pix1, L_INSERT);
    pix1 = pixRead("blend-orange.jpg");
    pixaAddPix(pixa, pix1, L_INSERT);
    pix1 = pixRead("blend-yellow.jpg");
    pixaAddPix(pixa, pix1, L_INSERT);
    pix1 = pixRead("blend-red.png");
    pixaAddPix(pixa, pix1, L_INSERT);
    n = pixaGetCount(pixa);
    pixa2 = pixaCreate(n);
    pixa3 = pixaCreate(n);
    for (i = 0; i < n; i++) {
        pix1 = pixaGetPix(pixa, i, L_CLONE);
        pix2 = DoBlendTest(pix1, box, color, gamma, minval, maxval, 1);
        regTestWritePixAndCheck(rp, pix2, IFF_JFIF_JPEG);  /* 12, 14, ... 22 */
        pixDisplayWithTitle(pix2, 150 * i, 0, NULL, rp->display);
        pixaAddPix(pixa2, pix2, L_INSERT);
        pix2 = DoBlendTest(pix1, box, color, gamma, minval, maxval, 2);
        regTestWritePixAndCheck(rp, pix2, IFF_JFIF_JPEG);  /* 13, 15, ... 23 */
        pixDisplayWithTitle(pix2, 150 * i, 200, NULL, rp->display);
        pixaAddPix(pixa3, pix2, L_INSERT);
        pixDestroy(&pix1);
    }
    if (rp->display) {
        pixaConvertToPdf(pixa2, 0, 0.75, L_FLATE_ENCODE, 0, "blend 1 test",
                         "/tmp/lept/alpha/blend1.pdf");
        pixaConvertToPdf(pixa3, 0, 0.75, L_FLATE_ENCODE, 0, "blend 2 test",
                         "/tmp/lept/alpha/blend2.pdf");
    }
    pixaDestroy(&pixa);
    pixaDestroy(&pixa2);
    pixaDestroy(&pixa3);
    boxDestroy(&box);

    /* ------------------------ (4) ----------------------------*/
        /* Use one image as the alpha component for a second image */
    pix1 = pixRead("test24.jpg");
    pix2 = pixRead("marge.jpg");
    pix3 = pixScale(pix2, 1.9, 2.2);
    pix4 = pixConvertTo8(pix3, 0);
    pixSetRGBComponent(pix1, pix4, L_ALPHA_CHANNEL);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 24 */
    pixDisplayWithTitle(pix1, 600, 0, NULL, rp->display);

        /* Set the alpha value in a colormap to bval */
    pix5 = pixOctreeColorQuant(pix1, 128, 0);
    cmap = pixGetColormap(pix5);
    pixcmapToArrays(cmap, &rmap, &gmap, &bmap, NULL);
    n = pixcmapGetCount(cmap);
    for (i = 0; i < n; i++) {
        pixcmapGetColor(cmap, i, &rval, &gval, &bval);
        cta = (RGBA_QUAD *)cmap->array;
        cta[i].alpha = bval;
    }

        /* Test binary serialization/deserialization of colormap with alpha */
    pixcmapSerializeToMemory(cmap, 4, &ncolors, &data);
    cmap2 = pixcmapDeserializeFromMemory(data, 4, ncolors);
    cmapEqual(cmap, cmap2, 4, &equal);
    regTestCompareValues(rp, TRUE, equal, 0.0);  /* 25 */
    pixcmapDestroy(&cmap2);
    lept_free(data);

        /* Test ascii serialization/deserialization of colormap with alpha */
    if ((fp = lept_fopen("/tmp/lept/alpha/cmap.4", "wb")) != NULL) {
        pixcmapWriteStream(fp, cmap);
        lept_fclose(fp);
    }
    if ((fp = lept_fopen("/tmp/lept/alpha/cmap.4", "rb")) != NULL) {
        cmap2 = pixcmapReadStream(fp);
        lept_fclose(fp);
    }
    cmapEqual(cmap, cmap2, 4, &equal);
    regTestCompareValues(rp, TRUE, equal, 0.0);  /* 26 */
    pixcmapDestroy(&cmap2);

        /* Test r/w for cmapped pix with non-opaque alpha */
    pixDisplayWithTitle(pix5, 900, 0, NULL, rp->display);
    regTestWritePixAndCheck(rp, pix5, IFF_PNG);  /* 27 */
    pixWrite("/tmp/lept/alpha/fourcomp.png", pix5, IFF_PNG);
    pix6 = pixRead("/tmp/lept/alpha/fourcomp.png");
    regTestComparePix(rp, pix5, pix6);  /* 28 */
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);
    pixDestroy(&pix5);
    pixDestroy(&pix6);
    lept_free(rmap);
    lept_free(gmap);
    lept_free(bmap);
    return regTestCleanup(rp);
}


    /* Use which == 1 to test alpha blending; which == 2 to test "blending"
     * by pixel multiplication.   This generates a composite of 5 images:
     * the original, blending over a box at the bottom (2 ways), and
     * multiplying over a box at the bottom (2 ways). */
static PIX *
DoBlendTest(PIX *pix, BOX *box, l_uint32 color, l_float32 gamma,
            l_int32 minval, l_int32 maxval, l_int32 which)
{
PIX   *pix1, *pix2, *pix3, *pixd;
PIXA  *pixa;
  pixa = pixaCreate(5);
  pix1 = pixRemoveColormap(pix, REMOVE_CMAP_TO_FULL_COLOR);
  pix2 = pixCopy(NULL, pix1);
  pixaAddPix(pixa, pix2, L_COPY);
  if (which == 1)
      pix3 =  pixBlendBackgroundToColor(NULL, pix2, box, color,
                                        gamma, minval, maxval);
  else
      pix3 =  pixMultiplyByColor(NULL, pix2, box, color);
  pixaAddPix(pixa, pix3, L_INSERT);
  if (which == 1)
      pixBlendBackgroundToColor(pix2, pix2, box, color,
                                gamma, minval, maxval);
  else
      pixMultiplyByColor(pix2, pix2, box, color);
  pixaAddPix(pixa, pix2, L_INSERT);
  pix2 = pixCopy(NULL, pix1);
  if (which == 1)
      pix3 =  pixBlendBackgroundToColor(NULL, pix2, NULL, color,
                                        gamma, minval, maxval);
  else
      pix3 =  pixMultiplyByColor(NULL, pix2, NULL, color);
  pixaAddPix(pixa, pix3, L_INSERT);
  if (which == 1)
      pixBlendBackgroundToColor(pix2, pix2, NULL, color,
                                gamma, minval, maxval);
  else
      pixMultiplyByColor(pix2, pix2, NULL, color);
  pixaAddPix(pixa, pix2, L_INSERT);

  pixd = pixaDisplayTiledInRows(pixa, 32, 800, 1.0, 0, 30, 2);
  pixDestroy(&pix1);
  pixaDestroy(&pixa);
  return pixd;
}
