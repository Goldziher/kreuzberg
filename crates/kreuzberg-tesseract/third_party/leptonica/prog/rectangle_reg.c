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
 *  rectangle_reg.c
 *
 *    Tests the largest rectangle in bg or fg.
 *
 *    Also tests finding rectangles associated with single
 *    connected components.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include <string.h>
#include "allheaders.h"

static const l_int32  NBoxes = 20;
static const l_int32  Polarity = 0;  /* background */

int main(int    argc,
         char **argv)
{
char          buf[64];
char         *newpath;
l_int32       i, bx, by, bw, bh, index, rval, gval, bval;
BOX          *box1, *box2;
BOXA         *boxa;
PIX          *pixs, *pix1, *pix2, *pix3;
PIXCMAP      *cmap;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    /* ---------------- Largest rectangles in image ---------------- */
    pixs = pixRead("test1.png");
    pix1 = pixConvertTo8(pixs, FALSE);
    cmap = pixcmapCreateRandom(8, 1, 1);
    pixSetColormap(pix1, cmap);

    boxa = boxaCreate(0);
    for (i = 0; i < NBoxes; i++) {
        pixFindLargestRectangle(pixs, Polarity, &box1, NULL);
        boxGetGeometry(box1, &bx, &by, &bw, &bh);
        pixSetInRect(pixs, box1);
        if (rp->display)
            lept_stderr("bx = %5d, by = %5d, bw = %5d, bh = %5d, area = %d\n",
                        bx, by, bw, bh, bw * bh);
        boxaAddBox(boxa, box1, L_INSERT);
    }

    for (i = 0; i < NBoxes; i++) {
        index = 32 + (i & 254);
        pixcmapGetColor(cmap, index, &rval, &gval, &bval);
        box1 = boxaGetBox(boxa, i, L_CLONE);
        pixRenderHashBoxArb(pix1, box1, 6, 2, L_NEG_SLOPE_LINE, 1,
                            rval, gval, bval);
        boxDestroy(&box1);
    }
    pix2 = pixAddBorder(pix1, 2, 0x0);
    pix3 = pixAddBorder(pix2, 20, 0xffffff00);
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 0 */
    pixDisplayWithTitle(pix3, 0, 0, NULL, rp->display);
    pixDestroy(&pixs);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    boxaDestroy(&boxa);

    /* ----------- Rectangle(s) from connected component ----------- */
    pixs = pixRead("singlecc.tif");
    pix1 = pixScale(pixs, 0.5, 0.5);
    boxa = pixConnCompBB(pix1, 8);
    box1 = boxaGetBox(boxa, 0, L_COPY);

        /* Do 4 cases with vertical scan */
    box2 = pixFindRectangleInCC(pix1, box1, 0.75, L_SCAN_VERTICAL,
                                L_GEOMETRIC_UNION, TRUE);
    boxDestroy(&box2);
    snprintf(buf, sizeof(buf), "rectangle.%02d.png", 2);
    lept_cp("/tmp/lept/rect/fitrect.png", "lept/regout", buf, &newpath);
    regTestCheckFile(rp, newpath);  /* 1 */
    if (rp->display) l_fileDisplay(newpath, 0, 500, 0.4);
    lept_free(newpath);

    box2 = pixFindRectangleInCC(pix1, box1, 0.75, L_SCAN_VERTICAL,
                                L_GEOMETRIC_INTERSECTION, TRUE);
    boxDestroy(&box2);
    snprintf(buf, sizeof(buf), "rectangle.%02d.png", 3);
    lept_cp("/tmp/lept/rect/fitrect.png", "lept/regout", buf, &newpath);
    regTestCheckFile(rp, newpath);  /* 2 */
    if (rp->display) l_fileDisplay(newpath, 200, 500, 0.4);
    lept_free(newpath);

    box2 = pixFindRectangleInCC(pix1, box1, 0.75, L_SCAN_VERTICAL,
                                L_LARGEST_AREA, TRUE);
    boxDestroy(&box2);
    snprintf(buf, sizeof(buf), "rectangle.%02d.png", 4);
    lept_cp("/tmp/lept/rect/fitrect.png", "lept/regout", buf, &newpath);
    regTestCheckFile(rp, newpath);  /* 3 */
    if (rp->display) l_fileDisplay(newpath, 400, 500, 0.4);
    lept_free(newpath);

    box2 = pixFindRectangleInCC(pix1, box1, 0.75, L_SCAN_VERTICAL,
                                L_SMALLEST_AREA, TRUE);
    boxDestroy(&box2);
    snprintf(buf, sizeof(buf), "rectangle.%02d.png", 5);
    lept_cp("/tmp/lept/rect/fitrect.png", "lept/regout", buf, &newpath);
    regTestCheckFile(rp, newpath);  /* 4 */
    if (rp->display) l_fileDisplay(newpath, 600, 500, 0.4);
    lept_free(newpath);

        /* Do 4 cases with horizontal scan */
    box2 = pixFindRectangleInCC(pix1, box1, 0.75, L_SCAN_HORIZONTAL,
                                L_GEOMETRIC_UNION, TRUE);
    boxDestroy(&box2);
    snprintf(buf, sizeof(buf), "rectangle.%02d.png", 6);
    lept_cp("/tmp/lept/rect/fitrect.png", "lept/regout", buf, &newpath);
    regTestCheckFile(rp, newpath);  /* 5 */
    if (rp->display) l_fileDisplay(newpath, 800, 500, 0.4);
    lept_free(newpath);

    box2 = pixFindRectangleInCC(pix1, box1, 0.75, L_SCAN_HORIZONTAL,
                                L_GEOMETRIC_INTERSECTION, TRUE);
    boxDestroy(&box2);
    snprintf(buf, sizeof(buf), "rectangle.%02d.png", 7);
    lept_cp("/tmp/lept/rect/fitrect.png", "lept/regout", buf, &newpath);
    regTestCheckFile(rp, newpath);  /* 6 */
    if (rp->display) l_fileDisplay(newpath, 1000, 500, 0.4);
    lept_free(newpath);

    box2 = pixFindRectangleInCC(pix1, box1, 0.75, L_SCAN_HORIZONTAL,
                                L_LARGEST_AREA, TRUE);
    boxDestroy(&box2);
    snprintf(buf, sizeof(buf), "rectangle.%02d.png", 8);
    lept_cp("/tmp/lept/rect/fitrect.png", "lept/regout", buf, &newpath);
    regTestCheckFile(rp, newpath);  /* 7 */
    if (rp->display) l_fileDisplay(newpath, 1200, 500, 0.4);
    lept_free(newpath);

    box2 = pixFindRectangleInCC(pix1, box1, 0.75, L_SCAN_HORIZONTAL,
                                L_SMALLEST_AREA, TRUE);
    boxDestroy(&box2);
    snprintf(buf, sizeof(buf), "rectangle.%02d.png", 9);
    lept_cp("/tmp/lept/rect/fitrect.png", "lept/regout", buf, &newpath);
    regTestCheckFile(rp, newpath);  /* 8 */
    if (rp->display) l_fileDisplay(newpath, 1400, 500, 0.4);
    lept_free(newpath);

    boxDestroy(&box1);
    pixDestroy(&pixs);
    pixDestroy(&pix1);
    boxaDestroy(&boxa);
    return regTestCleanup(rp);
}

