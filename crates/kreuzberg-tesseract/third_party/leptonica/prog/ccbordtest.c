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
 * ccbordtest.c
 *
 *      Comprehensive test for border-following representations
 *      of binary images.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
         char **argv)
{
char     *filein;
l_int32   count;
CCBORDA  *ccba, *ccba2;
PIX      *pixs, *pixd, *pixd2, *pixd3;
PIX      *pixt, *pixc, *pixc2;

    if (argc != 2)
        return ERROR_INT(" Syntax:  ccbordtest filein", __func__, 1);
    filein = argv[1];

    setLeptDebugOK(1);
    lept_mkdir("lept/ccbord");

    if ((pixs = pixRead(filein)) == NULL)
        return ERROR_INT("pixs not made", __func__, 1);

    lept_stderr("Get border representation...");
    startTimer();
    ccba = pixGetAllCCBorders(pixs);
    lept_stderr("%6.3f sec\n", stopTimer());

        /* Get global locs directly and display borders */
    lept_stderr("Convert from local to global locs...");
    startTimer();
    ccbaGenerateGlobalLocs(ccba);
    lept_stderr("%6.3f sec\n", stopTimer());
    lept_stderr("Display border representation...");
    startTimer();
    pixd = ccbaDisplayBorder(ccba);
    lept_stderr("%6.3f sec\n", stopTimer());
    pixWrite("/tmp/lept/ccbord/junkborder1.png", pixd, IFF_PNG);
    pixDestroy(&pixd);

        /* Get step chain code, then global coords, and display borders */
    lept_stderr("Get step chain code...");
    startTimer();
    ccbaGenerateStepChains(ccba);
    lept_stderr("%6.3f sec\n", stopTimer());
    lept_stderr("Convert from step chain to global locs...");
    startTimer();
    ccbaStepChainsToPixCoords(ccba, CCB_GLOBAL_COORDS);
    lept_stderr("%6.3f sec\n", stopTimer());
    lept_stderr("Display border representation...");
    startTimer();
    pixd = ccbaDisplayBorder(ccba);
    lept_stderr("%6.3f sec\n", stopTimer());
    pixWrite("/tmp/lept/ccbord/junkborder1.png", pixd, IFF_PNG);

        /* Check if border pixels are in original set */
    lept_stderr("Check if border pixels are in original set ...\n");
    pixt = pixSubtract(NULL, pixd, pixs);
    pixCountPixels(pixt, &count, NULL);
    if (count == 0)
        lept_stderr("   all border pixels are in original set\n");
    else
        lept_stderr("   %d border pixels are not in original set\n", count);
    pixDestroy(&pixt);

        /* Display image */
    lept_stderr("Reconstruct image ...");
    startTimer();
/*    pixc = ccbaDisplayImage1(ccba); */
    pixc = ccbaDisplayImage2(ccba);
    lept_stderr("%6.3f sec\n", stopTimer());
    pixWrite("/tmp/lept/ccbord/junkrecon1.png", pixc, IFF_PNG);

        /* check with original to see if correct */
    lept_stderr("Check with original to see if correct ...\n");
    pixXor(pixc, pixc, pixs);
    pixCountPixels(pixc, &count, NULL);
    if (count == 0)
        lept_stderr("   perfect direct recon\n");
    else {
        l_int32  w, h, i, j;
        l_uint32 val;
        lept_stderr("   %d pixels in error in recon\n", count);
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
        pixWrite("/tmp/lept/ccbord/junkbadpixels.png", pixc, IFF_PNG);
#endif
    }


    /*----------------------------------------------------------*
     *        write to file (compressed) and read back          *
     *----------------------------------------------------------*/
    lept_stderr("Write serialized step data...");
    startTimer();
    ccbaWrite("/tmp/junkstepout", ccba);
    lept_stderr("%6.3f sec\n", stopTimer());
    lept_stderr("Read serialized step data...");
    startTimer();
    ccba2 = ccbaRead("/tmp/junkstepout");
    lept_stderr("%6.3f sec\n", stopTimer());

        /* display the border pixels again */
    lept_stderr("Convert from step chain to global locs...");
    startTimer();
    ccbaStepChainsToPixCoords(ccba2, CCB_GLOBAL_COORDS);
    lept_stderr("%6.3f sec\n", stopTimer());
    lept_stderr("Display border representation...");
    startTimer();
    pixd2 = ccbaDisplayBorder(ccba2);
    lept_stderr("%6.3f sec\n", stopTimer());
    pixWrite("/tmp/lept/ccbord/junkborder2.png", pixd2, IFF_PNG);

        /* check if border pixels are same as first time */
    pixXor(pixd2, pixd2, pixd);
    pixCountPixels(pixd2, &count, NULL);
    if (count == 0)
        lept_stderr("   perfect w/r border recon\n");
    else
        lept_stderr("   %d pixels in error in w/r recon\n", count);
    pixDestroy(&pixd2);

        /* display image again */
    lept_stderr("Convert from step chain to local coords...");
    startTimer();
    ccbaStepChainsToPixCoords(ccba2, CCB_LOCAL_COORDS);
    lept_stderr("%6.3f sec\n", stopTimer());
    lept_stderr("Reconstruct image from file ...");
    startTimer();
/*    pixc2 = ccbaDisplayImage1(ccba2); */
    pixc2 = ccbaDisplayImage2(ccba2);
    lept_stderr("%6.3f sec\n", stopTimer());
    pixWrite("/tmp/lept/ccbord/junkrecon2.png", pixc2, IFF_PNG);

        /* check with original to see if correct */
    lept_stderr("Check with original to see if correct ...\n");
    pixXor(pixc2, pixc2, pixs);
    pixCountPixels(pixc2, &count, NULL);
    if (count == 0)
        lept_stderr("   perfect image recon\n");
    else {
        l_int32  w, h, i, j;
        l_uint32 val;
        lept_stderr("   %d pixels in error in image recon\n", count);
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
        pixWrite("/tmp/lept/ccbord/junkbadpixels2.png", pixc2, IFF_PNG);
#endif
    }

    /*----------------------------------------------------------*
     *     make, display and check single path border for svg   *
     *----------------------------------------------------------*/
        /* make local single path border for svg */
    lept_stderr("Make local single path borders for svg ...");
    startTimer();
    ccbaGenerateSinglePath(ccba);
    lept_stderr("%6.3f sec\n", stopTimer());
        /* generate global single path border */
    lept_stderr("Generate global single path borders ...");
    startTimer();
    ccbaGenerateSPGlobalLocs(ccba, CCB_SAVE_TURNING_PTS);
    lept_stderr("%6.3f sec\n", stopTimer());
        /* display border pixels from single path */
    lept_stderr("Display border from single path...");
    startTimer();
    pixd3 = ccbaDisplaySPBorder(ccba);
    lept_stderr("%6.3f sec\n", stopTimer());
    pixWrite("/tmp/lept/ccbord/junkborder3.png", pixd3, IFF_PNG);
        /* check if border pixels are in original set */
    lept_stderr("Check if border pixels are in original set ...\n");
    pixt = pixSubtract(NULL, pixd3, pixs);
    pixCountPixels(pixt, &count, NULL);
    if (count == 0)
        lept_stderr("   all border pixels are in original set\n");
    else
        lept_stderr("   %d border pixels are not in original set\n", count);
    pixDestroy(&pixt);
    pixDestroy(&pixd3);

        /*  output in svg file format */
    lept_stderr("Write output in svg file format ...\n");
    startTimer();
    ccbaWriteSVG("/tmp/junksvg", ccba);
    lept_stderr("%6.3f sec\n", stopTimer());

    ccbaDestroy(&ccba2);
    ccbaDestroy(&ccba);
    pixDestroy(&pixs);
    pixDestroy(&pixd);
    pixDestroy(&pixc);
    pixDestroy(&pixc2);
    return 0;
}
