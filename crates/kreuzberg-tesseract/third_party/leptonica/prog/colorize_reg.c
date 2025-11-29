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
 *  colorize_reg.c
 *
 *  This regresstion test demonstrates the detection of red highlight
 *  color in an image, and the generation of a colormapped version
 *  with clean background and colorized highlighting.
 *
 *  The input image is rgb.  Other examples are breviar.32 and amoris.2.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

PIX *TestForRedColor(L_REGPARAMS *rp, const char *fname,
                     l_float32 gold_red, L_BMF *bmf);

l_int32 main(int    argc,
             char **argv)
{
l_int32       irval, igval, ibval;
l_float32     rval, gval, bval, fract, fgfract;
L_BMF        *bmf;
BOX          *box;
BOXA         *boxa;
FPIX         *fpix;
PIX          *pixs, *pix1, *pix2, *pix3, *pix4, *pix5, *pix6, *pix7;
PIX          *pix8, *pix9, *pix10, *pix11, *pix12, *pix13, *pix14, *pix15;
PIXA         *pixa;
L_REGPARAMS  *rp;

#if !defined(HAVE_LIBPNG)
    L_ERROR("This test requires libpng to run.\n", "colorize_reg");
    exit(77);
#endif

    if (regTestSetup(argc, argv, &rp))
              return 1;

    pixa = pixaCreate(0);
    pixs = pixRead("breviar.38.150.jpg");
    pixaAddPix(pixa, pixs, L_CLONE);
    regTestWritePixAndCheck(rp, pixs, IFF_JFIF_JPEG);  /* 0 */
    pixDisplayWithTitle(pixs, 0, 0, "Input image", rp->display);

        /* Extract the blue component, which is small in all the text
         * regions, including in the highlight color region */
    pix1 = pixGetRGBComponent(pixs, COLOR_BLUE);
    pixaAddPix(pixa, pix1, L_CLONE);
    regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 1 */
    pixDisplayWithTitle(pix1, 200, 0, "Blue component", rp->display);

        /* Do a background normalization, with the background set to
         * approximately 200 */
    pix2 = pixBackgroundNormSimple(pix1, NULL, NULL);
    pixaAddPix(pixa, pix2, L_COPY);
    regTestWritePixAndCheck(rp, pix2, IFF_JFIF_JPEG);  /* 2 */
    pixDisplayWithTitle(pix2, 400, 0, "BG normalized to 200", rp->display);

        /* Do a linear transform on the gray pixels, with 50 going to
         * black and 160 going to white.  50 is sufficiently low to
         * make both the red and black print quite dark.  Quantize
         * to a few equally spaced gray levels.  This is the image
         * to which highlight color will be applied. */
    pixGammaTRC(pix2, pix2, 1.0, 50, 160);
    pix3 = pixThresholdOn8bpp(pix2, 7, 1);
    pixaAddPix(pixa, pix3, L_CLONE);
    regTestWritePixAndCheck(rp, pix3, IFF_JFIF_JPEG);  /* 3 */
    pixDisplayWithTitle(pix3, 600, 0, "Basic quantized with white bg",
                        rp->display);

        /* Identify the regions of red text.  First, make a mask
         * consisting of all pixels such that (R-B)/B is larger
         * than 2.0.  This will have all the red, plus a lot of
         * the dark pixels. */
    fpix = pixComponentFunction(pixs, 1.0, 0.0, -1.0, 0.0, 0.0, 1.0);
    pix4 = fpixThresholdToPix(fpix, 2.0);
    pixInvert(pix4, pix4);  /* red plus some dark text */
    pixaAddPix(pixa, pix4, L_CLONE);
    regTestWritePixAndCheck(rp, pix4, IFF_PNG);  /* 4 */
    pixDisplayWithTitle(pix4, 800, 0, "Red plus dark pixels", rp->display);

        /* Make a mask consisting of all the red and background pixels */
    pix5 = pixGetRGBComponent(pixs, COLOR_RED);
    pix6 = pixThresholdToBinary(pix5, 128);
    pixInvert(pix6, pix6);  /* red plus background (white) */

        /* Intersect the two masks to get a mask consisting of pixels
         * that are almost certainly red.  This is the seed. */
    pix7 = pixAnd(NULL, pix4, pix6);  /* red only (seed) */
    pixaAddPix(pixa, pix7, L_COPY);
    regTestWritePixAndCheck(rp, pix7, IFF_PNG);  /* 5 */
    pixDisplayWithTitle(pix7, 0, 600, "Seed for red color", rp->display);

        /* Make the clipping mask by thresholding the image with
         * the background cleaned to white. */
    pix8 =  pixThresholdToBinary(pix2, 230);  /* mask */
    pixaAddPix(pixa, pix8, L_CLONE);
    regTestWritePixAndCheck(rp, pix8, IFF_PNG);  /* 6 */
    pixDisplayWithTitle(pix8, 200, 600, "Clipping mask for red components",
                        rp->display);

        /* Fill into the mask from the seed */
    pixSeedfillBinary(pix7, pix7, pix8, 8);  /* filled: red plus touching */
    regTestWritePixAndCheck(rp, pix7, IFF_PNG);  /* 7 */
    pixDisplayWithTitle(pix7, 400, 600, "Red component mask filled",
                        rp->display);

        /* Small closing on regions to be colored  */
    pix9 = pixMorphSequence(pix7, "c5.1", 0);
    pixaAddPix(pixa, pix9, L_CLONE);
    regTestWritePixAndCheck(rp, pix9, IFF_PNG);  /* 8 */
    pixDisplayWithTitle(pix9, 600, 600,
                        "Components defining regions allowing coloring",
                        rp->display);

        /* Sanity check on amount to be colored.  Only accept images
         * with less than 10% of all the pixels with highlight color */
    pixForegroundFraction(pix9, &fgfract);
    if (fgfract >= 0.10) {
        L_INFO("too much highlighting: fract = %6.3f; removing it\n",
               rp->testname, fgfract);
        pixClearAll(pix9);
        pixSetPixel(pix9, 0, 0, 1);
    }

        /* Get a color to paint that is representative of the
         * actual highlight color in the image.  Scale each
         * color component up from the average by an amount necessary
         * to saturate the red.  Then divide the green and
         * blue components by 2.0.  */
    pixGetAverageMaskedRGB(pixs, pix7, 0, 0, 1, L_MEAN_ABSVAL,
                           &rval, &gval, &bval);
    fract = 255.0 / rval;
    irval = lept_roundftoi(fract * rval);
    igval = lept_roundftoi(fract * gval / 2.0);
    ibval = lept_roundftoi(fract * bval / 2.0);
    lept_stderr("(r,g,b) = (%d,%d,%d)\n", irval, igval, ibval);

        /* Test mask-based colorization on gray and cmapped gray */
    pix10 = pixColorGrayMasked(pix2, pix9, L_PAINT_DARK, 225,
                                irval, igval, ibval);
    pixaAddPix(pixa, pix10, L_CLONE);
    regTestWritePixAndCheck(rp, pix10, IFF_PNG);  /* 9 */
    pixDisplayWithTitle(pix10, 800, 600, "Colorize mask gray", rp->display);
    pixaAddPix(pixa, pixs, L_CLONE);

    pix11 = pixColorGrayMasked(pix3, pix9, L_PAINT_DARK, 225,
                               irval, igval, ibval);
    pixaAddPix(pixa, pix11, L_CLONE);
    regTestWritePixAndCheck(rp, pix11, IFF_PNG);  /* 10 */
    pixDisplayWithTitle(pix11, 900, 600, "Colorize mask cmapped", rp->display);

        /* Get the bounding boxes of the mask components to be colored */
    boxa = pixConnCompBB(pix9, 8);

        /* Test region colorization on gray and cmapped gray */
    pix12 = pixColorGrayRegions(pix2, boxa, L_PAINT_DARK, 220, 0, 255, 0);
    pixaAddPix(pixa, pix12, L_CLONE);
    regTestWritePixAndCheck(rp, pix12, IFF_PNG);  /* 11 */
    pixDisplayWithTitle(pix12, 900, 600, "Colorize boxa gray", rp->display);

    box = boxCreate(200, 200, 250, 350);
    pix13 = pixCopy(NULL, pix2);
    pixColorGray(pix13, box, L_PAINT_DARK, 220, 0, 0, 255);
    pixaAddPix(pixa, pix13, L_CLONE);
    regTestWritePixAndCheck(rp, pix13, IFF_PNG);  /* 12 */
    pixDisplayWithTitle(pix13, 1000, 600, "Colorize box gray", rp->display);

    pix14 = pixThresholdTo4bpp(pix2, 6, 1);
    pix15 = pixColorGrayRegions(pix14, boxa, L_PAINT_DARK, 220, 0, 0, 255);
    pixaAddPix(pixa, pix15, L_CLONE);
    regTestWritePixAndCheck(rp, pix15, IFF_PNG);  /* 13 */
    pixDisplayWithTitle(pix15, 1100, 600, "Colorize boxa cmap", rp->display);

    pixColorGrayCmap(pix14, box, L_PAINT_DARK, 0, 255, 255);
    pixaAddPix(pixa, pix14, L_CLONE);
    regTestWritePixAndCheck(rp, pix14, IFF_PNG);  /* 14 */
    pixDisplayWithTitle(pix14, 1200, 600, "Colorize box cmap", rp->display);
    boxDestroy(&box);

        /* Generate a pdf of the intermediate results */
    lept_mkdir("lept/color");
    L_INFO("Writing to /tmp/lept/color/colorize.pdf\n", rp->testname);
    pixaConvertToPdf(pixa, 90, 1.0, 0, 0, "Colorizing highlighted text",
                     "/tmp/lept/color/colorize.pdf");


    pixaDestroy(&pixa);
    fpixDestroy(&fpix);
    boxDestroy(&box);
    boxaDestroy(&boxa);
    pixDestroy(&pixs);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);
    pixDestroy(&pix5);
    pixDestroy(&pix6);
    pixDestroy(&pix7);
    pixDestroy(&pix8);
    pixDestroy(&pix9);
    pixDestroy(&pix10);
    pixDestroy(&pix11);
    pixDestroy(&pix12);
    pixDestroy(&pix13);
    pixDestroy(&pix14);
    pixDestroy(&pix15);

        /* Test the color detector */
    pixa = pixaCreate(7);
    bmf = bmfCreate(NULL, 4);
    pix1 = TestForRedColor(rp, "brev.06.75.jpg", 1, bmf);  /* 14 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pix1 = TestForRedColor(rp, "brev.10.75.jpg", 0, bmf);  /* 15 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pix1 = TestForRedColor(rp, "brev.14.75.jpg", 1, bmf);  /* 16 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pix1 = TestForRedColor(rp, "brev.20.75.jpg", 1, bmf);  /* 17 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pix1 = TestForRedColor(rp, "brev.36.75.jpg", 0, bmf);  /* 18 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pix1 = TestForRedColor(rp, "brev.53.75.jpg", 1, bmf);  /* 19 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pix1 = TestForRedColor(rp, "brev.56.75.jpg", 1, bmf);  /* 20 */
    pixaAddPix(pixa, pix1, L_INSERT);

        /* Generate a pdf of the color detector results */
    L_INFO("Writing to /tmp/lept/color/colordetect.pdf\n", rp->testname);
    pixaConvertToPdf(pixa, 45, 1.0, 0, 0, "Color detection",
                     "/tmp/lept/color/colordetect.pdf");
    pixaDestroy(&pixa);
    bmfDestroy(&bmf);

    return regTestCleanup(rp);
}


PIX *
TestForRedColor(L_REGPARAMS  *rp,
                const char   *fname,
                l_float32     gold_red,
                L_BMF        *bmf)
{
char       text[64];
PIX       *pix1, *pix2;
l_int32    hasred;
l_float32  ratio;

    pix1 = pixRead(fname);
    pixHasHighlightRed(pix1, 1, 0.0001, 2.5, &hasred, &ratio, NULL);
    regTestCompareValues(rp, gold_red, hasred, 0.0);
    if (hasred)
        snprintf(text, sizeof(text), "Has red: ratio = %6.1f", ratio);
    else
        snprintf(text, sizeof(text), "Does not have red: ratio = %6.1f", ratio);
    pix2 = pixAddSingleTextblock(pix1, bmf, text, 0x0000ff00,
                                 L_ADD_BELOW, NULL);
    pixDestroy(&pix1);
    return pix2;
}

