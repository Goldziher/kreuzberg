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
 *  binmorph1_reg.c
 *
 *     This is a thorough regression test of different methods for
 *     doing binary morphology.  It should always be run if changes
 *     are made to the low-level morphology code.
 *
 *  Some things to note:
 *
 *  (1) We add a white border to guarantee safe closing; i.e., that
 *      closing is extensive for ASYMMETRIC_MORPH_BC.  The separable
 *      sequence for closing is not safe, so if we didn't add the border
 *      ab initio, we would get different results for the atomic sequence
 *      closing (which is safe) and the separable one.
 *
 *  (2) There are no differences in any of the operations:
 *           rasterop general
 *           rasterop brick
 *           morph sequence rasterop brick
 *           dwa brick
 *           morph sequence dwa brick
 *           morph sequence dwa composite brick
 *      when using ASYMMETRIC_MORPH_BC.
 *      However, when using SYMMETRIC_MORPH_BC, there are differences
 *      in two of the safe closing operations.  These differences
 *      are in pix numbers 4 and 5.  These differences are
 *      all due to the fact that for SYMMETRIC_MORPH_BC, we don't need
 *      to add any borders to get the correct answer.  When we do
 *      add a border of 0 pixels, we naturally get a different result.
 *
 *  (3) The 2-way Sel decomposition functions, implemented with the
 *      separable brick interface, are tested separately against
 *      the rasterop brick.  See binmorph2_reg.c.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

    /* set these ad lib. */
#define    WIDTH            21    /* brick sel width */
#define    HEIGHT           15    /* brick sel height */

void TestAll(L_REGPARAMS *rp, PIX *pix, l_int32 symmetric);

int main(int    argc,
         char **argv)
{
PIX  *pixs;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    pixs = pixRead("feyn-fract.tif");

    TestAll(rp, pixs, FALSE);
    TestAll(rp, pixs, TRUE);
    pixDestroy(&pixs);
    return regTestCleanup(rp);
}


void
TestAll(L_REGPARAMS  *rp,
        PIX          *pixs,
        l_int32       symmetric)
{
l_int32  ok, same;
char     sequence[512];
PIX     *pixref;
PIX     *pix1, *pix2, *pix3, *pix4, *pix5, *pix6;
PIX     *pix7, *pix8, *pix9, *pix10, *pix11;
PIX     *pix12, *pix13, *pix14;
SEL     *sel;

    if (symmetric) {
            /* This works properly if there is an added border */
        resetMorphBoundaryCondition(SYMMETRIC_MORPH_BC);
#if 1
        pix1 = pixAddBorder(pixs, 32, 0);
        pixTransferAllData(pixs, &pix1, 0, 0);
#endif
        lept_stderr("Testing with symmetric boundary conditions\n");
    } else {
        resetMorphBoundaryCondition(ASYMMETRIC_MORPH_BC);
        lept_stderr("Testing with asymmetric boundary conditions\n");
    }

        /* This is our test sel */
    sel = selCreateBrick(HEIGHT, WIDTH, HEIGHT / 2, WIDTH / 2, SEL_HIT);

        /* Dilation */
    lept_stderr("  Testing dilation\n");
    ok = TRUE;
    pixref = pixDilate(NULL, pixs, sel);   /* new one */
    pix1 = pixCreateTemplate(pixs);
    pixDilate(pix1, pixs, sel);           /* existing one */
    pixEqual(pixref, pix1, &same);
    if (!same) {
        lept_stderr("pixref != pix1 !\n"); ok = FALSE;
    }
    pix2 = pixCopy(NULL, pixs);
    pixDilate(pix2, pix2, sel);          /* in-place */
    pixEqual(pixref, pix2, &same);
    if (!same) {
        lept_stderr("pixref != pix2 !\n"); ok = FALSE;
    }
    snprintf(sequence, sizeof(sequence), "d%d.%d", WIDTH, HEIGHT);
    pix3 = pixMorphSequence(pixs, sequence, 0);    /* sequence, atomic */
    pixEqual(pixref, pix3, &same);
    if (!same) {
        lept_stderr("pixref != pix3 !\n"); ok = FALSE;
    }
    snprintf(sequence, sizeof(sequence), "d%d.1 + d1.%d", WIDTH, HEIGHT);
    pix4 = pixMorphSequence(pixs, sequence, 0);    /* sequence, separable */
    pixEqual(pixref, pix4, &same);
    if (!same) {
        lept_stderr("pixref != pix4 !\n"); ok = FALSE;
    }
    pix5 = pixDilateBrick(NULL, pixs, WIDTH, HEIGHT);  /* new one */
    pixEqual(pixref, pix5, &same);
    if (!same) {
        lept_stderr("pixref != pix5 !\n"); ok = FALSE;
    }
    pix6 = pixCreateTemplate(pixs);
    pixDilateBrick(pix6, pixs, WIDTH, HEIGHT);  /* existing one */
    pixEqual(pixref, pix6, &same);
    if (!same) {
        lept_stderr("pixref != pix6 !\n"); ok = FALSE;
    }
    pix7 = pixCopy(NULL, pixs);
    pixDilateBrick(pix7, pix7, WIDTH, HEIGHT);  /* in-place */
    pixEqual(pixref, pix7, &same);
    if (!same) {
        lept_stderr("pixref != pix7 !\n"); ok = FALSE;
    }
    pix8 = pixDilateBrickDwa(NULL, pixs, WIDTH, HEIGHT);  /* new one */
    pixEqual(pixref, pix8, &same);
    if (!same) {
        lept_stderr("pixref != pix8 !\n"); ok = FALSE;
    }
    pix9 = pixCreateTemplate(pixs);
    pixDilateBrickDwa(pix9, pixs, WIDTH, HEIGHT);  /* existing one */
    pixEqual(pixref, pix9, &same);
    if (!same) {
        lept_stderr("pixref != pix9 !\n"); ok = FALSE;
    }
    pix10 = pixCopy(NULL, pixs);
    pixDilateBrickDwa(pix10, pix10, WIDTH, HEIGHT);  /* in-place */
    pixEqual(pixref, pix10, &same);
    if (!same) {
        lept_stderr("pixref != pix10 !\n"); ok = FALSE;
    }
    pix11 = pixCreateTemplate(pixs);
    pixDilateCompBrickDwa(pix11, pixs, WIDTH, HEIGHT);  /* existing one */
    pixEqual(pixref, pix11, &same);
    if (!same) {
        lept_stderr("pixref != pix11 !\n"); ok = FALSE;
    }
    snprintf(sequence, sizeof(sequence), "d%d.%d", WIDTH, HEIGHT);
    pix12 = pixMorphCompSequence(pixs, sequence, 0);    /* comp sequence */
    pixEqual(pixref, pix12, &same);
    if (!same) {
        lept_stderr("pixref != pix12!\n"); ok = FALSE;
    }
    pix13 = pixMorphSequenceDwa(pixs, sequence, 0);    /* dwa sequence */
    pixEqual(pixref, pix13, &same);
    if (!same) {
        lept_stderr("pixref != pix13!\n"); ok = FALSE;
    }
    pixDestroy(&pixref);
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

        /* Erosion */
    lept_stderr("  Testing erosion\n");
    pixref = pixErode(NULL, pixs, sel);   /* new one */
    pix1 = pixCreateTemplate(pixs);
    pixErode(pix1, pixs, sel);           /* existing one */
    pixEqual(pixref, pix1, &same);
    if (!same) {
        lept_stderr("pixref != pix1 !\n"); ok = FALSE;
    }
    pix2 = pixCopy(NULL, pixs);
    pixErode(pix2, pix2, sel);          /* in-place */
    pixEqual(pixref, pix2, &same);
    if (!same) {
        lept_stderr("pixref != pix2 !\n"); ok = FALSE;
    }
    snprintf(sequence, sizeof(sequence), "e%d.%d", WIDTH, HEIGHT);
    pix3 = pixMorphSequence(pixs, sequence, 0);    /* sequence, atomic */
    pixEqual(pixref, pix3, &same);
    if (!same) {
        lept_stderr("pixref != pix3 !\n"); ok = FALSE;
    }
    snprintf(sequence, sizeof(sequence), "e%d.1 + e1.%d", WIDTH, HEIGHT);
    pix4 = pixMorphSequence(pixs, sequence, 0);    /* sequence, separable */
    pixEqual(pixref, pix4, &same);
    if (!same) {
        lept_stderr("pixref != pix4 !\n"); ok = FALSE;
    }
    pix5 = pixErodeBrick(NULL, pixs, WIDTH, HEIGHT);  /* new one */
    pixEqual(pixref, pix5, &same);
    if (!same) {
        lept_stderr("pixref != pix5 !\n"); ok = FALSE;
    }
    pix6 = pixCreateTemplate(pixs);
    pixErodeBrick(pix6, pixs, WIDTH, HEIGHT);  /* existing one */
    pixEqual(pixref, pix6, &same);
    if (!same) {
        lept_stderr("pixref != pix6 !\n"); ok = FALSE;
    }
    pix7 = pixCopy(NULL, pixs);
    pixErodeBrick(pix7, pix7, WIDTH, HEIGHT);  /* in-place */
    pixEqual(pixref, pix7, &same);
    if (!same) {
        lept_stderr("pixref != pix7 !\n"); ok = FALSE;
    }
    pix8 = pixErodeBrickDwa(NULL, pixs, WIDTH, HEIGHT);  /* new one */
    pixEqual(pixref, pix8, &same);
    if (!same) {
        lept_stderr("pixref != pix8 !\n"); ok = FALSE;
    }
    pix9 = pixCreateTemplate(pixs);
    pixErodeBrickDwa(pix9, pixs, WIDTH, HEIGHT);  /* existing one */
    pixEqual(pixref, pix9, &same);
    if (!same) {
        lept_stderr("pixref != pix9 !\n"); ok = FALSE;
    }
    pix10 = pixCopy(NULL, pixs);
    pixErodeBrickDwa(pix10, pix10, WIDTH, HEIGHT);  /* in-place */
    pixEqual(pixref, pix10, &same);
    if (!same) {
        lept_stderr("pixref != pix10 !\n"); ok = FALSE;
    }
    pix11 = pixCreateTemplate(pixs);
    pixErodeCompBrickDwa(pix11, pixs, WIDTH, HEIGHT);  /* existing one */
    pixEqual(pixref, pix11, &same);
    if (!same) {
        lept_stderr("pixref != pix11 !\n"); ok = FALSE;
    }

    snprintf(sequence, sizeof(sequence), "e%d.%d", WIDTH, HEIGHT);
    pix12 = pixMorphCompSequence(pixs, sequence, 0);    /* comp sequence */
    pixEqual(pixref, pix12, &same);
    if (!same) {
        lept_stderr("pixref != pix12!\n"); ok = FALSE;
    }
    pix13 = pixMorphSequenceDwa(pixs, sequence, 0);    /* dwa sequence */
    pixEqual(pixref, pix13, &same);
    if (!same) {
        lept_stderr("pixref != pix13!\n"); ok = FALSE;
    }
    pixDestroy(&pixref);
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

        /* Opening */
    lept_stderr("  Testing opening\n");
    pixref = pixOpen(NULL, pixs, sel);   /* new one */
    pix1 = pixCreateTemplate(pixs);
    pixOpen(pix1, pixs, sel);           /* existing one */
    pixEqual(pixref, pix1, &same);
    if (!same) {
        lept_stderr("pixref != pix1 !\n"); ok = FALSE;
    }
    pix2 = pixCopy(NULL, pixs);
    pixOpen(pix2, pix2, sel);          /* in-place */
    pixEqual(pixref, pix2, &same);
    if (!same) {
        lept_stderr("pixref != pix2 !\n"); ok = FALSE;
    }
    snprintf(sequence, sizeof(sequence), "o%d.%d", WIDTH, HEIGHT);
    pix3 = pixMorphSequence(pixs, sequence, 0);    /* sequence, atomic */
    pixEqual(pixref, pix3, &same);
    if (!same) {
        lept_stderr("pixref != pix3 !\n"); ok = FALSE;
    }
    snprintf(sequence, sizeof(sequence), "e%d.%d + d%d.%d",
             WIDTH, HEIGHT, WIDTH, HEIGHT);
    pix4 = pixMorphSequence(pixs, sequence, 0);    /* sequence, separable */
    pixEqual(pixref, pix4, &same);
    if (!same) {
        lept_stderr("pixref != pix4 !\n"); ok = FALSE;
    }
    snprintf(sequence, sizeof(sequence), "e%d.1 + e1.%d + d%d.1 + d1.%d",
             WIDTH, HEIGHT, WIDTH, HEIGHT);
    pix5 = pixMorphSequence(pixs, sequence, 0);    /* sequence, separable^2 */
    pixEqual(pixref, pix5, &same);
    if (!same) {
        lept_stderr("pixref != pix5 !\n"); ok = FALSE;
    }
    pix6 = pixOpenBrick(NULL, pixs, WIDTH, HEIGHT);  /* new one */
    pixEqual(pixref, pix6, &same);
    if (!same) {
        lept_stderr("pixref != pix6 !\n"); ok = FALSE;
    }
    pix7 = pixCreateTemplate(pixs);
    pixOpenBrick(pix7, pixs, WIDTH, HEIGHT);  /* existing one */
    pixEqual(pixref, pix7, &same);
    if (!same) {
        lept_stderr("pixref != pix7 !\n"); ok = FALSE;
    }
    pix8 = pixCopy(NULL, pixs);  /* in-place */
    pixOpenBrick(pix8, pix8, WIDTH, HEIGHT);  /* existing one */
    pixEqual(pixref, pix8, &same);
    if (!same) {
        lept_stderr("pixref != pix8 !\n"); ok = FALSE;
    }
    pix9 = pixOpenBrickDwa(NULL, pixs, WIDTH, HEIGHT);  /* new one */
    pixEqual(pixref, pix9, &same);
    if (!same) {
        lept_stderr("pixref != pix9 !\n"); ok = FALSE;
    }
    pix10 = pixCreateTemplate(pixs);
    pixOpenBrickDwa(pix10, pixs, WIDTH, HEIGHT);  /* existing one */
    pixEqual(pixref, pix10, &same);
    if (!same) {
        lept_stderr("pixref != pix10 !\n"); ok = FALSE;
    }
    pix11 = pixCopy(NULL, pixs);
    pixOpenBrickDwa(pix11, pix11, WIDTH, HEIGHT);  /* in-place */
    pixEqual(pixref, pix11, &same);
    if (!same) {
        lept_stderr("pixref != pix11 !\n"); ok = FALSE;
    }
    snprintf(sequence, sizeof(sequence), "o%d.%d", WIDTH, HEIGHT);
    pix12 = pixMorphCompSequence(pixs, sequence, 0);    /* comp sequence */
    pixEqual(pixref, pix12, &same);
    if (!same) {
        lept_stderr("pixref != pix12!\n"); ok = FALSE;
    }

    pix13 = pixMorphSequenceDwa(pixs, sequence, 0);    /* dwa sequence */
    pixEqual(pixref, pix13, &same);
    if (!same) {
        lept_stderr("pixref != pix13!\n"); ok = FALSE;
    }
    pix14 = pixCreateTemplate(pixs);
    pixOpenCompBrickDwa(pix14, pixs, WIDTH, HEIGHT);  /* existing one */
    pixEqual(pixref, pix14, &same);
    if (!same) {
        lept_stderr("pixref != pix14 !\n"); ok = FALSE;
    }

    pixDestroy(&pixref);
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

        /* Closing */
    lept_stderr("  Testing closing\n");
    pixref = pixClose(NULL, pixs, sel);   /* new one */
    pix1 = pixCreateTemplate(pixs);
    pixClose(pix1, pixs, sel);           /* existing one */
    pixEqual(pixref, pix1, &same);
    if (!same) {
        lept_stderr("pixref != pix1 !\n"); ok = FALSE;
    }
    pix2 = pixCopy(NULL, pixs);
    pixClose(pix2, pix2, sel);          /* in-place */
    pixEqual(pixref, pix2, &same);
    if (!same) {
        lept_stderr("pixref != pix2 !\n"); ok = FALSE;
    }
    snprintf(sequence, sizeof(sequence), "d%d.%d + e%d.%d",
             WIDTH, HEIGHT, WIDTH, HEIGHT);
    pix3 = pixMorphSequence(pixs, sequence, 0);    /* sequence, separable */
    pixEqual(pixref, pix3, &same);
    if (!same) {
        lept_stderr("pixref != pix3 !\n"); ok = FALSE;
    }
    snprintf(sequence, sizeof(sequence), "d%d.1 + d1.%d + e%d.1 + e1.%d",
             WIDTH, HEIGHT, WIDTH, HEIGHT);
    pix4 = pixMorphSequence(pixs, sequence, 0);    /* sequence, separable^2 */
    pixEqual(pixref, pix4, &same);
    if (!same) {
        lept_stderr("pixref != pix4 !\n"); ok = FALSE;
    }
    pix5 = pixCloseBrick(NULL, pixs, WIDTH, HEIGHT);  /* new one */
    pixEqual(pixref, pix5, &same);
    if (!same) {
        lept_stderr("pixref != pix5 !\n"); ok = FALSE;
    }
    pix6 = pixCreateTemplate(pixs);
    pixCloseBrick(pix6, pixs, WIDTH, HEIGHT);  /* existing one */
    pixEqual(pixref, pix6, &same);
    if (!same) {
        lept_stderr("pixref != pix6 !\n"); ok = FALSE;
    }
    pix7 = pixCopy(NULL, pixs);  /* in-place */
    pixCloseBrick(pix7, pix7, WIDTH, HEIGHT);  /* existing one */
    pixEqual(pixref, pix7, &same);
    if (!same) {
        lept_stderr("pixref != pix7 !\n"); ok = FALSE;
    }
    pixDestroy(&pixref);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);
    pixDestroy(&pix5);
    pixDestroy(&pix6);
    pixDestroy(&pix7);

        /* Safe closing (using pix, not pixs) */
    lept_stderr("  Testing safe closing\n");
    pixref = pixCloseSafe(NULL, pixs, sel);   /* new one */
    pix1 = pixCreateTemplate(pixs);
    pixCloseSafe(pix1, pixs, sel);           /* existing one */
    pixEqual(pixref, pix1, &same);
    if (!same) {
        lept_stderr("pixref != pix1 !\n"); ok = FALSE;
    }
    pix2 = pixCopy(NULL, pixs);
    pixCloseSafe(pix2, pix2, sel);          /* in-place */
    pixEqual(pixref, pix2, &same);
    if (!same) {
        lept_stderr("pixref != pix2 !\n"); ok = FALSE;
    }
    snprintf(sequence, sizeof(sequence), "c%d.%d", WIDTH, HEIGHT);
    pix3 = pixMorphSequence(pixs, sequence, 0);    /* sequence, atomic */
    pixEqual(pixref, pix3, &same);
    if (!same) {
        lept_stderr("pixref != pix3 !\n"); ok = FALSE;
    }
    snprintf(sequence, sizeof(sequence), "b32 + d%d.%d + e%d.%d",
             WIDTH, HEIGHT, WIDTH, HEIGHT);
    pix4 = pixMorphSequence(pixs, sequence, 0);    /* sequence, separable */
    pixEqual(pixref, pix4, &same);
    if (!same) {
        lept_stderr("pixref != pix4 !\n"); ok = FALSE;
    }
    snprintf(sequence, sizeof(sequence), "b32 + d%d.1 + d1.%d + e%d.1 + e1.%d",
             WIDTH, HEIGHT, WIDTH, HEIGHT);
    pix5 = pixMorphSequence(pixs, sequence, 0);    /* sequence, separable^2 */
    pixEqual(pixref, pix5, &same);
    if (!same) {
        lept_stderr("pixref != pix5 !\n"); ok = FALSE;
    }
    pix6 = pixCloseSafeBrick(NULL, pixs, WIDTH, HEIGHT);  /* new one */
    pixEqual(pixref, pix6, &same);
    if (!same) {
        lept_stderr("pixref != pix6 !\n"); ok = FALSE;
    }
    pix7 = pixCreateTemplate(pixs);
    pixCloseSafeBrick(pix7, pixs, WIDTH, HEIGHT);  /* existing one */
    pixEqual(pixref, pix7, &same);
    if (!same) {
        lept_stderr("pixref != pix7 !\n"); ok = FALSE;
    }
    pix8 = pixCopy(NULL, pixs);  /* in-place */
    pixCloseSafeBrick(pix8, pix8, WIDTH, HEIGHT);  /* existing one */
    pixEqual(pixref, pix8, &same);
    if (!same) {
        lept_stderr("pixref != pix8 !\n"); ok = FALSE;
    }
    pix9 = pixCloseBrickDwa(NULL, pixs, WIDTH, HEIGHT);  /* new one */
    pixEqual(pixref, pix9, &same);
    if (!same) {
        lept_stderr("pixref != pix9 !\n"); ok = FALSE;
    }
    pix10 = pixCreateTemplate(pixs);
    pixCloseBrickDwa(pix10, pixs, WIDTH, HEIGHT);  /* existing one */
    pixEqual(pixref, pix10, &same);
    if (!same) {
        lept_stderr("pixref != pix10 !\n"); ok = FALSE;
    }
    pix11 = pixCopy(NULL, pixs);
    pixCloseBrickDwa(pix11, pix11, WIDTH, HEIGHT);  /* in-place */
    pixEqual(pixref, pix11, &same);
    if (!same) {
        lept_stderr("pixref != pix11 !\n"); ok = FALSE;
    }
    snprintf(sequence, sizeof(sequence), "c%d.%d", WIDTH, HEIGHT);
    pix12 = pixMorphCompSequence(pixs, sequence, 0);    /* comp sequence */
    pixEqual(pixref, pix12, &same);
    if (!same) {
        lept_stderr("pixref != pix12!\n"); ok = FALSE;
    }
    pix13 = pixMorphSequenceDwa(pixs, sequence, 0);    /* dwa sequence */
    pixEqual(pixref, pix13, &same);
    if (!same) {
        lept_stderr("pixref != pix13!\n"); ok = FALSE;
    }
    pix14 = pixCreateTemplate(pixs);
    pixCloseCompBrickDwa(pix14, pixs, WIDTH, HEIGHT);  /* existing one */
    pixEqual(pixref, pix14, &same);
    if (!same) {
        lept_stderr("pixref != pix14 !\n"); ok = FALSE;
    }

    pixDestroy(&pixref);
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

    regTestCompareValues(rp, TRUE, ok, 0);
    if (ok)
        lept_stderr("  All morph tests OK!\n");
    else
        lept_stderr("  Some morph tests failed!\n");
    selDestroy(&sel);
}

