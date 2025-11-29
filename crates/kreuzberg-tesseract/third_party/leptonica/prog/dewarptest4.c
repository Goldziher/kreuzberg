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
 *   dewarptest4.c
 *
 *   Tests serialization functions for dewarpa and dewarp structs.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"


l_int32 main(int    argc,
             char **argv)
{
L_DEWARP   *dew1, *dew2, *dew3;
L_DEWARPA  *dewa1, *dewa2, *dewa3;
PIX        *pixs, *pixn, *pixg, *pixb, *pixd;
PIX        *pixs2, *pixn2, *pixg2, *pixb2, *pixd2;
PIX        *pixd3, *pixc1, *pixc2;

    setLeptDebugOK(1);
    lept_mkdir("lept");

/*    pixs = pixRead("1555.007.jpg"); */
    pixs = pixRead("cat.035.jpg");
    dewa1 = dewarpaCreate(40, 30, 1, 15, 10);
    dewarpaUseBothArrays(dewa1, 1);

        /* Normalize for varying background and binarize */
    pixn = pixBackgroundNormSimple(pixs, NULL, NULL);
    pixg = pixConvertRGBToGray(pixn, 0.5, 0.3, 0.2);
    pixb = pixThresholdToBinary(pixg, 130);

        /* Run the basic functions */
    dew1 = dewarpCreate(pixb, 35);
    dewarpaInsertDewarp(dewa1, dew1);
    dewarpBuildPageModel(dew1, "/tmp/lept/dewarp_junk35.pdf");
    dewarpPopulateFullRes(dew1, pixg, 0, 0);
    dewarpaApplyDisparity(dewa1, 35, pixg, 200, 0, 0, &pixd,
                          "/tmp/lept/dewarp_debug_35.pdf");

        /* Normalize another image. */
/*    pixs2 = pixRead("1555.003.jpg"); */
    pixs2 = pixRead("cat.007.jpg");
    pixn2 = pixBackgroundNormSimple(pixs2, NULL, NULL);
    pixg2 = pixConvertRGBToGray(pixn2, 0.5, 0.3, 0.2);
    pixb2 = pixThresholdToBinary(pixg2, 130);

        /* Run the basic functions */
    dew2 = dewarpCreate(pixb2, 7);
    dewarpaInsertDewarp(dewa1, dew2);
    dewarpBuildPageModel(dew2, "/tmp/lept/dewarp_junk7.pdf");
    dewarpaApplyDisparity(dewa1, 7, pixg, 200, 0, 0, &pixd2,
                          "/tmp/lept/dewarp_debug_7.pdf");

        /* Serialize and deserialize dewarpa */
    dewarpaWrite("/tmp/lept/dewarpa1.dewa", dewa1);
    dewa2 = dewarpaRead("/tmp/lept/dewarpa1.dewa");
    dewarpaWrite("/tmp/lept/dewarpa2.dewa", dewa2);
    dewa3 = dewarpaRead("/tmp/lept/dewarpa2.dewa");
    dewarpDebug(dewa3->dewarp[7], "dew1", 7);
    dewarpaWrite("/tmp/lept/dewarpa3.dewa", dewa3);

        /* Repopulate and show the vertical disparity arrays */
    dewarpPopulateFullRes(dew1, NULL, 0, 0);
    pixc1 = fpixRenderContours(dew1->fullvdispar, 2.0, 0.2);
    pixDisplay(pixc1, 1400, 900);
    dew3 = dewarpaGetDewarp(dewa2, 35);
    dewarpPopulateFullRes(dew3, pixs, 0, 0);
    pixc2 = fpixRenderContours(dew3->fullvdispar, 2.0, 0.2);
    pixDisplay(pixc2, 1400, 900);
    dewarpaApplyDisparity(dewa2, 35, pixb, 200, 0, 0, &pixd3,
                          "/tmp/lept/dewarp_debug_35b.pdf");
    pixDisplay(pixd, 0, 1000);
    pixDisplay(pixd2, 600, 1000);
    pixDisplay(pixd3, 1200, 1000);
    pixDestroy(&pixd3);

    dewarpaDestroy(&dewa1);
    dewarpaDestroy(&dewa2);
    dewarpaDestroy(&dewa3);
    pixDestroy(&pixs);
    pixDestroy(&pixn);
    pixDestroy(&pixg);
    pixDestroy(&pixb);
    pixDestroy(&pixd);
    pixDestroy(&pixs2);
    pixDestroy(&pixn2);
    pixDestroy(&pixg2);
    pixDestroy(&pixb2);
    pixDestroy(&pixd2);
    pixDestroy(&pixc1);
    pixDestroy(&pixc2);
    return 0;
}
