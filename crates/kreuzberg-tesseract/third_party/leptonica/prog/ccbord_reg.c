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
 * ccbord_reg.c
 *
 *      Regression test for border-following representations of binary images.
 *      This uses the steps in ccbordtest.c to test specified images.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include <string.h>
#include "allheaders.h"

void RunCCBordTest(const char *fname, L_REGPARAMS *rp);

int main(int    argc,
         char **argv)
{
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;
    lept_mkdir("lept/ccbord");

    RunCCBordTest("feyn-fract.tif", rp);
    RunCCBordTest("dreyfus1.png", rp);
    return regTestCleanup(rp);
}

/* ----------------------------------------------- */
void RunCCBordTest(const char   *fname,
                   L_REGPARAMS  *rp)
{
char     *svgstr;
l_int32   count, disp;
CCBORDA  *ccba, *ccba2;
PIX      *pixs, *pixd, *pixd2, *pixd3;
PIX      *pixt, *pixc, *pixc2;

    pixs = pixRead(fname);
    disp = rp->display;

    /*------------------------------------------------------------------*
     *        Get border representation and verify border pixels        *
     *------------------------------------------------------------------*/
    if(disp) lept_stderr("Get border representation...");
    ccba = pixGetAllCCBorders(pixs);

        /* Get global locs directly and display borders */
    if (disp) lept_stderr("Convert from local to global locs...");
    ccbaGenerateGlobalLocs(ccba);
    if (disp) lept_stderr("display border representation...");
    pixd = ccbaDisplayBorder(ccba);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);  /* 0,7 */
    pixDisplayWithTitle(pixd, 0, 0, NULL, rp->display);
    pixDestroy(&pixd);

        /* Get step chain code, then global coords, and display borders */
    if (disp) lept_stderr("get step chain code...");
    ccbaGenerateStepChains(ccba);
    if (disp) lept_stderr("convert from step chain to global locs...");
    ccbaStepChainsToPixCoords(ccba, CCB_GLOBAL_COORDS);
    if (disp) lept_stderr("display border representation\n");
    pixd = ccbaDisplayBorder(ccba);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);  /* 1,8 */
    pixDisplayWithTitle(pixd, 200, 0, NULL, rp->display);

        /* Check if border pixels are in original set */
    lept_stderr("Check if border pixels are in original set\n");
    pixt = pixSubtract(NULL, pixd, pixs);
    pixCountPixels(pixt, &count, NULL);
    if (count == 0)
        lept_stderr(" ==> all border pixels are in original set\n");
    else
        lept_stderr(" ==> %d border pixels are not in original set\n", count);
    pixDestroy(&pixt);

        /* Display image */
    lept_stderr("Reconstruct image\n");
    pixc = ccbaDisplayImage2(ccba);
    regTestWritePixAndCheck(rp, pixc, IFF_PNG);  /* 2,9 */
    pixDisplayWithTitle(pixc, 400, 0, NULL, rp->display);

        /* Check with original to see if correct */
    pixXor(pixc, pixc, pixs);
    pixCountPixels(pixc, &count, NULL);
    if (count == 0)
        lept_stderr(" ==> perfect direct reconstruction\n");
    else {
        l_int32  w, h, i, j;
        l_uint32 val;
        lept_stderr(" ==> %d pixels in error in reconstruction\n", count);
#if 1
        w = pixGetWidth(pixc);
        h = pixGetHeight(pixc);
        for (i = 0; i < h; i++) {
            for (j = 0; j < w; j++) {
                pixGetPixel(pixc, j, i, &val);
                if (val == 1)
                    lept_stderr("bad pixel at (%d, %d)\n", j, i);
            }
        }
        pixWrite("/tmp/lept/ccbord/badpixels1.png", pixc, IFF_PNG);
#endif
    }

    /*----------------------------------------------------------*
     *        Write to file (compressed) and read back          *
     *----------------------------------------------------------*/
    if (disp) lept_stderr("Write serialized step data...");
    ccbaWrite("/tmp/lept/ccbord/stepdata.ccb", ccba);
    if (disp) lept_stderr("read serialized step data...");
    ccba2 = ccbaRead("/tmp/lept/ccbord/stepdata.ccb");

        /* Display the border pixels again */
    if (disp) lept_stderr("convert from step chain to global locs...");
    ccbaStepChainsToPixCoords(ccba2, CCB_GLOBAL_COORDS);
    if (disp) lept_stderr("display border representation\n");
    pixd2 = ccbaDisplayBorder(ccba2);
    regTestWritePixAndCheck(rp, pixd2, IFF_PNG);  /* 3,10 */
    pixDisplayWithTitle(pixd2, 600, 0, NULL, rp->display);

        /* Check if border pixels are same as first time */
    lept_stderr("Check border after write/read\n");
    pixXor(pixd2, pixd2, pixd);
    pixCountPixels(pixd2, &count, NULL);
    if (count == 0)
        lept_stderr(" ==> perfect w/r border recon\n");
    else
        lept_stderr(" ==> %d pixels in error in w/r recon\n", count);
    pixDestroy(&pixd2);

        /* Display image again */
    if (disp) lept_stderr("Convert from step chain to local coords...\n");
    ccbaStepChainsToPixCoords(ccba2, CCB_LOCAL_COORDS);
    lept_stderr("Reconstruct image from file\n");
    pixc2 = ccbaDisplayImage2(ccba2);
    regTestWritePixAndCheck(rp, pixc2, IFF_PNG);  /* 4,11 */
    pixDisplayWithTitle(pixc2, 800, 0, NULL, rp->display);

        /* Check with original to see if correct */
    pixXor(pixc2, pixc2, pixs);
    pixCountPixels(pixc2, &count, NULL);
    if (count == 0)
        lept_stderr(" ==> perfect image recon\n");
    else {
        l_int32  w, h, i, j;
        l_uint32 val;
        lept_stderr(" ==> %d pixels in error in image recon\n", count);
#if 1
        w = pixGetWidth(pixc2);
        h = pixGetHeight(pixc2);
        for (i = 0; i < h; i++) {
            for (j = 0; j < w; j++) {
                pixGetPixel(pixc2, j, i, &val);
                if (val == 1)
                    lept_stderr("bad pixel at (%d, %d)\n", j, i);
            }
        }
        pixWrite("/tmp/lept/ccbord/badpixels2.png", pixc, IFF_PNG);
#endif
    }

    /*----------------------------------------------------------*
     *     Make, display and check single path border for svg   *
     *----------------------------------------------------------*/
        /* Make local single path border for svg */
    if (disp) lept_stderr("Make local single path borders for svg ...");
    ccbaGenerateSinglePath(ccba);

        /* Generate global single path border */
    if (disp) lept_stderr("generate global single path borders ...");
    ccbaGenerateSPGlobalLocs(ccba, CCB_SAVE_TURNING_PTS);

        /* Display border pixels from single path */
    if (disp) lept_stderr("display border from single path\n");
    pixd3 = ccbaDisplaySPBorder(ccba);
    regTestWritePixAndCheck(rp, pixd3, IFF_PNG);  /* 5,12 */
    pixDisplayWithTitle(pixd3, 1000, 0, NULL, rp->display);

        /* Check if border pixels are in original set */
    lept_stderr("Check if border pixels are in original set\n");
    pixt = pixSubtract(NULL, pixd3, pixs);
    pixCountPixels(pixt, &count, NULL);
    if (count == 0)
        lept_stderr(" ==> all border pixels are in original set\n");
    else
        lept_stderr(" ==> %d border pixels are not in original set\n", count);
    lept_stderr("============================================\n");
    pixDestroy(&pixt);
    pixDestroy(&pixd3);

        /*  Output in svg file format */
    svgstr = ccbaWriteSVGString(ccba);
    regTestWriteDataAndCheck(rp, svgstr, strlen(svgstr), "ccb"); /* 6,13 */

    ccbaDestroy(&ccba2);
    ccbaDestroy(&ccba);
    pixDestroy(&pixs);
    pixDestroy(&pixd);
    pixDestroy(&pixc);
    pixDestroy(&pixc2);
    lept_free(svgstr);
}
