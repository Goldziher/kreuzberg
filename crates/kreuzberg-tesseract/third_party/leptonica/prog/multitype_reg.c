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
 * multitype_reg.c
 *
 *    Tests various functions against a set of the different image types.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

static const char *fnames[10] = {"feyn-fract.tif", "speckle2.png",
                                 "weasel2.4g.png", "speckle4.png",
                                 "weasel4.16c.png", "dreyfus8.png",
                                 "weasel8.240c.png", "test8.jpg",
                                 "marge.jpg", "test-gray-alpha.png"};

    /* Affine uses the first 3 pt pairs; projective & bilinear use all 4 */
static const l_int32  xs[] = {300, 1200, 225, 750};
static const l_int32  xd[] = {330, 1225, 250, 870};
static const l_int32  ys[] = {1250, 1120, 250, 200};
static const l_int32  yd[] = {1150, 1200, 250, 290};

enum {
  PROJECTIVE = 1,
  BILINEAR = 2
};

static l_float32 *Generate3PtTransformVector();
static l_float32 *Generate4PtTransformVector(l_int32 type);

#define  DO_ALL   1

int main(int    argc,
         char **argv)
{
l_int32       w, h, x, y, i, n;
l_float32    *vc;
PIX          *pix1, *pix2, *pix3, *pix4, *pix5;
PIXA         *pixas, *pixa;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    pixas = pixaCreate(11);
    for (i = 0; i < 10; i++) {  /* this preserves any alpha */
        pix1 = pixRead(fnames[i]);
        pix2 = pixScaleBySamplingToSize(pix1, 250, 150);
        pixaAddPix(pixas, pix2, L_INSERT);
        pixDestroy(&pix1);
    }

        /* Add a transparent grid over the rgb image */
    pix1 = pixaGetPix(pixas, 8, L_COPY);
    pixGetDimensions(pix1, &w, &h, NULL);
    pix2 = pixCreate(w, h, 1);
    for (i = 0; i < 5; i++) {
        y = h * (i + 1) / 6;
        pixRenderLine(pix2, 0, y, w, y, 3, L_SET_PIXELS);
    }
    for (i = 0; i < 7; i++) {
        x = w * (i + 1) / 8;
        pixRenderLine(pix2, x, 0, x, h, 3, L_SET_PIXELS);
    }
    pix3 = pixConvertTo8(pix2, 0);  /* 1 --> 0 ==> transparent */
    pixSetRGBComponent(pix1, pix3, L_ALPHA_CHANNEL);
    pixaAddPix(pixas, pix1, L_INSERT);
    n = pixaGetCount(pixas);
    pixDestroy(&pix2);
    pixDestroy(&pix3);

#if DO_ALL
        /* Display with and without removing alpha with white bg */
    pix1 = pixaDisplayTiledInRows(pixas, 32, 1200, 1.0, 0, 25, 2);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 0 */
    pixDisplayWithTitle(pix1, 0, 0, NULL, rp->display);
    pixDestroy(&pix1);
    pixa = pixaCreate(n);
    for (i = 0; i < n; i++) {
        pix1 = pixaGetPix(pixas, i, L_COPY);
        pix2 = pixRemoveAlpha(pix1);
        pixaAddPix(pixa, pix2, L_INSERT);
        pixDestroy(&pix1);
    }
    pix1 = pixaDisplayTiledInRows(pixa, 32, 1200, 1.0, 0, 25, 2);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 1 */
    pixDisplayWithTitle(pix1, 200, 0, NULL, rp->display);
    pixDestroy(&pix1);
    pixaDestroy(&pixa);
#endif

#if DO_ALL
         /* Setting to gray */
    pixa = pixaCreate(n);
    for (i = 0; i < n; i++) {
        pix1 = pixaGetPix(pixas, i, L_COPY);
        pixSetAllGray(pix1, 170);
        pix2 = pixRemoveAlpha(pix1);
        pixaAddPix(pixa, pix2, L_INSERT);
        pixDestroy(&pix1);
    }
    pix1 = pixaDisplayTiledInRows(pixa, 32, 1200, 1.0, 0, 25, 2);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 2 */
    pixDisplayWithTitle(pix1, 400, 0, NULL, rp->display);
    pixDestroy(&pix1);
    pixaDestroy(&pixa);
#endif

#if DO_ALL
        /* General scaling */
    pixa = pixaCreate(n);
    for (i = 0; i < n; i++) {
        pix1 = pixaGetPix(pixas, i, L_COPY);
        pix2 = pixScaleToSize(pix1, 350, 650);
        pix3 = pixScaleToSize(pix2, 200, 200);
        pix4 = pixRemoveAlpha(pix3);
        pixaAddPix(pixa, pix4, L_INSERT);
        pixDestroy(&pix1);
        pixDestroy(&pix2);
        pixDestroy(&pix3);
    }
    pix1 = pixaDisplayTiledInRows(pixa, 32, 1200, 1.0, 0, 25, 2);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 3 */
    pixDisplayWithTitle(pix1, 600, 0, NULL, rp->display);
    pixDestroy(&pix1);
    pixaDestroy(&pixa);
#endif

#if DO_ALL
        /* Scaling by sampling */
    pixa = pixaCreate(n);
    for (i = 0; i < n; i++) {
        pix1 = pixaGetPix(pixas, i, L_COPY);
        pix2 = pixScaleBySamplingToSize(pix1, 350, 650);
        pix3 = pixScaleBySamplingToSize(pix2, 200, 200);
        pix4 = pixRemoveAlpha(pix3);
        pixaAddPix(pixa, pix4, L_INSERT);
        pixDestroy(&pix1);
        pixDestroy(&pix2);
        pixDestroy(&pix3);
    }
    pix1 = pixaDisplayTiledInRows(pixa, 32, 1200, 1.0, 0, 25, 2);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 4 */
    pixDisplayWithTitle(pix1, 800, 0, NULL, rp->display);
    pixDestroy(&pix1);
    pixaDestroy(&pixa);
#endif

#if DO_ALL
        /* Rotation by area mapping; no embedding */
    pixa = pixaCreate(n);
    for (i = 0; i < n; i++) {
        pix1 = pixaGetPix(pixas, i, L_COPY);
        pix2 = pixRotate(pix1, 0.25, L_ROTATE_AREA_MAP,
                         L_BRING_IN_WHITE, 0, 0);
        pix3 = pixRotate(pix2, -0.35, L_ROTATE_AREA_MAP,
                         L_BRING_IN_WHITE, 0, 0);
        pix4 = pixRemoveAlpha(pix3);
        pixaAddPix(pixa, pix4, L_INSERT);
        pixDestroy(&pix1);
        pixDestroy(&pix2);
        pixDestroy(&pix3);
    }
    pix1 = pixaDisplayTiledInRows(pixa, 32, 1200, 1.0, 0, 25, 2);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 5 */
    pixDisplayWithTitle(pix1, 1000, 0, NULL, rp->display);
    pixDestroy(&pix1);
    pixaDestroy(&pixa);
#endif

#if DO_ALL
        /* Rotation by area mapping; with embedding */
    pixa = pixaCreate(n);
    for (i = 0; i < n; i++) {
        pix1 = pixaGetPix(pixas, i, L_COPY);
        pix2 = pixRotate(pix1, 0.25, L_ROTATE_AREA_MAP,
                         L_BRING_IN_WHITE, 250, 150);
        pix3 = pixRotate(pix2, -0.35, L_ROTATE_AREA_MAP,
                         L_BRING_IN_WHITE, 250, 150);
        pix4 = pixRemoveBorderToSize(pix3, 250, 150);
        pix5 = pixRemoveAlpha(pix4);
        pixaAddPix(pixa, pix5, L_INSERT);
        pixDestroy(&pix1);
        pixDestroy(&pix2);
        pixDestroy(&pix3);
        pixDestroy(&pix4);
    }
    pix1 = pixaDisplayTiledInRows(pixa, 32, 1200, 1.0, 0, 25, 2);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 6 */
    pixDisplayWithTitle(pix1, 0, 400, NULL, rp->display);
    pixDestroy(&pix1);
    pixaDestroy(&pixa);
#endif

#if DO_ALL
        /* Rotation by 3-shear; no embedding */
    pixa = pixaCreate(n);
    for (i = 0; i < n; i++) {
        pix1 = pixaGetPix(pixas, i, L_COPY);
        pix2 = pixRotate(pix1, 0.25, L_ROTATE_SHEAR,
                         L_BRING_IN_WHITE, 0, 0);
        pix3 = pixRotate(pix2, -0.35, L_ROTATE_SHEAR,
                         L_BRING_IN_WHITE, 0, 0);
        pix4 = pixRemoveAlpha(pix3);
        pixaAddPix(pixa, pix4, L_INSERT);
        pixDestroy(&pix1);
        pixDestroy(&pix2);
        pixDestroy(&pix3);
    }
    pix1 = pixaDisplayTiledInRows(pixa, 32, 1200, 1.0, 0, 25, 2);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 7 */
    pixDisplayWithTitle(pix1, 200, 400, NULL, rp->display);
    pixDestroy(&pix1);
    pixaDestroy(&pixa);
#endif

#if DO_ALL
        /* Rotation by 3-shear; with embedding */
    pixa = pixaCreate(n);
    for (i = 0; i < n; i++) {
        pix1 = pixaGetPix(pixas, i, L_COPY);
        pix2 = pixRotate(pix1, 0.25, L_ROTATE_SHEAR,
                         L_BRING_IN_WHITE, 250, 150);
        pix3 = pixRotate(pix2, -0.35, L_ROTATE_SHEAR,
                         L_BRING_IN_WHITE, 250, 150);
        pix4 = pixRemoveBorderToSize(pix3, 250, 150);
        pix5 = pixRemoveAlpha(pix4);
        pixaAddPix(pixa, pix5, L_INSERT);
        pixDestroy(&pix1);
        pixDestroy(&pix2);
        pixDestroy(&pix3);
        pixDestroy(&pix4);
    }
    pix1 = pixaDisplayTiledInRows(pixa, 32, 1200, 1.0, 0, 25, 2);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 8 */
    pixDisplayWithTitle(pix1, 400, 400, NULL, rp->display);
    pixDestroy(&pix1);
    pixaDestroy(&pixa);
#endif

#if DO_ALL
        /* Rotation by 2-shear about the center */
    pixa = pixaCreate(n);
    for (i = 0; i < n; i++) {
        pix1 = pixaGetPix(pixas, i, L_COPY);
        pixGetDimensions(pix1, &w, &h, NULL);
        pix2 = pixRotate2Shear(pix1, w / 2, h / 2, 0.25, L_BRING_IN_WHITE);
        pix3 = pixRotate2Shear(pix2, w / 2, h / 2, -0.35, L_BRING_IN_WHITE);
        pix4 = pixRemoveAlpha(pix3);
        pixaAddPix(pixa, pix4, L_INSERT);
        pixDestroy(&pix1);
        pixDestroy(&pix2);
        pixDestroy(&pix3);
    }
    pix1 = pixaDisplayTiledInRows(pixa, 32, 1200, 1.0, 0, 25, 2);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 9 */
    pixDisplayWithTitle(pix1, 600, 400, NULL, rp->display);
    pixDestroy(&pix1);
    pixaDestroy(&pixa);
#endif

#if DO_ALL
        /* Rotation by sampling; no embedding */
    pixa = pixaCreate(n);
    for (i = 0; i < n; i++) {
        pix1 = pixaGetPix(pixas, i, L_COPY);
        pix2 = pixRotate(pix1, 0.25, L_ROTATE_SAMPLING,
                         L_BRING_IN_WHITE, 0, 0);
        pix3 = pixRotate(pix2, -0.35, L_ROTATE_SAMPLING,
                         L_BRING_IN_WHITE, 0, 0);
        pix4 = pixRemoveAlpha(pix3);
        pixaAddPix(pixa, pix4, L_INSERT);
        pixDestroy(&pix1);
        pixDestroy(&pix2);
        pixDestroy(&pix3);
    }
    pix1 = pixaDisplayTiledInRows(pixa, 32, 1200, 1.0, 0, 25, 2);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 10 */
    pixDisplayWithTitle(pix1, 800, 400, NULL, rp->display);
    pixDestroy(&pix1);
    pixaDestroy(&pixa);
#endif

#if DO_ALL
        /* Rotation by sampling; with embedding */
    pixa = pixaCreate(n);
    for (i = 0; i < n; i++) {
        pix1 = pixaGetPix(pixas, i, L_COPY);
        pix2 = pixRotate(pix1, 0.25, L_ROTATE_SAMPLING,
                         L_BRING_IN_WHITE, 250, 150);
        pix3 = pixRotate(pix2, -0.35, L_ROTATE_SAMPLING,
                         L_BRING_IN_WHITE, 250, 150);
        pix4 = pixRemoveBorderToSize(pix3, 250, 150);
        pix5 = pixRemoveAlpha(pix4);
        pixaAddPix(pixa, pix5, L_INSERT);
        pixDestroy(&pix1);
        pixDestroy(&pix2);
        pixDestroy(&pix3);
        pixDestroy(&pix4);
    }
    pix1 = pixaDisplayTiledInRows(pixa, 32, 1200, 1.0, 0, 25, 2);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 11 */
    pixDisplayWithTitle(pix1, 1000, 400, NULL, rp->display);
    pixDestroy(&pix1);
    pixaDestroy(&pixa);
#endif

#if DO_ALL
        /* Rotation by area mapping at corner */
    pixa = pixaCreate(n);
    for (i = 0; i < n; i++) {
        pix1 = pixaGetPix(pixas, i, L_COPY);
        pix2 = pixRotateAMCorner(pix1, 0.25, L_BRING_IN_WHITE);
        pix3 = pixRotateAMCorner(pix2, -0.35, L_BRING_IN_WHITE);
        pix4 = pixRemoveAlpha(pix3);
        pixaAddPix(pixa, pix4, L_INSERT);
        pixDestroy(&pix1);
        pixDestroy(&pix2);
        pixDestroy(&pix3);
    }
    pix1 = pixaDisplayTiledInRows(pixa, 32, 1200, 1.0, 0, 25, 2);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 12 */
    pixDisplayWithTitle(pix1, 0, 800, NULL, rp->display);
    pixDestroy(&pix1);
    pixaDestroy(&pixa);
#endif

#if DO_ALL
        /* Affine transform by interpolation */
    pixa = pixaCreate(n);
    vc = Generate3PtTransformVector();
    for (i = 0; i < n; i++) {
        pix1 = pixaGetPix(pixas, i, L_COPY);
        pix2 = pixAffine(pix1, vc, L_BRING_IN_WHITE);
/*        pix2 = pixAffineSampled(pix1, vc, L_BRING_IN_WHITE); */
        pix3 = pixRemoveAlpha(pix2);
        pixaAddPix(pixa, pix3, L_INSERT);
        pixDestroy(&pix1);
        pixDestroy(&pix2);
    }
    pix1 = pixaDisplayTiledInRows(pixa, 32, 1200, 1.0, 0, 25, 2);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 13 */
    pixDisplayWithTitle(pix1, 200, 800, NULL, rp->display);
    pixDestroy(&pix1);
    pixaDestroy(&pixa);
    lept_free(vc);
#endif

#if DO_ALL
        /* Projective transform by sampling */
    pixa = pixaCreate(n);
    vc = Generate4PtTransformVector(PROJECTIVE);
    for (i = 0; i < n; i++) {
        pix1 = pixaGetPix(pixas, i, L_COPY);
        pix2 = pixProjectiveSampled(pix1, vc, L_BRING_IN_WHITE);
        pix3 = pixRemoveAlpha(pix2);
        pixaAddPix(pixa, pix3, L_INSERT);
        pixDestroy(&pix1);
        pixDestroy(&pix2);
    }
    pix1 = pixaDisplayTiledInRows(pixa, 32, 1200, 1.0, 0, 25, 2);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 14 */
    pixDisplayWithTitle(pix1, 400, 800, NULL, rp->display);
    pixDestroy(&pix1);
    pixaDestroy(&pixa);
    lept_free(vc);
#endif

#if DO_ALL
        /* Projective transform by interpolation */
    pixa = pixaCreate(n);
    vc = Generate4PtTransformVector(PROJECTIVE);
    for (i = 0; i < n; i++) {
        pix1 = pixaGetPix(pixas, i, L_COPY);
        pix2 = pixProjective(pix1, vc, L_BRING_IN_WHITE);
        pix3 = pixRemoveAlpha(pix2);
        pixaAddPix(pixa, pix3, L_INSERT);
        pixDestroy(&pix1);
        pixDestroy(&pix2);
    }
    pix1 = pixaDisplayTiledInRows(pixa, 32, 1200, 1.0, 0, 25, 2);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 15 */
    pixDisplayWithTitle(pix1, 600, 800, NULL, rp->display);
    pixDestroy(&pix1);
    pixaDestroy(&pixa);
    lept_free(vc);
#endif

#if DO_ALL
        /* Bilinear transform by interpolation */
    pixa = pixaCreate(n);
    vc = Generate4PtTransformVector(BILINEAR);
    for (i = 0; i < n; i++) {
        pix1 = pixaGetPix(pixas, i, L_COPY);
        pix2 = pixBilinear(pix1, vc, L_BRING_IN_WHITE);
        pix3 = pixRemoveAlpha(pix2);
        pixaAddPix(pixa, pix3, L_INSERT);
        pixDestroy(&pix1);
        pixDestroy(&pix2);
    }
    pix1 = pixaDisplayTiledInRows(pixa, 32, 1200, 1.0, 0, 25, 2);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 16 */
    pixDisplayWithTitle(pix1, 800, 800, NULL, rp->display);
    pixDestroy(&pix1);
    pixaDestroy(&pixa);
    lept_free(vc);
#endif

    pixaDestroy(&pixas);
    return regTestCleanup(rp);
}


static l_float32 *
Generate3PtTransformVector()
{
l_int32     i;
l_float32  *vc;
PTA        *ptas, *ptad;

    ptas = ptaCreate(3);
    ptad = ptaCreate(3);
    for (i = 0; i < 3; i++) {
        ptaAddPt(ptas, xs[i], ys[i]);
        ptaAddPt(ptad, xd[i], yd[i]);
    }

    getAffineXformCoeffs(ptad, ptas, &vc);
    ptaDestroy(&ptas);
    ptaDestroy(&ptad);
    return vc;
}

static l_float32 *
Generate4PtTransformVector(l_int32 type)
{
l_int32     i;
l_float32  *vc;
PTA        *ptas, *ptad;

    ptas = ptaCreate(4);
    ptad = ptaCreate(4);
    for (i = 0; i < 4; i++) {
        ptaAddPt(ptas, xs[i], ys[i]);
        ptaAddPt(ptad, xd[i], yd[i]);
    }

    if (type == PROJECTIVE)
        getProjectiveXformCoeffs(ptad, ptas, &vc);
    else  /* BILINEAR */
        getBilinearXformCoeffs(ptad, ptas, &vc);
    ptaDestroy(&ptas);
    ptaDestroy(&ptad);
    return vc;
}
