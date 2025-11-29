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
 * fmorphauto_reg.c
 *
 *    Basic regression test for erosion & dilation: rasterops & dwa.
 *
 *    Tests erosion and dilation from 58 structuring elements
 *    by comparing the full image rasterop results with the
 *    automatically generated dwa results.
 *
 *    Results must be identical for all operations.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

    /* defined in morph.c */
LEPT_DLL extern l_int32 MORPH_BC;


int main(int    argc,
         char **argv)
{
l_int32  i, nsels, same, xorcount;
char    *filein, *selname;
PIX     *pixs, *pixs1, *pixt1, *pixt2, *pixt3, *pixt4;
SEL     *sel;
SELA    *sela;

    if (argc != 2)
        return ERROR_INT(" Syntax:  fmorphauto_reg filein", __func__, 1);
    filein = argv[1];
    setLeptDebugOK(1);

    if ((pixs = pixRead(filein)) == NULL)
        return ERROR_INT("pix not made", __func__, 1);

    sela = selaAddBasic(NULL);
    nsels = selaGetCount(sela);
    for (i = 0; i < nsels; i++)
    {
        sel = selaGetSel(sela, i);
        selname = selGetName(sel);

            /*  ---------  dilation  ----------*/

        pixt1 = pixDilate(NULL, pixs, sel);

        pixs1 = pixAddBorder(pixs, 32, 0);
        pixt2 = pixFMorphopGen_1(NULL, pixs1, L_MORPH_DILATE, selname);
        pixt3 = pixRemoveBorder(pixt2, 32);

        pixt4 = pixXor(NULL, pixt1, pixt3);
        pixZero(pixt4, &same);

        if (same == 1) {
            lept_stderr("dilations are identical for sel %d (%s)\n",
                        i, selname);
        } else {
            lept_stderr("dilations differ for sel %d (%s)\n", i, selname);
            pixCountPixels(pixt4, &xorcount, NULL);
            lept_stderr("Number of pixels in XOR: %d\n", xorcount);
        }

        pixDestroy(&pixt1);
        pixDestroy(&pixt2);
        pixDestroy(&pixt3);
        pixDestroy(&pixt4);
        pixDestroy(&pixs1);

            /*  ---------  erosion with asymmetric b.c  ----------*/

        resetMorphBoundaryCondition(ASYMMETRIC_MORPH_BC);
        lept_stderr("MORPH_BC = %d ... ", MORPH_BC);
        pixt1 = pixErode(NULL, pixs, sel);

        if (MORPH_BC == ASYMMETRIC_MORPH_BC)
            pixs1 = pixAddBorder(pixs, 32, 0);  /* OFF border pixels */
        else
            pixs1 = pixAddBorder(pixs, 32, 1);  /* ON border pixels */
        pixt2 = pixFMorphopGen_1(NULL, pixs1, L_MORPH_ERODE, selname);
        pixt3 = pixRemoveBorder(pixt2, 32);

        pixt4 = pixXor(NULL, pixt1, pixt3);
        pixZero(pixt4, &same);

        if (same == 1) {
            lept_stderr("erosions are identical for sel %d (%s)\n", i, selname);
        } else {
            lept_stderr("erosions differ for sel %d (%s)\n", i, selname);
            pixCountPixels(pixt4, &xorcount, NULL);
            lept_stderr("Number of pixels in XOR: %d\n", xorcount);
        }

        pixDestroy(&pixt1);
        pixDestroy(&pixt2);
        pixDestroy(&pixt3);
        pixDestroy(&pixt4);
        pixDestroy(&pixs1);

            /*  ---------  erosion with symmetric b.c  ----------*/

        resetMorphBoundaryCondition(SYMMETRIC_MORPH_BC);
        lept_stderr("MORPH_BC = %d ... ", MORPH_BC);
        pixt1 = pixErode(NULL, pixs, sel);

        if (MORPH_BC == ASYMMETRIC_MORPH_BC)
            pixs1 = pixAddBorder(pixs, 32, 0);  /* OFF border pixels */
        else
            pixs1 = pixAddBorder(pixs, 32, 1);  /* ON border pixels */
        pixt2 = pixFMorphopGen_1(NULL, pixs1, L_MORPH_ERODE, selname);
        pixt3 = pixRemoveBorder(pixt2, 32);

        pixt4 = pixXor(NULL, pixt1, pixt3);
        pixZero(pixt4, &same);

        if (same == 1) {
            lept_stderr("erosions are identical for sel %d (%s)\n", i, selname);
        } else {
            lept_stderr("erosions differ for sel %d (%s)\n", i, selname);
            pixCountPixels(pixt4, &xorcount, NULL);
            lept_stderr("Number of pixels in XOR: %d\n", xorcount);
        }

        pixDestroy(&pixt1);
        pixDestroy(&pixt2);
        pixDestroy(&pixt3);
        pixDestroy(&pixt4);
        pixDestroy(&pixs1);
    }

    return 0;
}

