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
 *  texturefill_reg.c
 *
 *  This demonstrates a method for filling in a region using nearby
 *  patches and mirrored tiling to generate the texture.
 *
 *  For each of a set of possible tiles to use, convert to gray
 *  and compute the mean and standard deviation of the intensity.
 *  Then determine the specific tile to use for filling by selecting
 *  the one that (1) has a mean value within 1.0 stdev of the median
 *  of average intensities, and (2) of that set has the smallest
 *  standard deviation of intensity.
 *
 *  We can choose tiles looking either horizontally or vertically
 *  away from the region to be textured, or both.  If both, the
 *  selected tiles are blended before painting the resulting
 *  texture through a mask.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

   /* Designed to work with amoris.2.150.jpg  */
static PIX *MakeReplacementMask(PIX *pixs);

l_int32 main(int    argc,
             char **argv)
{
l_int32       bx, by, bw, bh;
l_uint32      pixval;
BOX          *box1, *box2;
BOXA         *boxa;
PIX          *pixs, *pixm, *pixd;
PIX          *pix0, *pix1, *pix2, *pix3, *pix4, *pix5, *pix6;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

        /* Find a mask for repainting pixels */
    pixs = pixRead("amoris.2.150.jpg");
    pix1 = MakeReplacementMask(pixs);
    boxa = pixConnCompBB(pix1, 8);
    box1 = boxaGetBox(boxa, 0, L_COPY);
    boxaDestroy(&boxa);

    /*--------------------------------------------------------*
     *                Show the individual steps               *
     *--------------------------------------------------------*/
        /* Locate a good tile to use */
    pixFindRepCloseTile(pixs, box1, L_VERT, 20, 30, 7, &box2, 1);
    pix0 = pixCopy(NULL, pix1);
    pixRenderBox(pix0, box2, 2, L_SET_PIXELS);

        /* Make a patch using this tile */
    boxGetGeometry(box1, &bx, &by, &bw, &bh);
    pix2 = pixClipRectangle(pixs, box2, NULL);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 0 */
    pixDisplayWithTitle(pix2, 400, 100, NULL, rp->display);
    pix3 = pixMirroredTiling(pix2, bw, bh);
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 1 */
    pixDisplayWithTitle(pix3, 1000, 0, NULL, rp->display);

        /* Paint the patch through the mask */
    pixd = pixCopy(NULL, pixs);
    pixm = pixClipRectangle(pix1, box1, NULL);
    pixCombineMaskedGeneral(pixd, pix3, pixm, bx, by);
    regTestWritePixAndCheck(rp, pixd, IFF_JFIF_JPEG);  /* 2 */
    pixDisplayWithTitle(pixd, 0, 0, NULL, rp->display);
    boxDestroy(&box2);
    pixDestroy(&pixm);
    pixDestroy(&pixd);
    pixDestroy(&pix2);

        /* Blend two patches and then overlay.  Use the previous
         * tile found vertically and a new one found horizontally. */
    pixFindRepCloseTile(pixs, box1, L_HORIZ, 20, 30, 7, &box2, 1);
    pixRenderBox(pix0, box2, 2, L_SET_PIXELS);
    regTestWritePixAndCheck(rp, pix0, IFF_TIFF_G4);  /* 3 */
    pixDisplayWithTitle(pix0, 100, 100, NULL, rp->display);
    pix2 = pixClipRectangle(pixs, box2, NULL);
    pix4 = pixMirroredTiling(pix2, bw, bh);
    regTestWritePixAndCheck(rp, pix4, IFF_PNG);  /* 4 */
    pixDisplayWithTitle(pix4, 1100, 0, NULL, rp->display);
    pix5 = pixBlend(pix3, pix4, 0, 0, 0.5);
    regTestWritePixAndCheck(rp, pix5, IFF_PNG);  /* 5 */
    pixDisplayWithTitle(pix5, 1200, 0, NULL, rp->display);
    pix6 = pixClipRectangle(pix1, box1, NULL);
    pixd = pixCopy(NULL, pixs);
    pixCombineMaskedGeneral(pixd, pix5, pix6, bx, by);
    regTestWritePixAndCheck(rp, pixd, IFF_JFIF_JPEG);  /* 6 */
    pixDisplayWithTitle(pixd, 700, 200, NULL, rp->display);
    boxDestroy(&box2);
    pixDestroy(&pixd);
    pixDestroy(&pix0);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);
    pixDestroy(&pix5);
    pixDestroy(&pix6);

    /*--------------------------------------------------------*
     *          Show painting from a color near region        *
     *--------------------------------------------------------*/
    pix2 = pixCopy(NULL, pixs);
    pixGetColorNearMaskBoundary(pix2, pix1, box1, 20, &pixval, 0);
    pix3 = pixClipRectangle(pix1, box1, NULL);
    boxGetGeometry(box1, &bx, &by, NULL, NULL);
    pixSetMaskedGeneral(pix2, pix3, pixval, bx, by);
    regTestWritePixAndCheck(rp, pix2, IFF_JFIF_JPEG);  /* 7 */
    pixDisplayWithTitle(pix2, 0, 0, NULL, rp->display);
    boxDestroy(&box1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);

    /*--------------------------------------------------------*
     *             Use the higher-level function              *
     *--------------------------------------------------------*/
        /* Use various tile selections and tile blending with one component */
    pix2 = pixCopy(NULL, pixs);
    pix3 = pixCopy(NULL, pixs);
    pix4 = pixCopy(NULL, pixs);
    pixPaintSelfThroughMask(pix2, pix1, 0, 0, L_HORIZ, 30, 50, 5, 10);
    pixPaintSelfThroughMask(pix3, pix1, 0, 0, L_VERT, 30, 50, 5, 0);
    pixPaintSelfThroughMask(pixs, pix1, 0, 0, L_BOTH_DIRECTIONS, 30, 50, 5, 20);
    regTestWritePixAndCheck(rp, pix2, IFF_JFIF_JPEG);  /* 8 */
    regTestWritePixAndCheck(rp, pix3, IFF_JFIF_JPEG);  /* 9 */
    regTestWritePixAndCheck(rp, pixs, IFF_JFIF_JPEG);  /* 10 */
    pixDisplayWithTitle(pix2, 300, 0, NULL, rp->display);
    pixDisplayWithTitle(pix3, 500, 0, NULL, rp->display);
    pixDisplayWithTitle(pixs, 700, 0, NULL, rp->display);

        /* Test with two components; */
    pix5 = pixFlipLR(NULL, pix1);
    pixOr(pix5, pix5, pix1);
    pixPaintSelfThroughMask(pix4, pix5, 0, 0, L_BOTH_DIRECTIONS, 50, 100, 5, 9);
    regTestWritePixAndCheck(rp, pix4, IFF_JFIF_JPEG);  /* 11 */
    pixDisplayWithTitle(pix4, 900, 0, NULL, rp->display);
    pixDestroy(&pixs);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);
    pixDestroy(&pix5);

    return regTestCleanup(rp);
}


PIX *
MakeReplacementMask(PIX  *pixs)
{
PIX  *pix1, *pix2, *pix3, *pix4;

    pix1 = pixMaskOverColorPixels(pixs, 95, 3);
    pix2 = pixMorphSequence(pix1, "o15.15", 0);
    pixSeedfillBinary(pix2, pix2, pix1, 8);
    pix3 = pixMorphSequence(pix2, "c15.15 + d61.31", 0);
    pix4 = pixRemoveBorderConnComps(pix3, 8);
    pixXor(pix4, pix4, pix3);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    return pix4;
}
