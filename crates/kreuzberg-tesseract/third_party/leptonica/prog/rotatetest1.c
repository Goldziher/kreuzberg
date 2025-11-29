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
 * rotatetest1.c
 *
 *    rotatetest1 filein angle(in degrees) fileout
 */

#include "allheaders.h"

#define  NTIMES   180
#define  NITERS   3

int main(int    argc,
         char **argv)
{
l_int32    i, w, h, d, rotflag;
PIX       *pixs, *pixt, *pixd;
l_float32  angle, deg2rad, ang;
char      *filein, *fileout;

    if (argc != 4)
        return ERROR_INT(" Syntax:  rotatetest1 filein angle fileout",
                         __func__, 1);
    filein = argv[1];
    angle = atof(argv[2]);
    fileout = argv[3];

    setLeptDebugOK(1);
    lept_mkdir("lept/rotate");

    deg2rad = 3.1415926535 / 180.;
    if ((pixs = pixRead(filein)) == NULL)
        return ERROR_INT("pix not made", __func__, 1);
    if (pixGetDepth(pixs) == 1) {
        pixt = pixScaleToGray3(pixs);
        pixDestroy(&pixs);
        pixs = pixAddBorderGeneral(pixt, 1, 0, 1, 0, 255);
        pixDestroy(&pixt);
    }

    pixGetDimensions(pixs, &w, &h, &d);
    lept_stderr("w = %d, h = %d\n", w, h);

#if 0
        /* repertory of rotation operations to choose from */
    pixd = pixRotateAM(pixs, deg2rad * angle, L_BRING_IN_WHITE);
    pixd = pixRotateAMColor(pixs, deg2rad * angle, 0xffffff00);
    pixd = pixRotateAMColorFast(pixs, deg2rad * angle, 255);
    pixd = pixRotateAMCorner(pixs, deg2rad * angle, L_BRING_IN_WHITE);
    pixd = pixRotateShear(pixs, w /2, h / 2, deg2rad * angle,
                          L_BRING_IN_WHITE);
    pixd = pixRotate3Shear(pixs, w /2, h / 2, deg2rad * angle,
                           L_BRING_IN_WHITE);
    pixRotateShearIP(pixs, w / 2, h / 2, deg2rad * angle); pixd = pixs;
#endif

#if 0
        /* timing of shear rotation */
    for (i = 0; i < NITERS; i++) {
        pixd = pixRotateShear(pixs, (i * w) / NITERS,
                              (i * h) / NITERS, deg2rad * angle,
                              L_BRING_IN_WHITE);
        pixDisplay(pixd, 100 + 20 * i, 100 + 20 * i);
        pixDestroy(&pixd);
    }
#endif

#if 0
        /* timing of in-place shear rotation */
    for (i = 0; i < NITERS; i++) {
        pixRotateShearIP(pixs, w/2, h/2, deg2rad * angle, L_BRING_IN_WHITE);
/*        pixRotateShearCenterIP(pixs, deg2rad * angle, L_BRING_IN_WHITE); */
        pixDisplay(pixs, 100 + 20 * i, 100 + 20 * i);
    }
    pixd = pixs;
    if (pixGetDepth(pixd) == 1)
        pixWrite(fileout, pixd, IFF_PNG);
    else
        pixWrite(fileout, pixd, IFF_JFIF_JPEG);
    pixDestroy(&pixs);
#endif

#if 0
{
    l_float32  pops;

        /* timing of various rotation operations (choose) */
    startTimer();
    w = pixGetWidth(pixs);
    h = pixGetHeight(pixs);
    for (i = 0; i < NTIMES; i++) {
        pixd = pixRotateShearCenter(pixs, deg2rad * angle, L_BRING_IN_WHITE);
        pixDestroy(&pixd);
    }
    pops = (l_float32)(w * h * NTIMES / 1000000.) / stopTimer();
    lept_stderr("vers. 1, mpops: %f\n", pops);
    startTimer();
    w = pixGetWidth(pixs);
    h = pixGetHeight(pixs);
    for (i = 0; i < NTIMES; i++) {
        pixRotateShearIP(pixs, w/2, h/2, deg2rad * angle, L_BRING_IN_WHITE);
    }
    pops = (l_float32)(w * h * NTIMES / 1000000.) / stopTimer();
    lept_stderr("shear, mpops: %f\n", pops);
    pixWrite(fileout, pixs, IFF_PNG);
    for (i = 0; i < NTIMES; i++) {
        pixRotateShearIP(pixs, w/2, h/2, -deg2rad * angle, L_BRING_IN_WHITE);
    }
    pixWrite("/usr/tmp/junkout", pixs, IFF_PNG);
}
#endif

#if 0
        /* area-mapping rotation operations */
    pixd = pixRotateAM(pixs, deg2rad * angle, L_BRING_IN_WHITE);
/*    pixd = pixRotateAMColorFast(pixs, deg2rad * angle, 255); */
    if (pixGetDepth(pixd) == 1)
        pixWrite(fileout, pixd, IFF_PNG);
    else
        pixWrite(fileout, pixd, IFF_JFIF_JPEG);
#endif

#if 0
        /* compare the standard area-map color rotation with
         * the fast area-map color rotation, on a pixel basis */
    {
    PIX    *pix1, *pix2;
    NUMA   *nar, *nag, *nab, *naseq;
    GPLOT  *gplot;

    startTimer();
    pix1 = pixRotateAMColor(pixs, 0.12, 0xffffff00);
    lept_stderr(" standard color rotate: %7.2f sec\n", stopTimer());
    pixWrite("/tmp/lept/rotate/color1.jpg", pix1, IFF_JFIF_JPEG);
    startTimer();
    pix2 = pixRotateAMColorFast(pixs, 0.12, 0xffffff00);
    lept_stderr(" fast color rotate: %7.2f sec\n", stopTimer());
    pixWrite("/tmp/lept/rotate/color2.jpg", pix2, IFF_JFIF_JPEG);
    pixd = pixAbsDifference(pix1, pix2);
    pixGetColorHistogram(pixd, 1, &nar, &nag, &nab);
    naseq = numaMakeSequence(0., 1., 256);
    gplot = gplotCreate("/tmp/lept/rotate/absdiff", GPLOT_PNG,
                        "Number vs diff", "diff", "number");
    gplotAddPlot(gplot, naseq, nar, GPLOT_POINTS, "red");
    gplotAddPlot(gplot, naseq, nag, GPLOT_POINTS, "green");
    gplotAddPlot(gplot, naseq, nab, GPLOT_POINTS, "blue");
    gplotMakeOutput(gplot);
    l_fileDisplay("/tmp/lept/rotate/absdiff.png", 100, 100, 1.0);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pixd);
    numaDestroy(&nar);
    numaDestroy(&nag);
    numaDestroy(&nab);
    numaDestroy(&naseq);
    gplotDestroy(&gplot);
    }
#endif

        /* Do a succession of 180 7-degree rotations in a cw
         * direction, and unwind the result with another set in
         * a ccw direction.  Although there is a considerable amount
         * of distortion after successive rotations, after all
         * 360 rotations, the resulting image is restored to
         * its original pristine condition! */
#if 1
    rotflag = L_ROTATE_AREA_MAP;
/*    rotflag = L_ROTATE_SHEAR;     */
/*    rotflag = L_ROTATE_SAMPLING;   */
    ang = 7.0 * deg2rad;
    pixGetDimensions(pixs, &w, &h, NULL);
    pixd = pixRotate(pixs, ang, rotflag, L_BRING_IN_WHITE, w, h);
    pixWrite("/tmp/lept/rotate/rot7.png", pixd, IFF_PNG);
    for (i = 1; i < 180; i++) {
        pixs = pixd;
        pixd = pixRotate(pixs, ang, rotflag, L_BRING_IN_WHITE, w, h);
        if ((i % 30) == 0)  pixDisplay(pixd, 600, 0);
        pixDestroy(&pixs);
    }

    pixWrite("/tmp/lept/rotate/spin.png", pixd, IFF_PNG);
    pixDisplay(pixd, 0, 0);

    for (i = 0; i < 180; i++) {
        pixs = pixd;
        pixd = pixRotate(pixs, -ang, rotflag, L_BRING_IN_WHITE, w, h);
        if (i && (i % 30) == 0)  pixDisplay(pixd, 600, 500);
        pixDestroy(&pixs);
    }

    pixWrite("/tmp/lept/rotate/unspin.png", pixd, IFF_PNG);
    pixDisplay(pixd, 0, 500);
    pixDestroy(&pixd);
#endif

    return 0;
}

