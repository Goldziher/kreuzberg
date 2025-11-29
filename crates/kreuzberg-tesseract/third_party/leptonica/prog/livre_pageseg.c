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
 * livre_pageseg.c
 *
 *    This gives examples of the use of binary morphology for
 *    some simple and fast document segmentation operations.
 *
 *    The operations are carried out at 2x reduction.
 *    For images scanned at 300 ppi, this is typically
 *    high enough resolution for accurate results.
 *
 *    This generates several of the figures used in Chapter 18 of
 *    "Mathematical morphology: from theory to applications",
 *    edited by Laurent Najman and Hugues Talbot.  Published by
 *    Hermes Scientific Publishing, Ltd, 2010.
 *
 *    Use pageseg*.tif input images.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

    /* Control the display output */
#define   DFLAG        0


l_int32 DoPageSegmentation(PIX *pixs, l_int32 which);

int main(int    argc,
         char **argv)
{
char    *filein;
l_int32  i;
PIX     *pixs;   /* input image should be at least 300 ppi */

    if (argc != 2)
        return ERROR_INT(" Syntax:  livre_pageseg filein", __func__, 1);
    filein = argv[1];
    setLeptDebugOK(1);

    if ((pixs = pixRead(filein)) == NULL)
        return ERROR_INT("pix not made", __func__, 1);

    for (i = 1; i <= 4; i++)
        DoPageSegmentation(pixs, i);
    pixDestroy(&pixs);
    return 0;
}


l_int32
DoPageSegmentation(PIX     *pixs,   /* should be at least 300 ppi */
                   l_int32  which)  /* 1, 2, 3, 4 */
{
char      buf[256];
l_int32   zero;
BOXA     *boxatm, *boxahm;
PIX      *pixr;   /* image reduced to 150 ppi */
PIX      *pixhs;  /* image of halftone seed, 150 ppi */
PIX      *pixm;   /* image of mask of components, 150 ppi */
PIX      *pixhm1; /* image of halftone mask, 150 ppi */
PIX      *pixhm2; /* image of halftone mask, 300 ppi */
PIX      *pixht;  /* image of halftone components, 150 ppi */
PIX      *pixnht; /* image without halftone components, 150 ppi */
PIX      *pixi;   /* inverted image, 150 ppi */
PIX      *pixvws; /* image of vertical whitespace, 150 ppi */
PIX      *pixm1;  /* image of closed textlines, 150 ppi */
PIX      *pixm2;  /* image of refined text line mask, 150 ppi */
PIX      *pixm3;  /* image of refined text line mask, 300 ppi */
PIX      *pixb1;  /* image of text block mask, 150 ppi */
PIX      *pixb2;  /* image of text block mask, 300 ppi */
PIX      *pixnon; /* image of non-text or halftone, 150 ppi */
PIX      *pix1, *pix2, *pix3;
PIXA     *pixa;
PIXCMAP  *cmap;
PTAA     *ptaa;
l_int32   ht_flag = 0;
l_int32   ws_flag = 0;
l_int32   text_flag = 0;
l_int32   block_flag = 0;

    if (which == 1)
        ht_flag = 1;
    else if (which == 2)
        ws_flag = 1;
    else if (which == 3)
        text_flag = 1;
    else if (which == 4)
        block_flag = 1;
    else
        return ERROR_INT("invalid parameter: not in [1...4]", __func__, 1);

    pixa = pixaCreate(0);
    lept_mkdir("lept/livre");

        /* Reduce to 150 ppi */
    pix1 = pixScaleToGray2(pixs);
    if (ws_flag || ht_flag || block_flag) pixaAddPix(pixa, pix1, L_COPY);
    if (which == 1)
        pixWrite("/tmp/lept/livre/orig.gray.150.png", pix1, IFF_PNG);
    pixDestroy(&pix1);
    pixr = pixReduceRankBinaryCascade(pixs, 1, 0, 0, 0);

        /* Get seed for halftone parts */
    pix1 = pixReduceRankBinaryCascade(pixr, 4, 4, 3, 0);
    pix2 = pixOpenBrick(NULL, pix1, 5, 5);
    pixhs = pixExpandBinaryPower2(pix2, 8);
    if (ht_flag) pixaAddPix(pixa, pixhs, L_COPY);
    if (which == 1)
        pixWrite("/tmp/lept/livre/htseed.150.png", pixhs, IFF_PNG);
    pixDestroy(&pix1);
    pixDestroy(&pix2);

        /* Get mask for connected regions */
    pixm = pixCloseSafeBrick(NULL, pixr, 4, 4);
    if (ht_flag) pixaAddPix(pixa, pixm, L_COPY);
    if (which == 1)
        pixWrite("/tmp/lept/livre/ccmask.150.png", pixm, IFF_PNG);

        /* Fill seed into mask to get halftone mask */
    pixhm1 = pixSeedfillBinary(NULL, pixhs, pixm, 4);
    if (ht_flag) pixaAddPix(pixa, pixhm1, L_COPY);
    if (which == 1) pixWrite("/tmp/lept/livre/htmask.150.png", pixhm1, IFF_PNG);
    pixhm2 = pixExpandBinaryPower2(pixhm1, 2);

        /* Extract halftone stuff */
    pixht = pixAnd(NULL, pixhm1, pixr);
    if (which == 1) pixWrite("/tmp/lept/livre/ht.150.png", pixht, IFF_PNG);

        /* Extract non-halftone stuff */
    pixnht = pixXor(NULL, pixht, pixr);
    if (text_flag) pixaAddPix(pixa, pixnht, L_COPY);
    if (which == 1) pixWrite("/tmp/lept/livre/text.150.png", pixnht, IFF_PNG);
    pixZero(pixht, &zero);
    if (zero)
        lept_stderr("No halftone parts found\n");
    else
        lept_stderr("Halftone parts found\n");

        /* Get bit-inverted image */
    pixi = pixInvert(NULL, pixnht);
    if (ws_flag) pixaAddPix(pixa, pixi, L_COPY);
    if (which == 1) pixWrite("/tmp/lept/livre/invert.150.png", pixi, IFF_PNG);

        /* The whitespace mask will break textlines where there
         * is a large amount of white space below or above.
         * We can prevent this by identifying regions of the
         * inverted image that have large horizontal (bigger than
         * the separation between columns) and significant
         * vertical extent (bigger than the separation between
         * textlines), and subtracting this from the whitespace mask. */
    pix1 = pixMorphCompSequence(pixi, "o80.60", 0);
    pix2 = pixSubtract(NULL, pixi, pix1);
    if (ws_flag) pixaAddPix(pixa, pix2, L_COPY);
    pixDestroy(&pix1);

        /* Identify vertical whitespace by opening inverted image */
    pix3 = pixOpenBrick(NULL, pix2, 5, 1);  /* removes thin vertical lines */
    pixvws = pixOpenBrick(NULL, pix3, 1, 200);  /* gets long vertical lines */
    if (text_flag || ws_flag) pixaAddPix(pixa, pixvws, L_COPY);
    if (which == 1) pixWrite("/tmp/lept/livre/vertws.150.png", pixvws, IFF_PNG);
    pixDestroy(&pix2);
    pixDestroy(&pix3);

        /* Get proto (early processed) text line mask. */
        /* First close the characters and words in the textlines */
    pixm1 = pixCloseSafeBrick(NULL, pixnht, 30, 1);
    if (text_flag) pixaAddPix(pixa, pixm1, L_COPY);
    if (which == 1)
        pixWrite("/tmp/lept/livre/textmask1.150.png", pixm1, IFF_PNG);

        /* Next open back up the vertical whitespace corridors */
    pixm2 = pixSubtract(NULL, pixm1, pixvws);
    if (which == 1)
        pixWrite("/tmp/lept/livre/textmask2.150.png", pixm2, IFF_PNG);

        /* Do a small opening to remove noise */
    pixOpenBrick(pixm2, pixm2, 3, 3);
    if (text_flag) pixaAddPix(pixa, pixm2, L_COPY);
    if (which == 1)
         pixWrite("/tmp/lept/livre/textmask3.150.png", pixm2, IFF_PNG);
    pixm3 = pixExpandBinaryPower2(pixm2, 2);

        /* Join pixels vertically to make text block mask */
    pixb1 = pixMorphSequence(pixm2, "c1.10 + o4.1", 0);
    if (block_flag) pixaAddPix(pixa, pixb1, L_COPY);
    if (which == 1)
        pixWrite("/tmp/lept/livre/textblock1.150.png", pixb1, IFF_PNG);

        /* Solidify the textblock mask and remove noise:
         *  (1) For each c.c., close the blocks and dilate slightly
         *      to form a solid mask.
         *  (2) Small horizontal closing between components
         *  (3) Open the white space between columns, again
         *  (4) Remove small components */
    pix1 = pixMorphSequenceByComponent(pixb1, "c30.30 + d3.3", 8, 0, 0, NULL);
    pixCloseSafeBrick(pix1, pix1, 10, 1);
    if (block_flag) pixaAddPix(pixa, pix1, L_COPY);
    pix2 = pixSubtract(NULL, pix1, pixvws);
    pix3 = pixSelectBySize(pix2, 25, 5, 8, L_SELECT_IF_BOTH,
                            L_SELECT_IF_GTE, NULL);
    if (block_flag) pixaAddPix(pixa, pix3, L_COPY);
    if (which == 1)
        pixWrite("/tmp/lept/livre/textblock2.150.png", pix3, IFF_PNG);
    pixb2 = pixExpandBinaryPower2(pix3, 2);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);

        /* Identify the outlines of each textblock */
    ptaa = pixGetOuterBordersPtaa(pixb2);
    pix1 = pixRenderRandomCmapPtaa(pixb2, ptaa, 1, 8, 1);
    cmap = pixGetColormap(pix1);
    pixcmapResetColor(cmap, 0, 130, 130, 130);  /* set interior to gray */
    if (which == 1)
        pixWrite("/tmp/lept/livre/textblock3.300.png", pix1, IFF_PNG);
    pixDisplayWithTitle(pix1, 480, 360, "textblock mask with outlines", DFLAG);
    ptaaDestroy(&ptaa);
    pixDestroy(&pix1);

        /* Fill line mask (as seed) into the original */
    pix1 = pixSeedfillBinary(NULL, pixm3, pixs, 8);
    pixOr(pixm3, pixm3, pix1);
    pixDestroy(&pix1);
    if (which == 1)
        pixWrite("/tmp/lept/livre/textmask.300.png", pixm3, IFF_PNG);
    pixDisplayWithTitle(pixm3, 480, 360, "textline mask 4", DFLAG);

        /* Fill halftone mask (as seed) into the original */
    pix1 = pixSeedfillBinary(NULL, pixhm2, pixs, 8);
    pixOr(pixhm2, pixhm2, pix1);
    pixDestroy(&pix1);
    if (which == 1)
        pixWrite("/tmp/lept/livre/htmask.300.png", pixhm2, IFF_PNG);
    pixDisplayWithTitle(pixhm2, 520, 390, "halftonemask 2", DFLAG);

        /* Find objects that are neither text nor halftones */
    pix1 = pixSubtract(NULL, pixs, pixm3);  /* remove text pixels */
    pixnon = pixSubtract(NULL, pix1, pixhm2);  /* remove halftone pixels */
    pixDestroy(&pix1);
    if (which == 1)
        pixWrite("/tmp/lept/livre/other.300.png", pixnon, IFF_PNG);
    pixDisplayWithTitle(pixnon, 540, 420, "other stuff", DFLAG);

        /* Write out b.b. for text line mask and halftone mask components */
    boxatm = pixConnComp(pixm3, NULL, 4);
    boxahm = pixConnComp(pixhm2, NULL, 8);
    if (which == 1) {
        boxaWrite("/tmp/lept/livre/textmask.boxa", boxatm);
        boxaWrite("/tmp/lept/livre/htmask.boxa", boxahm);
    }

    pix1 = pixaDisplayTiledAndScaled(pixa, 8, 250, 4, 0, 25, 2);
    pixDisplay(pix1, 0, 375 * (which - 1));
    snprintf(buf, sizeof(buf), "/tmp/lept/livre/segout.%d.png", which);
    pixWrite(buf, pix1, IFF_PNG);
    pixDestroy(&pix1);
    pixaDestroy(&pixa);

        /* clean up to test with valgrind */
    pixDestroy(&pixr);
    pixDestroy(&pixhs);
    pixDestroy(&pixm);
    pixDestroy(&pixhm1);
    pixDestroy(&pixhm2);
    pixDestroy(&pixht);
    pixDestroy(&pixi);
    pixDestroy(&pixnht);
    pixDestroy(&pixvws);
    pixDestroy(&pixm1);
    pixDestroy(&pixm2);
    pixDestroy(&pixm3);
    pixDestroy(&pixb1);
    pixDestroy(&pixb2);
    pixDestroy(&pixnon);
    boxaDestroy(&boxatm);
    boxaDestroy(&boxahm);
    return 0;
}

