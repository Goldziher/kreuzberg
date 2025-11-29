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
 * subpixel_reg.c
 *
 *   Regression test for subpixel scaling.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

void AddTextAndSave(PIXA *pixa, PIX *pixs,
                    L_BMF *bmf, const char *textstr,
                    l_int32 location, l_uint32 val);

const char  *textstr[] =
           {"Downscaled with sharpening",
            "Subpixel scaling; horiz R-G-B",
            "Subpixel scaling; horiz B-G-R",
            "Subpixel scaling; vert R-G-B",
            "Subpixel scaling; vert B-G-R"};

int main(int    argc,
         char **argv)
{
l_float32     scalefact;
L_BMF        *bmf, *bmftop;
L_KERNEL     *kel, *kelx, *kely;
PIX          *pixs, *pixg, *pixt, *pixd;
PIX          *pix1, *pix2, *pix3, *pix4, *pix5, *pix6, *pix7, *pix8;
PIXA         *pixa;
L_REGPARAMS  *rp;

#if !defined(HAVE_LIBPNG)
    L_ERROR("This test requires libpng to run.\n", "subpixel_reg");
    exit(77);
#endif

    if (regTestSetup(argc, argv, &rp))
        return 1;

    /* ----------------- Test on 8 bpp grayscale ---------------------*/
    pixa = pixaCreate(5);
    bmf = bmfCreate("./fonts", 6);
    bmftop = bmfCreate("./fonts", 10);
    pixs = pixRead("lucasta.047.jpg");
    pixg = pixScale(pixs, 0.4, 0.4);  /* 8 bpp grayscale */
    pix1 = pixConvertTo32(pixg);  /* 32 bpp rgb */
    AddTextAndSave(pixa, pix1, bmf, textstr[0], L_ADD_BELOW, 0xff000000);
    pix2 = pixConvertGrayToSubpixelRGB(pixs, 0.4, 0.4, L_SUBPIXEL_ORDER_RGB);
    AddTextAndSave(pixa, pix2, bmf, textstr[1], L_ADD_BELOW, 0x00ff0000);
    pix3 = pixConvertGrayToSubpixelRGB(pixs, 0.4, 0.4, L_SUBPIXEL_ORDER_BGR);
    AddTextAndSave(pixa, pix3, bmf, textstr[2], L_ADD_BELOW, 0x0000ff00);
    pix4 = pixConvertGrayToSubpixelRGB(pixs, 0.4, 0.4, L_SUBPIXEL_ORDER_VRGB);
    AddTextAndSave(pixa, pix4, bmf, textstr[3], L_ADD_BELOW, 0x00ff0000);
    pix5 = pixConvertGrayToSubpixelRGB(pixs, 0.4, 0.4, L_SUBPIXEL_ORDER_VBGR);
    AddTextAndSave(pixa, pix5, bmf, textstr[4], L_ADD_BELOW, 0x0000ff00);

    pixt = pixaDisplayTiledInColumns(pixa, 5, 1.0, 30, 2);
    pixd = pixAddSingleTextblock(pixt, bmftop,
                                 "Regression test for subpixel scaling: gray",
                                 0xff00ff00, L_ADD_ABOVE, NULL);
    regTestWritePixAndCheck(rp, pixd, IFF_JFIF_JPEG);  /* 0 */
    pixDisplayWithTitle(pixd, 50, 50, NULL, rp->display);
    pixaDestroy(&pixa);
    pixDestroy(&pixs);
    pixDestroy(&pixg);
    pixDestroy(&pixt);
    pixDestroy(&pixd);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);
    pixDestroy(&pix5);


    /* ----------------- Test on 32 bpp rgb ---------------------*/
    pixa = pixaCreate(5);
    pixs = pixRead("fish24.jpg");
    pix1 = pixScale(pixs, 0.4, 0.4);
    AddTextAndSave(pixa, pix1, bmf, textstr[0], L_ADD_BELOW, 0xff000000);
    pix2 = pixConvertToSubpixelRGB(pixs, 0.4, 0.4, L_SUBPIXEL_ORDER_RGB);
    AddTextAndSave(pixa, pix2, bmf, textstr[1], L_ADD_BELOW, 0x00ff0000);
    pix3 = pixConvertToSubpixelRGB(pixs, 0.4, 0.35, L_SUBPIXEL_ORDER_BGR);
    AddTextAndSave(pixa, pix3, bmf, textstr[2], L_ADD_BELOW, 0x0000ff00);
    pix4 = pixConvertToSubpixelRGB(pixs, 0.4, 0.45, L_SUBPIXEL_ORDER_VRGB);
    AddTextAndSave(pixa, pix4, bmf, textstr[3], L_ADD_BELOW, 0x00ff0000);
    pix5 = pixConvertToSubpixelRGB(pixs, 0.4, 0.4, L_SUBPIXEL_ORDER_VBGR);
    AddTextAndSave(pixa, pix5, bmf, textstr[4], L_ADD_BELOW, 0x0000ff00);

    pixt = pixaDisplayTiledInColumns(pixa, 5, 1.0, 30, 2);
    pixd = pixAddSingleTextblock(pixt, bmftop,
                                 "Regression test for subpixel scaling: color",
                                 0xff00ff00, L_ADD_ABOVE, NULL);
    regTestWritePixAndCheck(rp, pixd, IFF_JFIF_JPEG);  /* 1 */
    pixDisplayWithTitle(pixd, 50, 350, NULL, rp->display);
    pixaDestroy(&pixa);
    pixDestroy(&pixs);
    pixDestroy(&pixt);
    pixDestroy(&pixd);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);
    pixDestroy(&pix5);
    bmfDestroy(&bmf);
    bmfDestroy(&bmftop);


    /* --------------- Test on images that are initially 1 bpp ------------*/
    /*   For these, it is better to apply a lowpass filter before scaling  */
        /* Normal scaling of 8 bpp grayscale */
    scalefact = 800. / 2320.;
    pixs = pixRead("patent.png");   /* sharp, 300 ppi, 1 bpp image */
    pix1 = pixConvertTo8(pixs, FALSE);  /* use 8 bpp input */
    pix2 = pixScale(pix1, scalefact, scalefact);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 2 */

        /* Subpixel scaling; bad because there is very little aliasing. */
    pix3 = pixConvertToSubpixelRGB(pix1, scalefact, scalefact,
                                   L_SUBPIXEL_ORDER_RGB);
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 3 */

       /* Get same (bad) result doing subpixel rendering on RGB input */
    pix4 = pixConvertTo32(pixs);
    pix5 = pixConvertToSubpixelRGB(pix4, scalefact, scalefact,
                                   L_SUBPIXEL_ORDER_RGB);
    regTestComparePix(rp, pix3, pix5);  /* 4 */
    regTestWritePixAndCheck(rp, pix5, IFF_PNG);  /* 5 */

        /* Now apply a small lowpass filter before scaling. */
    makeGaussianKernelSep(2, 2, 1.0, 1.0, &kelx, &kely);
    startTimer();
    pix6 = pixConvolveSep(pix1, kelx, kely, 8, 1);  /* normalized */
    lept_stderr("Time sep: %7.3f\n", stopTimer());
    regTestWritePixAndCheck(rp, pix6, IFF_PNG);  /* 6 */

        /* Get same lowpass result with non-separated convolution */
    kel = makeGaussianKernel(2, 2, 1.0, 1.0);
    startTimer();
    pix7 = pixConvolve(pix1, kel, 8, 1);  /* normalized */
    lept_stderr("Time non-sep: %7.3f\n", stopTimer());
    regTestComparePix(rp, pix6, pix7);  /* 7 */

        /* Now do the subpixel scaling on this slightly blurred image */
    pix8 = pixConvertToSubpixelRGB(pix6, scalefact, scalefact,
                                   L_SUBPIXEL_ORDER_RGB);
    regTestWritePixAndCheck(rp, pix8, IFF_PNG);  /* 8 */

    kernelDestroy(&kelx);
    kernelDestroy(&kely);
    kernelDestroy(&kel);
    pixDestroy(&pixs);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);
    pixDestroy(&pix5);
    pixDestroy(&pix6);
    pixDestroy(&pix7);
    pixDestroy(&pix8);
    return regTestCleanup(rp);
}


void
AddTextAndSave(PIXA        *pixa,
               PIX         *pixs,
               L_BMF       *bmf,
               const char  *textstr,
               l_int32      location,
               l_uint32     val)
{
l_int32  n, ovf;
PIX     *pixt;

    pixt = pixAddSingleTextblock(pixs, bmf, textstr, val, location, &ovf);
    n = pixaGetCount(pixa);
    pixaAddPix(pixa, pixt, L_INSERT);
    if (ovf) lept_stderr("Overflow writing text in image %d\n", n + 1);
    return;
}
