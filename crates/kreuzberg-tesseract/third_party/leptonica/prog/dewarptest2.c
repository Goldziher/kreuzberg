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
 *   dewarptest2.c
 *
 *   This runs the basic functions for a single page.  It can be used
 *   to debug the disparity model-building.
 *
 *     dewarptest2 method [image pageno]
 *
 *     where: method = 1 (use single page dewarp function)
 *                     2 (break down into multiple steps)
 *
 *   Default image is cat.035.jpg.
 *   Others are 1555.007.jpg, shearer.148.tif, lapide.052.100.jpg, etc.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

#define  NORMALIZE     1

l_int32 main(int    argc,
             char **argv)
{
l_int32     d, method, pageno;
L_DEWARP   *dew1;
L_DEWARPA  *dewa;
PIX        *pixs, *pixn, *pixg, *pixb, *pixd;

    if (argc != 2 && argc != 4)
        return ERROR_INT("Syntax: dewarptest2 method [image pageno]",
                         __func__, 1);
    if (argc == 2) {
        pixs = pixRead("cat.035.jpg");
        pageno = 35;
    }
    else {
        pixs = pixRead(argv[2]);
        pageno = atoi(argv[3]);
    }
    if (!pixs)
        return ERROR_INT("image not read", __func__, 1);
    method = atoi(argv[1]);

    setLeptDebugOK(1);
    lept_mkdir("lept/dewarp");

    if (method == 1) {  /* Use single page dewarp function */
        dewarpSinglePage(pixs, 0, 1, 1, 0, &pixd, NULL, 1);
    } else {  /* Break down into multiple steps; require min of only 8 lines */
        dewa = dewarpaCreate(40, 30, 1, 8, 50);
        dewarpaUseBothArrays(dewa, 1);
        dewarpaSetCheckColumns(dewa, 0);
        d = pixGetDepth(pixs);

#if NORMALIZE
            /* Normalize for varying background and binarize */
        if (d > 1) {
            pixn = pixBackgroundNormSimple(pixs, NULL, NULL);
            pixg = pixConvertRGBToGray(pixn, 0.5, 0.3, 0.2);
            pixb = pixThresholdToBinary(pixg, 130);
            pixDestroy(&pixn);
            pixDestroy(&pixg);
        } else {
            pixb = pixClone(pixs);
        }
#else
            /* Don't normalize; just threshold and clean edges */
        if (d > 1) {
            pixg = pixConvertTo8(pixs, 0);
            pixb = pixThresholdToBinary(pixg, 100);
            pixDestroy(&pixg);
        } else {
            pixb = pixClone(pixs);
        }
        pixSetOrClearBorder(pixb, 30, 30, 40, 40, PIX_CLR);
#endif

            /* Run the basic functions */
        pixWrite("/tmp/lept/dewarp/pixb.tif", pixb, IFF_TIFF_G4);
        dew1 = dewarpCreate(pixb, pageno);
        dewarpaInsertDewarp(dewa, dew1);
        dewarpBuildPageModel(dew1, "/tmp/lept/dewarp/test2_model.pdf");
        dewarpaApplyDisparity(dewa, pageno, pixb, -1, 0, 0, &pixd,
                              "/tmp/lept/dewarp/test2_apply.pdf");

        dewarpaInfo(stderr, dewa);
        dewarpaDestroy(&dewa);
        pixDestroy(&pixb);
    }

    pixDestroy(&pixs);
    pixDestroy(&pixd);
    return 0;
}
