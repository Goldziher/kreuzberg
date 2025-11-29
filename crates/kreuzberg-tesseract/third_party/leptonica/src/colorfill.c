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

/*!
 * \file colorfill.c
 * <pre>
 *
 *      Determine color content using proximity.  What can we say about the
 *      color in an image from growing regions with nearly the same color?
 *
 *         L_COLORFILL  *l_colorfillCreate()
 *         void          l_colorfillDestroy()
 *
 *         L_COLORFILL  *pixColorContentByLocation()
 *         PIX          *pixColorFill()
 *
 *      Generate data for testing
 *         PIXA         *makeColorfillTestData()
 *
 *      Static helpers
 *         static COLOREL      *colorelCreate()
 *         static void          pixColorFillFromSeed()
 *         static void          pixGetVisitedNeighbors()
 *         static l_int32       findNextUnvisited()
 *         static l_int32       colorsAreSimilarForFill()
 *         static void          pixelColorIsValid()
 *         static l_int32       pixelIsOnColorBoundary()
 *         static l_int32       evalColorfillData()
 *
 *  See colorcontent.c for location-independent measures of the amount
 *  of color in an image.
 * </pre>
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

struct ColorEl {
    l_int32   x;
    l_int32   y;
    l_uint32  color;
};
typedef struct ColorEl  COLOREL;

    /* Ignore pixels with smaller max component */
static l_int32 DefaultMinMax = 70;

    /* Static helpers */
static COLOREL *colorelCreate(l_int32 x, l_int32 y, l_uint32 color);
static void pixColorFillFromSeed(PIX *pixs, PIX *pixv, PTA **ppta,
                                 l_int32 x, l_int32 y, L_QUEUE *lq,
                                 l_int32 maxdiff, l_int32 minarea,
                                 l_int32 debug);
static void pixGetVisitedNeighbors(PIX *pixs, l_int32 x, l_int32 y,
                                   l_uint32 *visited);
static l_int32 findNextUnvisited(PIX *pixv, l_int32 *px, l_int32 *py);
static l_int32 colorsAreSimilarForFill(l_uint32 val1, l_uint32 val2,
                                       l_int32 maxdiff);
static l_int32 pixelColorIsValid(l_uint32 val, l_int32 minmax);
static l_int32 pixelIsOnColorBoundary(PIX *pixs, l_int32 x, l_int32 y);
static l_int32 evalColorfillData(L_COLORFILL *cf, l_int32 debug);


/*---------------------------------------------------------------------*
 *                   Colorfill creation and destruction                *
 *---------------------------------------------------------------------*/
/*!
 * \brief   l_colorfillCreate()
 *
 * \param[in]    pixs   input RGB image
 * \param[in]    nx     requested number of tiles in each row
 * \param[in]    ny     requested number of tiles in each column
 * \return  boxa, or NULL on error
 *
 * <pre>
 * Notes:
 *      (1) Tiles must at least 10 pixels in each dimension.
 * </pre>
 */
L_COLORFILL *
l_colorfillCreate(PIX     *pixs,
                  l_int32  nx,
                  l_int32  ny)
{
l_int32       i, j, w, h, tw, th, ntiles;
BOX          *box;
BOXA         *boxas;
L_COLORFILL  *cf;

    if (!pixs)
        return (L_COLORFILL *)ERROR_PTR("pixs not defined", __func__, NULL);
    if (pixGetDepth(pixs) != 32)
        return (L_COLORFILL *)ERROR_PTR("pixs not 32 bpp", __func__, NULL);

    pixGetDimensions(pixs, &w, &h, NULL);
    tw = w / nx;
    th = h / ny;
    if (tw < 10 || th < 10)
        return (L_COLORFILL *)ERROR_PTR("tile size too small", __func__, NULL);
    boxas = boxaCreate(nx * ny);
    for (i = 0; i < ny; i++) {
        for (j = 0; j < nx; j++) {
            box = boxCreate(j * tw, i * th, tw, th);
            boxaAddBox(boxas, box, L_INSERT);
        }
    }
    ntiles = nx * ny;

    cf = (L_COLORFILL *)LEPT_CALLOC(1, sizeof(L_COLORFILL));
    cf->pixs = pixClone(pixs);
    cf->nx = nx;
    cf->ny = ny;
    cf->tw = tw;
    cf->th = th;
    cf->boxas = boxas;
    cf->naa = numaaCreate(ntiles);
    cf->dnaa = l_dnaaCreate(ntiles);
    cf->pixadb = pixaCreate(0);
    return cf;
}


/*!
 * \brief   l_colorfillDestroy()
 *
 * \param[in,out]   pcf    will be set to null before returning
 * \return  void
 */
void
l_colorfillDestroy(L_COLORFILL  **pcf)
{
L_COLORFILL  *cf;

    if (pcf == NULL) {
        L_WARNING("ptr address is null!\n", __func__);
        return;
    }

    if ((cf = *pcf) == NULL)
        return;

    pixDestroy(&cf->pixs);
    pixDestroy(&cf->pixst);
    boxaDestroy(&cf->boxas);
    pixaDestroy(&cf->pixas);
    pixaDestroy(&cf->pixam);
    numaaDestroy(&cf->naa);
    l_dnaaDestroy(&cf->dnaa);
    pixaDestroy(&cf->pixadb);
    LEPT_FREE(cf);
    *pcf = NULL;
}


/* ----------------------------------------------------------------------- *
 *    Determine color content using proximity.  What do we get when        *
 *    growing regions with nearly the same color?                          *
 * ----------------------------------------------------------------------- */
/*!
 * \brief   pixColorContentByLocation()
 *
 * \param[in]    cf        colorfill
 * \param[in]    rref      reference value for red component
 * \param[in]    gref      reference value for green component
 * \param[in]    bref      reference value for blue component
 * \param[in]    minmax    min of max component for possible color region
 * \param[in]    maxdiff   max component diff to be in same color region
 * \param[in]    minarea   min number of pixels for a color region
 * \param[in]    smooth    low-pass kernel size (1,3,5); use 1 to skip
 * \param[in]    debug     generates debug images and fill info
 * \return  0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *      (1) This computes color fill information in each tile, identifying
 *          regions of approximately constant color.  It does this
 *          independently for each tile, using flood fills.  Regions
 *          of low intensity are considered 'not colorful'.
 *      (2) The three numbers (rref, gref and bref) can be thought
 *          of in two ways:
 *            (a) as rgb values in the image corresponding to white,
 *                to compensate for an unbalanced color white point.
 *            (b) as the median or mean values of the background color
 *                of a scan.
 *          The gamma TRC transformation, which does not change hue, is used
 *          to modify all colors so that these reference values become white.
 *          These three numbers must either be all 0 or all non-zero.
 *          To skip the TRC transform, set them all to 0.
 *      (3) If the maximum component after white point correction,
 *          max(r,g,b), is less than minmax, the pixel color is invalid, and it
 *          is assigned its neighbor's value in the filling operation.
 *          Use %minmax = 0 for a default value.
 * </pre>
 */
l_ok
pixColorContentByLocation(L_COLORFILL  *cf,
                          l_int32       rref,
                          l_int32       gref,
                          l_int32       bref,
                          l_int32       minmax,
                          l_int32       maxdiff,
                          l_int32       minarea,
                          l_int32       smooth,
                          l_int32       debug)
{
l_int32    i, n;
PIX       *pix1, *pix2, *pix3;
PIXA      *pixas, *pixam;

    if (!cf)
        return ERROR_INT("cf not defined", __func__, 1);
    if (minmax <= 0) minmax = DefaultMinMax;
    if (minmax > 200)
        return ERROR_INT("minmax > 200; unreasonably large", __func__, 1);

        /* Do the optional linear color map; this checks the ref vals
         * and uses them if valid.  Use {0,0,0} to skip this operation. */
    if ((pix1 = pixColorShiftWhitePoint(cf->pixs, rref, gref, bref)) == NULL)
        return ERROR_INT("pix1 not returned", __func__, 1);
    cf->pixst = pix1;

        /* Break the image up into small tiles */
    pixas = pixaCreateFromBoxa(pix1, cf->boxas, 0, 0, NULL);
    cf->pixas = pixas;

        /* Find regions of similar color in each tile */
    n = pixaGetCount(pixas);
    pixam = pixaCreate(n);
    cf->pixam = pixam;
    for (i = 0; i < n; i++) {
        pix2 = pixaGetPix(pixas, i, L_COPY);
        pix3 = pixColorFill(pix2, minmax, maxdiff, smooth, minarea, 0);
        pixDestroy(&pix2);
        pixaAddPix(pixam, pix3, L_INSERT);
    }

        /* Evaluate color components.  Find the average color in each
         * component and determine if there is more than one color in
         * each of the tiles. */
    evalColorfillData(cf, debug);

    return 0;
}


/*!
 * \brief   pixColorFill()
 *
 * \param[in]    pixs      32 bpp RGB
 * \param[in]    minmax    min of max component for possible color region
 * \param[in]    maxdiff   max component diff to be in same color region
 * \param[in]    smooth    low-pass kernel size (1,3,5); use 1 to skip
 * \param[in]    minarea   min number of pixels for a color region
 * \param[in]    debug     generates debug images and fill info
 * \return  pixm   mask showing connected regions of similar color,
 *                 or null on error
 *
 * <pre>
 * Notes:
 *      (1) This is the basic color filling operation.  It sets the
 *          non-color pixel to black, optionally does a low-pass filter,
 *          and grows the 8-connected color components.  Finally, it
 *          removes components that have a small area.
 * </pre>
 */
PIX *
pixColorFill(PIX     *pixs,
             l_int32  minmax,
             l_int32  maxdiff,
             l_int32  smooth,
             l_int32  minarea,
             l_int32  debug)
{
l_int32    x, y, w, h;
l_uint32   val;
L_KERNEL  *kel;
PIX       *pixm, *pixm1, *pixv, *pixnc, *pixncd, *pixss, *pixf;
PTA       *pta1;
L_QUEUE   *lq;

    if (!pixs || pixGetDepth(pixs) != 32)
        return (PIX *)ERROR_PTR("pixs undefined or not 32 bpp", __func__, NULL);

        /* Set the non-color pixels to 0; generate a mask representing them */
    pixGetDimensions(pixs, &w, &h, NULL);
    pixnc = pixCreate(w, h, 1);  /* mask for no color */
    for (y = 0; y < h; y++) {
        for (x = 0; x < w; x++) {
            pixGetPixel(pixs, x, y, &val);
            if (!pixelColorIsValid(val, minmax)) {
                pixSetPixel(pixnc, x, y, 1);
                pixSetPixel(pixs, x, y, 0x0);
            }
        }
    }

        /* Optionally, dilate the no-color mask */
    pixncd = pixDilateBrick(NULL, pixnc, smooth, smooth);
    pixDestroy(&pixnc);

        /* Do a low-pass filter on pixs.  This will make bad pixels
         * near the zeroed non-color pixels, but any components made
         * from these pixels will be removed at the end by the
         * (optionally dilated) no-color mask. */
    if (smooth > 1) {
        kel = makeFlatKernel(smooth, smooth, smooth / 2, smooth / 2);
        pixss = pixConvolveRGBSep(pixs, kel, kel);
        kernelDestroy(&kel);
    } else {
        pixss = pixCopy(NULL, pixs);
    }

        /* Paint through everything under pixncd */
    pixPaintThroughMask(pixss, pixncd, 0, 0, 0);

        /* Find the color components */
    pixv = pixCreate(w, h, 1);  /* visited pixels */
    pixOr(pixv, pixv, pixncd);  /* consider non-color as visited */
    pixSetBorderRingVal(pixv, 1, 1);
    pixm = pixCreate(w, h, 1);  /* color components */
    lq = lqueueCreate(0);
    x = y = 1;  /* first row and column have been marked as visited */
    while (findNextUnvisited(pixv, &x, &y) == 1) {
            /* Flood fill this component, starting from (x,y) */
        if (debug) lept_stderr("Start: x = %d, y = %d\n", x, y);
        pixColorFillFromSeed(pixss, pixv, &pta1, x, y, lq, maxdiff,
                             minarea, debug);
        if (pta1) {  /* erode and add the pixels to pixm */
            pixm1 = pixGenerateFromPta(pta1, w, h);
            pixErodeBrick(pixm1, pixm1, 3, 3);
            pixOr(pixm, pixm, pixm1);
            pixDestroy(&pixm1);
            ptaDestroy(&pta1);
        }
    }
    pixDestroy(&pixv);

        /* Remove everything under pixncd */
    pixSubtract(pixm, pixm, pixncd);

        /* Remove remaining small stuff */
    pixf = pixSelectByArea(pixm, minarea, 4, L_SELECT_IF_GTE, NULL);

    lqueueDestroy(&lq, 1);
    pixDestroy(&pixncd);
    pixDestroy(&pixss);
    pixDestroy(&pixm);
    return pixf;
}


/* ----------------------------------------------------------------------- *
 *                         Generate data for testing                       *
 * ----------------------------------------------------------------------- */
/*!
 * \brief   makeColorfillTestData()
 *
 * \param[in]       w         width of generated pix
 * \param[in]       h         height of generated pix
 * \param[in]       nseeds    number of regions
 * \param[in]       range     of color component values
 * \return  pixa   various pix with filled regions of random color,
 *                 or NULL on error
 *
 * <pre>
 * Notes:
 *      (1) The seeds are random points.  The colors are assigned
 *          randomly from a restricted range of component values,
 *          in [128 - range/2 ... 128 + range/2]
 *      (2) Output is pixa:
 *          * pixa[0] cmapped, with color regions shown
 *          - pixa[1] cmapped, additionally with boundary pixels set to black
 *          - pixa[2] cmapped, as in pixa[1] with all non-black pixels
 *                    in the same color
 * </pre>
 */
PIXA *
makeColorfillTestData(l_int32  w,
                      l_int32  h,
                      l_int32  nseeds,
                      l_int32  range)
{
l_int32    i, j, x, y, rval, gval, bval, start, end;
l_uint32   color;
l_float64  dval;
L_DNA     *da;
PIX       *pix1, *pix2, *pix3, *pix4;
PIXA      *pixa;
PTA       *pta;
PIXCMAP   *cmap;

        /* Generate data for seeds */
    pta = ptaCreate(nseeds);  /* for seed locations */
    da = l_dnaCreate(nseeds);  /* for colors */
    srand(4);
    start = 128 - range / 2;
    end = 128 + (range - 1) / 2;
    for (i = 0; i < nseeds; i++) {
        genRandomIntOnInterval(0, w - 1, 0, &x);
        genRandomIntOnInterval(0, h - 1, 0, &y);
        ptaAddPt(pta, x, y);
        genRandomIntOnInterval(start, end, 0, &rval);
        genRandomIntOnInterval(start, end, 0, &gval);
        genRandomIntOnInterval(start, end, 0, &bval);
        composeRGBPixel(rval, gval, bval, &color);
        l_dnaAddNumber(da, color);
    }

        /* Generate the 8 bpp seed image */
    pix1 = pixCreate(w, h, 8);
    for (i = 0; i < nseeds; i++) {
        ptaGetIPt(pta, i, &x, &y);
        pixSetPixel(pix1, x, y, i + 1);  /* all seeds have non-zero values */
    }

        /* Spread seed values to all pixels that are nearest to
         * the seed pixel from which they take their value. */
    pix2 = pixSeedspread(pix1, 4);

        /* Add a colormap for the random colors, using 0 for black */
    cmap = pixcmapCreate(8);
    pixcmapAddColor(cmap, 0, 0, 0);
    for (i = 0; i < nseeds; i++) {
        l_dnaGetDValue(da, i, &dval);
        extractRGBValues(dval, &rval, &gval, &bval);
        pixcmapAddColor(cmap, rval, gval, bval);
    }
    pixSetColormap(pix2, cmap);
    pixDestroy(&pix1);
    ptaDestroy(&pta);
    l_dnaDestroy(&da);

        /* Add to output; no black boundaries */
    pixa = pixaCreate(0);
    pixaAddPix(pixa, pix2, L_COPY);

       /* Make pixels on the color boundaries black */
    pix3 = pixCopy(NULL, pix2);
    for (i = 0; i < h; i++) {
        for (j = 0; j < w; j++) {
            if (pixelIsOnColorBoundary(pix2, j, i))
                pixSetPixel(pix3, j, i, 0);  /* black */
        }
    }
    pixaAddPix(pixa, pix3, L_INSERT);
    pixDestroy(&pix2);

        /* Have all the non-black regions be the same color */
    cmap = pixcmapCreate(8);
    pixcmapAddColor(cmap, 0, 0, 0);
    for (i = 0; i < nseeds; i++)
        pixcmapAddColor(cmap, rval, gval, bval);
    pix4 = pixCopy(NULL, pix3);
    pixSetColormap(pix4, cmap);
    pixaAddPix(pixa, pix4, L_INSERT);

    return pixa;
}


/* ----------------------------------------------------------------------- *
 *                             Static helpers                              *
 * ----------------------------------------------------------------------- */
static COLOREL *
colorelCreate(l_int32   x,
              l_int32   y,
              l_uint32  color)
{
COLOREL *el;

    el = (COLOREL *)LEPT_CALLOC(1, sizeof(COLOREL));
    el->x = x;
    el->y = y;
    el->color = color;
    return el;
}

/*!
 * \brief   pixColorFillFromSeed()
 *
 * \param[in]       pixs         32 bpp rgb
 * \param[in]       pixv         1 bpp labeling visited pixels
 * \param[out]      ppta         points visited with similar colors during fill
 * \param[in]       x            starting x coord for fill (seed)
 * \param[in]       y            starting y coord for fill (seed)
 * \param[in]       lq           head of queue holding pixels
 * \param[in]       maxdiff      max component diff allowed for similar pixels
 * \param[in]       minarea      min size of component to keep
 * \param[in]       debug        output some text data
 * \return  void
 *
 * <pre>
 * Notes:
 *      (1) Use 8-connected filling.  It is faster because it reduces the
 *          number of single-pixel noise components near color boundaries.
 *      (2) The seed pixel at (x,y) is unvisited, and can never be on the
 *          exterior boundary of the tile %pixs.
 *      (3) If the size of the connected component >= %minarea, we return
 *          the array of pixel locations; otherwise, return NULL for the pta.
 * </pre>
 */
static void
pixColorFillFromSeed(PIX      *pixs,
                     PIX      *pixv,
                     PTA     **ppta,
                     l_int32   x,
                     l_int32   y,
                     L_QUEUE  *lq,
                     l_int32   maxdiff,
                     l_int32   minarea,
                     l_int32   debug)
{
l_int32   w, h, np;
l_uint32  visited[8];  /* W, N, E, S, NW, NE, SW, SE */
l_uint32  color, val;
COLOREL  *el;
PTA      *pta;

        /* Prime the queue with this pixel */
    pixGetPixel(pixs, x, y, &val);
    el = colorelCreate(x, y, val);
    lqueueAdd(lq, el);
    pixSetPixel(pixv, x, y, 1);  /* visited */
    pta = ptaCreate(0);
    *ppta = pta;
    ptaAddPt(pta, x, y);

        /* Trace out the color component.  Each pixel on the queue has
         * a color.  Pop from the queue and for each of its 8 neighbors,
         * for those that have color:
         * - If the pixel has a similar color, add to the pta array for
         *   the component, using the color of its parent.
         * - Mark visited so that it will not be included in another
         *   component -- this effectively separates the growing component
         *   from all others. */
    pixGetDimensions(pixs, &w, &h, NULL);
    while (lqueueGetCount(lq) > 0) {
        el = (COLOREL *)lqueueRemove(lq);
        x = el->x;
        y = el->y;
        color = el->color;
        LEPT_FREE(el);
        pixGetVisitedNeighbors(pixv, x, y, visited);
        if (!visited[0]) {  /* check W */
            pixGetPixel(pixs, x - 1, y, &val);
            if (colorsAreSimilarForFill(color, val, maxdiff)) {
                el = colorelCreate(x - 1, y, color);
                lqueueAdd(lq, el);
                ptaAddPt(pta, x - 1, y);  /* added to component */
                pixSetPixel(pixv, x - 1, y, 1);  /* visited */
            }
        }
        if (!visited[1]) {  /* check N */
            pixGetPixel(pixs, x, y - 1, &val);
            if (colorsAreSimilarForFill(color, val, maxdiff)) {
                el = colorelCreate(x, y - 1, color);
                lqueueAdd(lq, el);
                ptaAddPt(pta, x, y - 1);
                pixSetPixel(pixv, x, y - 1, 1);
            }
        }
        if (!visited[2]) {  /* check E */
            pixGetPixel(pixs, x + 1, y, &val);
            if (colorsAreSimilarForFill(color, val, maxdiff)) {
                el = colorelCreate(x + 1, y, color);
                lqueueAdd(lq, el);
                ptaAddPt(pta, x + 1, y);
                pixSetPixel(pixv, x + 1, y, 1);
            }
        }
        if (!visited[3]) {  /* check S */
            pixGetPixel(pixs, x, y + 1, &val);
            if (colorsAreSimilarForFill(color, val, maxdiff)) {
                el = colorelCreate(x, y + 1, color);
                lqueueAdd(lq, el);
                ptaAddPt(pta, x, y + 1);
                pixSetPixel(pixv, x, y + 1, 1);
            }
        }
        if (!visited[4]) {  /* check NW */
            pixGetPixel(pixs, x - 1, y - 1, &val);
            if (colorsAreSimilarForFill(color, val, maxdiff)) {
                el = colorelCreate(x - 1, y - 1, color);
                lqueueAdd(lq, el);
                ptaAddPt(pta, x - 1, y - 1);
                pixSetPixel(pixv, x - 1, y - 1, 1);
            }
        }
        if (!visited[5]) {  /* check NE */
            pixGetPixel(pixs, x + 1, y - 1, &val);
            if (colorsAreSimilarForFill(color, val, maxdiff)) {
                el = colorelCreate(x + 1, y - 1, color);
                lqueueAdd(lq, el);
                ptaAddPt(pta, x + 1, y - 1);
                pixSetPixel(pixv, x + 1, y - 1, 1);
            }
        }
        if (!visited[6]) {  /* check SW */
            pixGetPixel(pixs, x - 1, y + 1, &val);
            if (colorsAreSimilarForFill(color, val, maxdiff)) {
                el = colorelCreate(x - 1, y + 1, color);
                lqueueAdd(lq, el);
                ptaAddPt(pta, x - 1, y + 1);
                pixSetPixel(pixv, x - 1, y + 1, 1);
            }
        }
        if (!visited[7]) {  /* check SE */
            pixGetPixel(pixs, x + 1, y + 1, &val);
            if (colorsAreSimilarForFill(color, val, maxdiff)) {
                el = colorelCreate(x + 1, y + 1, color);
                lqueueAdd(lq, el);
                ptaAddPt(pta, x + 1, y + 1);
                pixSetPixel(pixv, x + 1, y + 1, 1);
            }
        }
    }

        /* If there are not enough pixels, do not return the pta.
         * Otherwise, if a pta is returned, the caller will generate
         * a component and put it in the mask. */
    np = ptaGetCount(pta);
    if (np < minarea) {
        if (debug) lept_stderr("  Too small. End: x = %d, y = %d, np = %d\n",
                               x, y, np);
        ptaDestroy(ppta);
    } else {
        if (debug) lept_stderr("  Keep. End: x = %d, y = %d, np = %d\n",
                               x, y, np);
    }
}


/*!
 * \brief   pixGetVisitedNeighbors()
 *
 * \param[in]       pixs         1 bpp, representing visited locations
 * \param[in]       x            x coord of pixel
 * \param[in]       y            y coord of pixel
 * \param[in,out]   visited      array of 8 int
 * \return  void
 *
 * <pre>
 * Notes:
 *      (1) The values of the neighbors of (x,y) in pixs, given in
 *          order {W,N,E,S,NW,NE,SW,SE}, are returned in %visited.
 *          A "1" value means that pixel has been visited.  Initialize
 *          each neighbor to 1 (visited).
 *      (2) The input point (%x,%y) is never on the outer boundary of %pixs,
 *          (e.g., x >= 1, y >= 1), so no checking is required.
 * </pre>
 */
static void
pixGetVisitedNeighbors(PIX       *pixs,
                       l_int32    x,
                       l_int32    y,
                       l_uint32  *visited)
{
    pixGetPixel(pixs, x - 1, y, visited);  /* W */
    pixGetPixel(pixs, x, y - 1, visited + 1);  /* N */
    pixGetPixel(pixs, x + 1, y, visited + 2);  /* E */
    pixGetPixel(pixs, x, y + 1, visited + 3);  /* S */
    pixGetPixel(pixs, x - 1, y - 1, visited + 4);  /* NW */
    pixGetPixel(pixs, x + 1, y - 1, visited + 5);  /* NE */
    pixGetPixel(pixs, x - 1, y + 1, visited + 6);  /* SW */
    pixGetPixel(pixs, x + 1, y + 1, visited + 7);  /* SE */
}


/*!
 * \brief   findNextUnvisited()
 *
 * \param[in]       pixv      visited pixels
 * \param[in,out]   py        input start scanline; output y-coord of next seed
 * \param[out]      px        x-coord of next seed
 * \return  1 if new seed point is found; 0 if there are none
 *
 * <pre>
 * Notes:
 *      (1) Start the search at the beginning of the raster line
 *          for the pixel that ended the previous search.
 * </pre>
 */
static l_int32
findNextUnvisited(PIX      *pixv,
                  l_int32  *px,
                  l_int32  *py)
{
l_int32  ret;
PIX     *pix1;

    pix1 = pixCopy(NULL, pixv);
    pixInvert(pix1, pix1);   /* After inversion, ON pixels are unvisited */
    ret = nextOnPixelInRaster(pix1, 1, *py, px, py);
    pixDestroy(&pix1);
    return ret;
}


/*!
 * \brief   colorsAreSimilarForFill()
 *
 * \param[in]       val1      color of pixel 1, as 0xrrggbb00
 * \param[in]       val2      color of pixel 2, as 0xrrggbb00
 * \param[in]       maxdiff   max of difference function to be similar
 * \return  1 if val1 and val2 are similar; 0 otherwise
 *
 * <pre>
 * Notes:
 *      (1) An example will explain the approach.  Suppose we have:
 *            val1 = {100, 130, 70}
 *            val2 = {90, 135, 62}
 *          First find that red is the color with largest abs(difference):
 *            rdiff = val1 - val2 = 10
 *          Find the green and blue differences
 *            gdiff = 130 - 135 = -5
 *            bdiff = 70 - 62 = 8
 *          and subtract each from rdiff:
 *            rdiff - gdiff = 15
 *            rdiff - bdiff = 7
 *          The max of these is 15, which is then compared with %maxdiff
 * </pre>
 */
static l_int32
colorsAreSimilarForFill(l_uint32 val1,
                        l_uint32 val2,
                        l_int32  maxdiff)
{
l_int32  rdiff, gdiff, bdiff, maxindex, del1, del2, del3, maxdel;
l_int32  v1[3], v2[3];

    extractRGBValues(val1, v1, v1 + 1, v1 + 2);
    extractRGBValues(val2, v2, v2 + 1, v2 + 2);
    rdiff = v1[0] - v2[0];
    gdiff = v1[1] - v2[1];
    bdiff = v1[2] - v2[2];
    maxindex = 0;
    if (L_ABS(gdiff) > L_ABS(rdiff))
        maxindex = 1;
    if (L_ABS(bdiff) > L_ABS(rdiff) && L_ABS(bdiff) > L_ABS(gdiff))
        maxindex = 2;
    del1 = v1[maxindex] -  v2[maxindex];
    del2 = v1[(maxindex + 1) % 3] -  v2[(maxindex + 1) % 3];
    del3 = v1[(maxindex + 2) % 3] -  v2[(maxindex + 2) % 3];
    maxdel = L_MAX(L_ABS(del1 - del2), L_ABS(del1 - del3));
    return (maxdel <= maxdiff) ? 1 : 0;
}


/*!
 * \brief   pixelColorIsValid()
 *
 * \param[in]       val       color, as 0xrrggbb00
 * \param[in]       minmax    max component must be < %minmax to be valid
 * \return  0 if max component < %minmax; 1 otherwise
 */
static l_int32
pixelColorIsValid(l_uint32  val,
                  l_int32   minmax)
{
l_int32  rval, gval, bval;

    extractRGBValues(val, &rval, &gval, &bval);
    if (rval < minmax && gval < minmax && bval < minmax)
        return 0;  /* maximum component is less than threshold */
    else
        return 1;
}


/*!
 * \brief   pixelIsOnColorBoundary()
 *
 * \param[in]       pixs      32 bpp rgb or 8 bpp with colormap
 * \param[in]       x, y      location of pixel of interest
 * \return  1 if at least one neighboring pixel had a different color;
 *          0 otherwise.
 */
static l_int32
pixelIsOnColorBoundary(PIX     *pixs,
                       l_int32  x,
                       l_int32  y)
{
l_int32   w, h;
l_uint32  val, neigh;

    pixGetDimensions(pixs, &w, &h, NULL);
    pixGetPixel(pixs, x, y, &val);
    if (x > 0) {
        pixGetPixel(pixs, x - 1, y, &neigh);  /* W */
        if (neigh != val) return TRUE;
    }
    if (x < w - 1) {
        pixGetPixel(pixs, x + 1, y, &neigh);  /* E */
        if (neigh != val) return TRUE;
    }
    if (y > 0) {
        pixGetPixel(pixs, x, y - 1, &neigh);  /* N */
        if (neigh != val) return TRUE;
    }
    if (y < h - 1) {
        pixGetPixel(pixs, x, y + 1, &neigh);  /* S */
        if (neigh != val) return TRUE;
    }
    return FALSE;
}


/*!
 * \brief   evalColorfillData()
 *
 * \param[in]       cf       colorfill with masks generated for all tiles
 * \param[in]       debug    show segmented regions with their median color
 * \return  0 if OK, 1 on error
 */
static l_int32
evalColorfillData(L_COLORFILL  *cf,
                  l_int32       debug)
{
l_int32    i, j, n, nc, w, h, x, y, count;
l_float32  rval, gval, bval;
l_uint32   pixel;
l_int32   *tab;
BOX       *box1;
BOXA      *boxa1;
L_DNA     *da;
NUMA      *na;
PIX       *pixm, *pix1, *pix2, *pixdb;
PIXA      *pixa1;

    if (!cf)
        return ERROR_INT("cf not defind", __func__, 1);

    tab = makePixelSumTab8();
    n = cf->nx * cf->ny;
    for (i = 0; i < n; i++) {
        pix1 = pixaGetPix(cf->pixas, i, L_CLONE);
        pixm = pixaGetPix(cf->pixam, i, L_CLONE);
        pixGetDimensions(pix1, &w, &h, NULL);
        boxa1 = pixConnComp(pixm, &pixa1, 4);
        boxaDestroy(&boxa1);
        nc = pixaGetCount(pixa1);
        na = numaCreate(0);
        da = l_dnaCreate(0);
        pixdb = (debug) ? pixCreate(w, h, 32) : NULL;
        for (j = 0; j < nc; j++) {
            pix2 = pixaGetPix(pixa1, j, L_COPY);
            box1 = pixaGetBox(pixa1, j, L_COPY);
            boxGetGeometry(box1, &x, &y, NULL, NULL);
            pixGetRankValueMaskedRGB(pix1, pix2, x, y, 1, 0.5,
                                     &rval, &gval, &bval);
            composeRGBPixel(rval, gval, bval, &pixel);

            l_dnaAddNumber(da, pixel);
            pixCountPixels(pix2, &count, tab);
            numaAddNumber(na, count);
            if (debug)
                pixPaintThroughMask(pixdb, pix2, x, y, pixel);
            boxDestroy(&box1);
            pixDestroy(&pix2);
        }
        pixaAddPix(cf->pixadb, pixdb, L_INSERT);
        numaaAddNuma(cf->naa, na, L_INSERT);
        l_dnaaAddDna(cf->dnaa, da, L_INSERT);
        pixDestroy(&pix1);
        pixDestroy(&pixm);
        pixaDestroy(&pixa1);
    }

    if (debug) {  /* first tile */
        na = numaaGetNuma(cf->naa, 0, L_CLONE);
        lept_stderr("Size of components in tile 0:");
        numaWriteStderr(na);
        numaDestroy(&na);
    }
    LEPT_FREE(tab);
    return 0;
}
