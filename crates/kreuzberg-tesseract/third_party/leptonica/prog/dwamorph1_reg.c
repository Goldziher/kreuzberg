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
 * dwamorph1_reg.c
 *
 *     Fairly thorough regression test for autogen'd dwa.
 *
 *     The dwa code always implements safe closing.  With asymmetric
 *     b.c., the rasterop function must be pixCloseSafe().
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

    /* defined in morph.c */
LEPT_DLL extern l_int32 MORPH_BC;

    /* Complete set of linear brick dwa operations */
PIX *pixMorphDwa_3(PIX *pixd, PIX *pixs, l_int32 operation, char *selname);
PIX *pixFMorphopGen_3(PIX *pixd, PIX *pixs, l_int32 operation, char *selname);


int main(int    argc,
         char **argv)
{
l_int32       i, nsels, same, xorcount;
char         *selname;
PIX          *pixs, *pixt1, *pixt2, *pixt3;
SEL          *sel;
SELA         *sela;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    if ((pixs = pixRead("feyn-fract.tif")) == NULL) {
        rp->success = FALSE;
        return regTestCleanup(rp);
    }
    sela = selaAddDwaLinear(NULL);
    nsels = selaGetCount(sela);

    for (i = 0; i < nsels; i++)
    {
	sel = selaGetSel(sela, i);
	selname = selGetName(sel);

	    /*  ---------  dilation  ----------*/

	pixt1 = pixDilate(NULL, pixs, sel);
        pixt2 = pixMorphDwa_3(NULL, pixs, L_MORPH_DILATE, selname);
        pixEqual(pixt1, pixt2, &same);

	if (same == 1) {
            if (rp->display)
                lept_stderr("dilations are identical for sel %d (%s)\n",
                            i, selname);
	} else {
            rp->success = FALSE;
	    fprintf(rp->fp, "dilations differ for sel %d (%s)\n", i, selname);
	    pixt3 = pixXor(NULL, pixt1, pixt2);
	    pixCountPixels(pixt3, &xorcount, NULL);
	    fprintf(rp->fp, "Number of pixels in XOR: %d\n", xorcount);
            pixDestroy(&pixt3);
	}
	pixDestroy(&pixt1);
	pixDestroy(&pixt2);

	    /*  ---------  erosion with asymmetric b.c  ----------*/

        resetMorphBoundaryCondition(ASYMMETRIC_MORPH_BC);
        if (rp->display) lept_stderr("MORPH_BC = %d ... ", MORPH_BC);

	pixt1 = pixErode(NULL, pixs, sel);
        pixt2 = pixMorphDwa_3(NULL, pixs, L_MORPH_ERODE, selname);
        pixEqual(pixt1, pixt2, &same);

	if (same == 1) {
            if (rp->display)
                lept_stderr("erosions are identical for sel %d (%s)\n",
                            i, selname);
	} else {
            rp->success = FALSE;
	    fprintf(rp->fp, "erosions differ for sel %d (%s)\n", i, selname);
	    pixt3 = pixXor(NULL, pixt1, pixt2);
	    pixCountPixels(pixt3, &xorcount, NULL);
	    fprintf(rp->fp, "Number of pixels in XOR: %d\n", xorcount);
            pixDestroy(&pixt3);
	}
	pixDestroy(&pixt1);
	pixDestroy(&pixt2);

	    /*  ---------  erosion with symmetric b.c  ----------*/

        resetMorphBoundaryCondition(SYMMETRIC_MORPH_BC);
        if (rp->display) lept_stderr("MORPH_BC = %d ... ", MORPH_BC);

	pixt1 = pixErode(NULL, pixs, sel);
        pixt2 = pixMorphDwa_3(NULL, pixs, L_MORPH_ERODE, selname);
        pixEqual(pixt1, pixt2, &same);

	if (same == 1) {
            if (rp->display)
                lept_stderr("erosions are identical for sel %d (%s)\n",
                            i, selname);
	} else {
            rp->success = FALSE;
	    fprintf(rp->fp, "erosions differ for sel %d (%s)\n", i, selname);
	    pixt3 = pixXor(NULL, pixt1, pixt2);
	    pixCountPixels(pixt3, &xorcount, NULL);
	    fprintf(rp->fp, "Number of pixels in XOR: %d\n", xorcount);
            pixDestroy(&pixt3);
	}
	pixDestroy(&pixt1);
	pixDestroy(&pixt2);

	    /*  ---------  opening with asymmetric b.c  ----------*/

        resetMorphBoundaryCondition(ASYMMETRIC_MORPH_BC);
        if (rp->display) lept_stderr("MORPH_BC = %d ... ", MORPH_BC);

	pixt1 = pixOpen(NULL, pixs, sel);
        pixt2 = pixMorphDwa_3(NULL, pixs, L_MORPH_OPEN, selname);
        pixEqual(pixt1, pixt2, &same);

	if (same == 1) {
            if (rp->display)
                lept_stderr("openings are identical for sel %d (%s)\n",
                            i, selname);
	} else {
            rp->success = FALSE;
	    fprintf(rp->fp, "openings differ for sel %d (%s)\n", i, selname);
	    pixt3 = pixXor(NULL, pixt1, pixt2);
	    pixCountPixels(pixt3, &xorcount, NULL);
	    fprintf(rp->fp, "Number of pixels in XOR: %d\n", xorcount);
            pixDestroy(&pixt3);
	}
	pixDestroy(&pixt1);
	pixDestroy(&pixt2);

	    /*  ---------  opening with symmetric b.c  ----------*/

        resetMorphBoundaryCondition(SYMMETRIC_MORPH_BC);
        if (rp->display) lept_stderr("MORPH_BC = %d ... ", MORPH_BC);

	pixt1 = pixOpen(NULL, pixs, sel);
        pixt2 = pixMorphDwa_3(NULL, pixs, L_MORPH_OPEN, selname);
        pixEqual(pixt1, pixt2, &same);

	if (same == 1) {
            if (rp->display)
                lept_stderr("openings are identical for sel %d (%s)\n",
                            i, selname);
	} else {
            rp->success = FALSE;
	    fprintf(rp->fp, "openings differ for sel %d (%s)\n", i, selname);
	    pixt3 = pixXor(NULL, pixt1, pixt2);
	    pixCountPixels(pixt3, &xorcount, NULL);
	    fprintf(rp->fp, "Number of pixels in XOR: %d\n", xorcount);
            pixDestroy(&pixt3);
	}
	pixDestroy(&pixt1);
	pixDestroy(&pixt2);

	    /*  ---------  safe closing with asymmetric b.c  ----------*/

        resetMorphBoundaryCondition(ASYMMETRIC_MORPH_BC);
        if (rp->display) lept_stderr("MORPH_BC = %d ... ", MORPH_BC);

	pixt1 = pixCloseSafe(NULL, pixs, sel);  /* must use safe version */
        pixt2 = pixMorphDwa_3(NULL, pixs, L_MORPH_CLOSE, selname);
        pixEqual(pixt1, pixt2, &same);

	if (same == 1) {
            if (rp->display)
                lept_stderr("closings are identical for sel %d (%s)\n",
                            i, selname);
	} else {
            rp->success = FALSE;
	    fprintf(rp->fp, "closings differ for sel %d (%s)\n", i, selname);
	    pixt3 = pixXor(NULL, pixt1, pixt2);
	    pixCountPixels(pixt3, &xorcount, NULL);
	    fprintf(rp->fp, "Number of pixels in XOR: %d\n", xorcount);
            pixDestroy(&pixt3);
	}
	pixDestroy(&pixt1);
	pixDestroy(&pixt2);

	    /*  ---------  safe closing with symmetric b.c  ----------*/

        resetMorphBoundaryCondition(SYMMETRIC_MORPH_BC);
        if (rp->display) lept_stderr("MORPH_BC = %d ... ", MORPH_BC);

	pixt1 = pixClose(NULL, pixs, sel);  /* safe version not required */
        pixt2 = pixMorphDwa_3(NULL, pixs, L_MORPH_CLOSE, selname);
        pixEqual(pixt1, pixt2, &same);

	if (same == 1) {
            if (rp->display)
                lept_stderr("closings are identical for sel %d (%s)\n",
                i, selname);
	} else {
            rp->success = FALSE;
	    fprintf(rp->fp, "closings differ for sel %d (%s)\n", i, selname);
	    pixt3 = pixXor(NULL, pixt1, pixt2);
	    pixCountPixels(pixt3, &xorcount, NULL);
	    fprintf(rp->fp, "Number of pixels in XOR: %d\n", xorcount);
            pixDestroy(&pixt3);
	}
	pixDestroy(&pixt1);
	pixDestroy(&pixt2);
    }

    selaDestroy(&sela);
    pixDestroy(&pixs);
    return regTestCleanup(rp);
}
