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
 * \file watershed.c
 * <pre>
 *
 *      Top-level
 *            L_WSHED         *wshedCreate()
 *            void             wshedDestroy()
 *            l_int32          wshedApply()
 *
 *      Helpers
 *            static l_int32   identifyWatershedBasin()
 *            static l_int32   mergeLookup()
 *            static l_int32   wshedGetHeight()
 *            static void      pushNewPixel()
 *            static void      popNewPixel()
 *            static void      pushWSPixel()
 *            static void      popWSPixel()
 *            static void      debugPrintLUT()
 *            static void      debugWshedMerge()
 *
 *      Output
 *            l_int32          wshedBasins()
 *            PIX             *wshedRenderFill()
 *            PIX             *wshedRenderColors()
 *
 *  The watershed function identifies the "catch basins" of the input
 *  8 bpp image, with respect to the specified seeds or "markers".
 *  The use is in segmentation, but the selection of the markers is
 *  critical to getting meaningful results.
 *
 *  How are the markers selected?  You can't simply use the local
 *  minima, because a typical image has sufficient noise so that
 *  a useful catch basin can easily have multiple local minima.  However
 *  they are selected, the question for the watershed function is
 *  how to handle local minima that are not markers.  The reason
 *  this is important is because of the algorithm used to find the
 *  watersheds, which is roughly like this:
 *
 *    (1) Identify the markers and the local minima, and enter them
 *        into a priority queue based on the pixel value.  Each marker
 *        is shrunk to a single pixel, if necessary, before the
 *        operation starts.
 *    (2) Feed the priority queue with neighbors of pixels that are
 *        popped off the queue.  Each of these queue pixels is labeled
 *        with the index value of its parent.
 *    (3) Each pixel is also labeled, in a 32-bit image, with the marker
 *        or local minimum index, from which it was originally derived.
 *    (4) There are actually 3 classes of labels: seeds, minima, and
 *        fillers.  The fillers are labels of regions that have already
 *        been identified as watersheds and are continuing to fill, for
 *        the purpose of finding higher watersheds.
 *    (5) When a pixel is popped that has already been labeled in the
 *        32-bit image and that label differs from the label of its
 *        parent (stored in the queue pixel), a boundary has been crossed.
 *        There are several cases:
 *         (a) Both parents are derived from markers but at least one
 *             is not deep enough to become a watershed.  Absorb the
 *             shallower basin into the deeper one, fixing the LUT to
 *             redirect the shallower index to the deeper one.
 *         (b) Both parents are derived from markers and both are deep
 *             enough.  Identify and save the watershed for each marker.
 *         (c) One parent was derived from a marker and the other from
 *             a minima: absorb the minima basin into the marker basin.
 *         (d) One parent was derived from a marker and the other is
 *             a filler: identify and save the watershed for the marker.
 *         (e) Both parents are derived from minima: merge them.
 *         (f) One parent is a filler and the other is derived from a
 *             minima: merge the minima into the filler.
 *    (6) The output of the watershed operation consists of:
 *         ~ a pixa of the basins
 *         ~ a pta of the markers
 *         ~ a numa of the watershed levels
 *
 *  Typical usage:
 *      L_WShed *wshed = wshedCreate(pixs, pixseed, mindepth, 0);
 *      wshedApply(wshed);
 *
 *      wshedBasins(wshed, &pixa, &nalevels);
 *        ... do something with pixa, nalevels ...
 *      pixaDestroy(&pixa);
 *      numaDestroy(&nalevels);
 *
 *      Pix *pixd = wshedRenderFill(wshed);
 *
 *      wshedDestroy(&wshed);
 * </pre>
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

#ifndef  NO_CONSOLE_IO
#define   DEBUG_WATERSHED     0
#endif  /* ~NO_CONSOLE_IO */

static const l_uint32  MAX_LABEL_VALUE = 0x7fffffff;  /* largest l_int32 */

/*! New pixel coordinates */
struct L_NewPixel
{
    l_int32    x;      /*!< x coordinate */
    l_int32    y;      /*!< y coordinate */
};
typedef struct L_NewPixel  L_NEWPIXEL;

/*! Wartshed pixel */
struct L_WSPixel
{
    l_float32  val;    /*!< pixel value */
    l_int32    x;      /*!< x coordinate */
    l_int32    y;      /*!< y coordinate */
    l_int32    index;  /*!< label for set to which pixel belongs */
};
typedef struct L_WSPixel  L_WSPIXEL;


    /* Static functions for obtaining bitmap of watersheds  */
static void wshedSaveBasin(L_WSHED *wshed, l_int32 index, l_int32 level);

static l_int32 identifyWatershedBasin(L_WSHED *wshed,
                                      l_int32 index, l_int32 level,
                                      BOX **pbox, PIX **ppixd);

    /* Static function for merging lut and backlink arrays */
static l_int32 mergeLookup(L_WSHED *wshed, l_int32 sindex, l_int32 dindex);

    /* Static function for finding the height of the current pixel
       above its seed or minima in the watershed.  */
static l_int32 wshedGetHeight(L_WSHED *wshed, l_int32 val, l_int32 label,
                              l_int32 *pheight);

    /* Static accessors for NewPixel on a queue */
static void pushNewPixel(L_QUEUE *lq, l_int32 x, l_int32 y,
                         l_int32 *pminx, l_int32 *pmaxx,
                         l_int32 *pminy, l_int32 *pmaxy);
static void popNewPixel(L_QUEUE *lq, l_int32 *px, l_int32 *py);

    /* Static accessors for WSPixel on a heap */
static void pushWSPixel(L_HEAP *lh, L_STACK *stack, l_int32 val,
                        l_int32 x, l_int32 y, l_int32 index);
static void popWSPixel(L_HEAP *lh, L_STACK *stack, l_int32 *pval,
                       l_int32 *px, l_int32 *py, l_int32 *pindex);

    /* Static debug print output */
static void debugPrintLUT(l_int32 *lut, l_int32 size, l_int32 debug);

static void debugWshedMerge(L_WSHED *wshed, char *descr, l_int32 x,
                            l_int32 y, l_int32 label, l_int32 index);


/*-----------------------------------------------------------------------*
 *                        Top-level watershed                            *
 *-----------------------------------------------------------------------*/
/*!
 * \brief   wshedCreate()
 *
 * \param[in]    pixs  8 bpp source
 * \param[in]    pixm  1 bpp 'marker' seed
 * \param[in]    mindepth minimum depth; anything less is not saved
 * \param[in]    debugflag 1 for debug output
 * \return  WShed, or NULL on error
 *
 * <pre>
 * Notes:
 *      (1) It is not necessary for the fg pixels in the seed image
 *          be at minima, or that they be isolated.  We extract a
 *          single pixel from each connected component, and a seed
 *          anywhere in a watershed will eventually label the watershed
 *          when the filling level reaches it.
 *      (2) Set mindepth to some value to ignore noise in pixs that
 *          can create small local minima.  Any watershed shallower
 *          than mindepth, even if it has a seed, will not be saved;
 *          It will either be incorporated in another watershed or
 *          eliminated.
 * </pre>
 */
L_WSHED *
wshedCreate(PIX     *pixs,
            PIX     *pixm,
            l_int32  mindepth,
            l_int32  debugflag)
{
l_int32   w, h;
L_WSHED  *wshed;

    if (!pixs)
        return (L_WSHED *)ERROR_PTR("pixs is not defined", __func__, NULL);
    if (pixGetDepth(pixs) != 8)
        return (L_WSHED *)ERROR_PTR("pixs is not 8 bpp", __func__, NULL);
    if (!pixm)
        return (L_WSHED *)ERROR_PTR("pixm is not defined", __func__, NULL);
    if (pixGetDepth(pixm) != 1)
        return (L_WSHED *)ERROR_PTR("pixm is not 1 bpp", __func__, NULL);
    pixGetDimensions(pixs, &w, &h, NULL);
    if (pixGetWidth(pixm) != w || pixGetHeight(pixm) != h)
        return (L_WSHED *)ERROR_PTR("pixs/m sizes are unequal", __func__, NULL);

    if ((wshed = (L_WSHED *)LEPT_CALLOC(1, sizeof(L_WSHED))) == NULL)
        return (L_WSHED *)ERROR_PTR("wshed not made", __func__, NULL);

    wshed->pixs = pixClone(pixs);
    wshed->pixm = pixClone(pixm);
    wshed->mindepth = L_MAX(1, mindepth);
    wshed->pixlab = pixCreate(w, h, 32);
    pixSetAllArbitrary(wshed->pixlab, MAX_LABEL_VALUE);
    wshed->pixt = pixCreate(w, h, 1);
    wshed->lines8 = pixGetLinePtrs(pixs, NULL);
    wshed->linem1 = pixGetLinePtrs(pixm, NULL);
    wshed->linelab32 = pixGetLinePtrs(wshed->pixlab, NULL);
    wshed->linet1 = pixGetLinePtrs(wshed->pixt, NULL);
    wshed->debug = debugflag;
    return wshed;
}


/*!
 * \brief   wshedDestroy()
 *
 * \param[in,out]   pwshed will be set to null before returning
 * \return  void
 */
void
wshedDestroy(L_WSHED  **pwshed)
{
l_int32   i;
L_WSHED  *wshed;

    if (pwshed == NULL) {
        L_WARNING("ptr address is null!\n", __func__);
        return;
    }

    if ((wshed = *pwshed) == NULL)
        return;

    pixDestroy(&wshed->pixs);
    pixDestroy(&wshed->pixm);
    pixDestroy(&wshed->pixlab);
    pixDestroy(&wshed->pixt);
    if (wshed->lines8) LEPT_FREE(wshed->lines8);
    if (wshed->linem1) LEPT_FREE(wshed->linem1);
    if (wshed->linelab32) LEPT_FREE(wshed->linelab32);
    if (wshed->linet1) LEPT_FREE(wshed->linet1);
    pixaDestroy(&wshed->pixad);
    ptaDestroy(&wshed->ptas);
    numaDestroy(&wshed->nash);
    numaDestroy(&wshed->nasi);
    numaDestroy(&wshed->namh);
    numaDestroy(&wshed->nalevels);
    if (wshed->lut)
         LEPT_FREE(wshed->lut);
    if (wshed->links) {
        for (i = 0; i < wshed->arraysize; i++)
            numaDestroy(&wshed->links[i]);
        LEPT_FREE(wshed->links);
    }
    LEPT_FREE(wshed);
    *pwshed = NULL;
}


/*!
 * \brief   wshedApply()
 *
 * \param[in]    wshed generated from wshedCreate()
 * \return  0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *      (1) N.B. This is buggy!  It seems to locate watersheds that are
 *          duplicates.  The watershed extraction after complete fill
 *          grabs some regions belonging to existing watersheds.
 *          See prog/watershedtest.c for testing.
 * </pre>
 */
l_ok
wshedApply(L_WSHED  *wshed)
{
char      two_new_watersheds[] = "Two new watersheds";
char      seed_absorbed_into_seeded_basin[] = "Seed absorbed into seeded basin";
char      one_new_watershed_label[] = "One new watershed (label)";
char      one_new_watershed_index[] = "One new watershed (index)";
char      minima_absorbed_into_seeded_basin[] =
                 "Minima absorbed into seeded basin";
char      minima_absorbed_by_filler_or_another[] =
                 "Minima absorbed by filler or another";
l_int32   nseeds, nother, nboth, arraysize;
l_int32   i, j, val, x, y, w, h, index, mindepth;
l_int32   imin, imax, jmin, jmax, cindex, clabel, nindex;
l_int32   hindex, hlabel, hmin, hmax, minhindex, maxhindex;
l_int32  *lut;
l_uint32  ulabel, uval;
void    **lines8, **linelab32;
NUMA     *nalut, *nalevels, *nash, *namh, *nasi;
NUMA    **links;
L_HEAP   *lh;
PIX      *pixmin, *pixsd;
PIXA     *pixad;
L_STACK  *rstack;
PTA      *ptas, *ptao;

    if (!wshed)
        return ERROR_INT("wshed not defined", __func__, 1);

    /* ------------------------------------------------------------ *
     *  Initialize priority queue and pixlab with seeds and minima  *
     * ------------------------------------------------------------ */

    lh = lheapCreate(0, L_SORT_INCREASING);  /* remove lowest values first */
    rstack = lstackCreate(0);  /* for reusing the WSPixels */
    pixGetDimensions(wshed->pixs, &w, &h, NULL);
    lines8 = wshed->lines8;  /* wshed owns this */
    linelab32 = wshed->linelab32;  /* ditto */

        /* Identify seed (marker) pixels, 1 for each c.c. in pixm */
    pixSelectMinInConnComp(wshed->pixs, wshed->pixm, &ptas, &nash);
    pixsd = pixGenerateFromPta(ptas, w, h);
    nseeds = ptaGetCount(ptas);
    for (i = 0; i < nseeds; i++) {
        ptaGetIPt(ptas, i, &x, &y);
        uval = GET_DATA_BYTE(lines8[y], x);
        pushWSPixel(lh, rstack, (l_int32)uval, x, y, i);
    }
    wshed->ptas = ptas;
    nasi = numaMakeConstant(1, nseeds);  /* indicator array */
    wshed->nasi = nasi;
    wshed->nash = nash;
    wshed->nseeds = nseeds;

        /* Identify minima that are not seeds.  Use these 4 steps:
         *  (1) Get the local minima, which can have components
         *      of arbitrary size.  This will be a clipping mask.
         *  (2) Get the image of the actual seeds (pixsd)
         *  (3) Remove all elements of the clipping mask that have a seed.
         *  (4) Shrink each of the remaining elements of the minima mask
         *      to a single pixel.  */
    pixLocalExtrema(wshed->pixs, 200, 0, &pixmin, NULL);
    pixRemoveSeededComponents(pixmin, pixsd, pixmin, 8, 2);
    pixSelectMinInConnComp(wshed->pixs, pixmin, &ptao, &namh);
    nother = ptaGetCount(ptao);
    for (i = 0; i < nother; i++) {
        ptaGetIPt(ptao, i, &x, &y);
        uval = GET_DATA_BYTE(lines8[y], x);
        pushWSPixel(lh, rstack, (l_int32)uval, x, y, nseeds + i);
    }
    wshed->namh = namh;

    /* ------------------------------------------------------------ *
     *                Initialize merging lookup tables              *
     * ------------------------------------------------------------ */

        /* nalut should always give the current after-merging index.
         * links are effectively backpointers: they are numas associated with
         * a dest index of all indices in nalut that point to that index. */
    mindepth = wshed->mindepth;
    nboth = nseeds + nother;
    arraysize = 2 * nboth;
    wshed->arraysize = arraysize;
    nalut = numaMakeSequence(0, 1, arraysize);
    lut = numaGetIArray(nalut);
    wshed->lut = lut;  /* wshed owns this */
    links = (NUMA **)LEPT_CALLOC(arraysize, sizeof(NUMA *));
    wshed->links = links;  /* wshed owns this */
    nindex = nseeds + nother;  /* the next unused index value */

    /* ------------------------------------------------------------ *
     *              Fill the basins, using the priority queue       *
     * ------------------------------------------------------------ */

    pixad = pixaCreate(nseeds);
    wshed->pixad = pixad;  /* wshed owns this */
    nalevels = numaCreate(nseeds);
    wshed->nalevels = nalevels;  /* wshed owns this */
    L_INFO("nseeds = %d, nother = %d\n", __func__, nseeds, nother);
    while (lheapGetCount(lh) > 0) {
        popWSPixel(lh, rstack, &val, &x, &y, &index);
/*        lept_stderr("x = %d, y = %d, index = %d\n", x, y, index); */
        ulabel = GET_DATA_FOUR_BYTES(linelab32[y], x);
        if (ulabel == MAX_LABEL_VALUE)
            clabel = ulabel;
        else
            clabel = lut[ulabel];
        cindex = lut[index];
        if (clabel == cindex) continue;  /* have already seen this one */
        if (clabel == MAX_LABEL_VALUE) {  /* new one; assign index and try to
                                           * propagate to all neighbors */
            SET_DATA_FOUR_BYTES(linelab32[y], x, cindex);
            imin = L_MAX(0, y - 1);
            imax = L_MIN(h - 1, y + 1);
            jmin = L_MAX(0, x - 1);
            jmax = L_MIN(w - 1, x + 1);
            for (i = imin; i <= imax; i++) {
                for (j = jmin; j <= jmax; j++) {
                    if (i == y && j == x) continue;
                    uval = GET_DATA_BYTE(lines8[i], j);
                    pushWSPixel(lh, rstack, (l_int32)uval, j, i, cindex);
                }
            }
        } else {  /* pixel is already labeled (differently); must resolve */

                /* If both indices are seeds, check if the min height is
                 * greater than mindepth.  If so, we have two new watersheds;
                 * locate them and assign to both regions a new index
                 * for further waterfill.  If not, absorb the shallower
                 * watershed into the deeper one and continue filling it. */
            pixGetPixel(pixsd, x, y, &uval);
            if (clabel < nseeds && cindex < nseeds) {
                wshedGetHeight(wshed, val, clabel, &hlabel);
                wshedGetHeight(wshed, val, cindex, &hindex);
                hmin = L_MIN(hlabel, hindex);
                hmax = L_MAX(hlabel, hindex);
                if (hmin == hmax) {
                    hmin = hlabel;
                    hmax = hindex;
                }
                if (wshed->debug) {
                    lept_stderr("clabel,hlabel = %d,%d\n", clabel, hlabel);
                    lept_stderr("hmin = %d, hmax = %d\n", hmin, hmax);
                    lept_stderr("cindex,hindex = %d,%d\n", cindex, hindex);
                    if (hmin < mindepth)
                        lept_stderr("Too shallow!\n");
                }

                if (hmin >= mindepth) {
                    debugWshedMerge(wshed, two_new_watersheds,
                                    x, y, clabel, cindex);
                    wshedSaveBasin(wshed, cindex, val - 1);
                    wshedSaveBasin(wshed, clabel, val - 1);
                    numaSetValue(nasi, cindex, 0);
                    numaSetValue(nasi, clabel, 0);

                    if (wshed->debug) lept_stderr("nindex = %d\n", nindex);
                    debugPrintLUT(lut, nindex, wshed->debug);
                    mergeLookup(wshed, clabel, nindex);
                    debugPrintLUT(lut, nindex, wshed->debug);
                    mergeLookup(wshed, cindex, nindex);
                    debugPrintLUT(lut, nindex, wshed->debug);
                    nindex++;
                } else  /* extraneous seed within seeded basin; absorb */ {
                    debugWshedMerge(wshed, seed_absorbed_into_seeded_basin,
                                    x, y, clabel, cindex);
                }
                maxhindex = clabel;  /* TODO: is this part of above 'else'? */
                minhindex = cindex;
                if (hindex > hlabel) {
                    maxhindex = cindex;
                    minhindex = clabel;
                }
                mergeLookup(wshed, minhindex, maxhindex);
            } else if (clabel < nseeds && cindex >= nboth) {
                /* If one index is a seed and the other is a merge of
                 * 2 watersheds, generate a single watershed. */
                debugWshedMerge(wshed, one_new_watershed_label,
                                x, y, clabel, cindex);
                wshedSaveBasin(wshed, clabel, val - 1);
                numaSetValue(nasi, clabel, 0);
                mergeLookup(wshed, clabel, cindex);
            } else if (cindex < nseeds && clabel >= nboth) {
                debugWshedMerge(wshed, one_new_watershed_index,
                                x, y, clabel, cindex);
                wshedSaveBasin(wshed, cindex, val - 1);
                numaSetValue(nasi, cindex, 0);
                mergeLookup(wshed, cindex, clabel);
            } else if (clabel < nseeds) {  /* cindex from minima; absorb */
                /* If one index is a seed and the other is from a minimum,
                 * merge the minimum wshed into the seed wshed. */
                debugWshedMerge(wshed, minima_absorbed_into_seeded_basin,
                                x, y, clabel, cindex);
                mergeLookup(wshed, cindex, clabel);
            } else if (cindex < nseeds) {  /* clabel from minima; absorb */
                debugWshedMerge(wshed, minima_absorbed_into_seeded_basin,
                                x, y, clabel, cindex);
                mergeLookup(wshed, clabel, cindex);
            } else {  /* If neither index is a seed, just merge */
                debugWshedMerge(wshed, minima_absorbed_by_filler_or_another,
                                x, y, clabel, cindex);
                mergeLookup(wshed, clabel, cindex);
            }
        }
    }

#if 0
        /*  Use the indicator array to save any watersheds that fill
         *  to the maximum value.  This seems to screw things up!  */
    for (i = 0; i < nseeds; i++) {
        numaGetIValue(nasi, i, &ival);
        if (ival == 1) {
            wshedSaveBasin(wshed, lut[i], val - 1);
            numaSetValue(nasi, i, 0);
        }
    }
#endif

    numaDestroy(&nalut);
    pixDestroy(&pixmin);
    pixDestroy(&pixsd);
    ptaDestroy(&ptao);
    lheapDestroy(&lh, TRUE);
    lstackDestroy(&rstack, TRUE);
    return 0;
}


/*-----------------------------------------------------------------------*
 *                               Helpers                                 *
 *-----------------------------------------------------------------------*/
/*!
 * \brief   wshedSaveBasin()
 *
 * \param[in]    wshed
 * \param[in]    index   index of basin to be located
 * \param[in]    level   filling level reached at the time this function
 *                       is called
 * \return  0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *      (1) This identifies a single watershed.  It does not change
 *          the LUT, which must be done subsequently.
 *      (2) The fill level of a basin is taken to be %level - 1.
 * </pre>
 */
static void
wshedSaveBasin(L_WSHED  *wshed,
               l_int32   index,
               l_int32   level)
{
BOX  *box;
PIX  *pix;

    if (!wshed) {
        L_ERROR("wshed not defined\n", __func__);
        return;
    }

    if (identifyWatershedBasin(wshed, index, level, &box, &pix) == 0) {
        pixaAddPix(wshed->pixad, pix, L_INSERT);
        pixaAddBox(wshed->pixad, box, L_INSERT);
        numaAddNumber(wshed->nalevels, level - 1);
    }
}


/*!
 * \brief   identifyWatershedBasin()
 *
 * \param[in]    wshed
 * \param[in]    index   index of basin to be located
 * \param[in]    level   of basin at point at which the two basins met
 * \param[out]   pbox    bounding box of basin
 * \param[out]   ppixd   pix of basin, cropped to its bounding box
 * \return  0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *      (1) This is a static function, so we assume pixlab, pixs and pixt
 *          exist and are the same size.
 *      (2) It selects all pixels that have the label %index in pixlab
 *          and that have a value in pixs that is less than %level.
 *      (3) It is used whenever two seeded basins meet (typically at a saddle),
 *          or when one seeded basin meets a 'filler'.  All identified
 *          basins are saved as a watershed.
 * </pre>
 */
static l_int32
identifyWatershedBasin(L_WSHED  *wshed,
                       l_int32   index,
                       l_int32   level,
                       BOX     **pbox,
                       PIX     **ppixd)
{
l_int32   imin, imax, jmin, jmax, minx, miny, maxx, maxy;
l_int32   bw, bh, i, j, w, h, x, y;
l_int32  *lut;
l_uint32  label, bval, lval;
void    **lines8, **linelab32, **linet1;
BOX      *box;
PIX      *pixs, *pixt, *pixd;
L_QUEUE  *lq;

    if (!pbox)
        return ERROR_INT("&box not defined", __func__, 1);
    *pbox = NULL;
    if (!ppixd)
        return ERROR_INT("&pixd not defined", __func__, 1);
    *ppixd = NULL;
    if (!wshed)
        return ERROR_INT("wshed not defined", __func__, 1);

        /* Make a queue and an auxiliary stack */
    lq = lqueueCreate(0);
    lq->stack = lstackCreate(0);

    pixs = wshed->pixs;
    pixt = wshed->pixt;
    lines8 = wshed->lines8;
    linelab32 = wshed->linelab32;
    linet1 = wshed->linet1;
    lut = wshed->lut;
    pixGetDimensions(pixs, &w, &h, NULL);

        /* Prime the queue with the seed pixel for this watershed. */
    minx = miny = 1000000;
    maxx = maxy = 0;
    ptaGetIPt(wshed->ptas, index, &x, &y);
    pixSetPixel(pixt, x, y, 1);
    pushNewPixel(lq, x, y, &minx, &maxx, &miny, &maxy);
    if (wshed->debug) lept_stderr("prime: (x,y) = (%d, %d)\n", x, y);

        /* Each pixel in a spreading breadth-first search is inspected.
         * It is accepted as part of this watershed, and pushed on
         * the search queue, if:
         *     (1) It has a label value equal to %index
         *     (2) The pixel value is less than %level, the overflow
         *         height at which the two basins join.
         *     (3) It has not yet been seen in this search.  */
    while (lqueueGetCount(lq) > 0) {
        popNewPixel(lq, &x, &y);
        imin = L_MAX(0, y - 1);
        imax = L_MIN(h - 1, y + 1);
        jmin = L_MAX(0, x - 1);
        jmax = L_MIN(w - 1, x + 1);
        for (i = imin; i <= imax; i++) {
            for (j = jmin; j <= jmax; j++) {
                if (j == x && i == y) continue;  /* parent */
                label = GET_DATA_FOUR_BYTES(linelab32[i], j);
                if (label == MAX_LABEL_VALUE || lut[label] != index) continue;
                bval = GET_DATA_BIT(linet1[i], j);
                if (bval == 1) continue;  /* already seen */
                lval = GET_DATA_BYTE(lines8[i], j);
                if (lval >= level) continue;  /* too high */
                SET_DATA_BIT(linet1[i], j);
                pushNewPixel(lq, j, i, &minx, &maxx, &miny, &maxy);
            }
        }
    }

        /* Extract the box and pix, and clear pixt */
    bw = maxx - minx + 1;
    bh = maxy - miny + 1;
    box = boxCreate(minx, miny, bw, bh);
    pixd = pixClipRectangle(pixt, box, NULL);
    pixRasterop(pixt, minx, miny, bw, bh, PIX_SRC ^ PIX_DST, pixd, 0, 0);
    *pbox = box;
    *ppixd = pixd;

    lqueueDestroy(&lq, 1);
    return 0;
}


/*!
 * \brief   mergeLookup()
 *
 * \param[in]    wshed
 * \param[in]    sindex   primary index being changed in the merge
 * \param[in]    dindex   index that %sindex will point to after the merge
 * \return  0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *      (1) The links are a sparse array of Numas showing current back-links.
 *          The lut gives the current index (of the seed or the minima
 *          for the wshed  in which it is located.
 *      (2) Think of each entry in the lut.  There are two types:
 *             owner:     lut[index] = index
 *             redirect:  lut[index] != index
 *      (3) This is called each time a merge occurs.  It puts the lut
 *          and backlinks in a canonical form after the merge, where
 *          all entries in the lut point to the current "owner", which
 *          has all backlinks.  That is, every "redirect" in the lut
 *          points to an "owner".  The lut always gives the index of
 *          the current owner.
 * </pre>
 */
static l_int32
mergeLookup(L_WSHED  *wshed,
            l_int32   sindex,
            l_int32   dindex)
{
l_int32   i, n, size, index;
l_int32  *lut;
NUMA     *na;
NUMA    **links;

    if (!wshed)
        return ERROR_INT("wshed not defined", __func__, 1);
    size = wshed->arraysize;
    if (sindex < 0 || sindex >= size)
        return ERROR_INT("invalid sindex", __func__, 1);
    if (dindex < 0 || dindex >= size)
        return ERROR_INT("invalid dindex", __func__, 1);

        /* Redirect links in the lut */
    n = 0;
    links = wshed->links;
    lut = wshed->lut;
    if ((na = links[sindex]) != NULL) {
        n = numaGetCount(na);
        for (i = 0; i < n; i++) {
            numaGetIValue(na, i, &index);
            lut[index] = dindex;
        }
    }
    lut[sindex] = dindex;

        /* Shift the backlink arrays from sindex to dindex.
         * sindex should have no backlinks because all entries in the
         * lut that were previously pointing to it have been redirected
         * to dindex. */
    if (!links[dindex])
        links[dindex] = numaCreate(n);
    numaJoin(links[dindex], links[sindex], 0, -1);
    numaAddNumber(links[dindex], sindex);
    numaDestroy(&links[sindex]);

    return 0;
}


/*!
 * \brief   wshedGetHeight()
 *
 * \param[in]    wshed     array of current indices
 * \param[in]    val       value of current pixel popped off queue
 * \param[in]    label     of pixel or 32 bpp label image
 * \param[out]   pheight   height of current value from seed
 *                         or minimum of watershed
 * \return  0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *      (1) It is only necessary to find the height for a watershed
 *          that is indexed by a seed or a minima.  This function should
 *          not be called on a finished watershed (that continues to fill).
 * </pre>
 */
static l_int32
wshedGetHeight(L_WSHED  *wshed,
               l_int32   val,
               l_int32   label,
               l_int32  *pheight)
{
l_int32  minval;

    if (!pheight)
        return ERROR_INT("&height not defined", __func__, 1);
    *pheight = 0;
    if (!wshed)
        return ERROR_INT("wshed not defined", __func__, 1);

    if (label < wshed->nseeds)
        numaGetIValue(wshed->nash, label, &minval);
    else if (label < wshed->nseeds + wshed->nother)
        numaGetIValue(wshed->namh, label, &minval);
    else
        return ERROR_INT("finished watershed; should not call", __func__, 1);

    *pheight = val - minval;
    return 0;
}


/*
 * \brief   pushNewPixel()
 *
 * \param[in]     lqueue
 * \param[in]     x, y                          pixel coordinates
 * \param[out]    pminx, pmaxx, pminy, pmaxy    bounding box update
 * \return   void
 *
 * <pre>
 * Notes:
 *      (1) This is a wrapper for adding a NewPixel to a queue, which
 *          updates the bounding box for all pixels on that queue and
 *          uses the storage stack to retrieve a NewPixel.
 * </pre>
 */
static void
pushNewPixel(L_QUEUE  *lq,
             l_int32   x,
             l_int32   y,
             l_int32  *pminx,
             l_int32  *pmaxx,
             l_int32  *pminy,
             l_int32  *pmaxy)
{
L_NEWPIXEL  *np;

    if (!lq) {
        L_ERROR("queue not defined\n", __func__);
        return;
    }

        /* Adjust bounding box */
    *pminx = L_MIN(*pminx, x);
    *pmaxx = L_MAX(*pmaxx, x);
    *pminy = L_MIN(*pminy, y);
    *pmaxy = L_MAX(*pmaxy, y);

        /* Get a newpixel to use */
    if (lstackGetCount(lq->stack) > 0)
        np = (L_NEWPIXEL *)lstackRemove(lq->stack);
    else
        np = (L_NEWPIXEL *)LEPT_CALLOC(1, sizeof(L_NEWPIXEL));

    np->x = x;
    np->y = y;
    lqueueAdd(lq, np);
}


/*
 * \brief   popNewPixel()
 *
 * \param[in]    lqueue
 * \param[out]   px, py    pixel coordinates
 * \return   void
 *
 * <pre>
 * Notes:
 *      (1) This is a wrapper for removing a NewPixel from a queue,
 *          which returns the pixel coordinates and saves the NewPixel
 *          on the storage stack.
 * </pre>
 */
static void
popNewPixel(L_QUEUE  *lq,
            l_int32  *px,
            l_int32  *py)
{
L_NEWPIXEL  *np;

    if (!lq) {
        L_ERROR("lqueue not defined\n", __func__);
        return;
    }

    if ((np = (L_NEWPIXEL *)lqueueRemove(lq)) == NULL)
        return;
    *px = np->x;
    *py = np->y;
    lstackAdd(lq->stack, np);  /* save for re-use */
}


/*
 * \brief   pushWSPixel()
 *
 * \param[in]    lh       priority queue
 * \param[in]    stack    of reusable WSPixels
 * \param[in]    val      pixel value: used for ordering the heap
 * \param[in]    x, y     pixel coordinates
 * \param[in]    index    label for set to which pixel belongs
 * \return    void
 *
 * <pre>
 * Notes:
 *      (1) This is a wrapper for adding a WSPixel to a heap.  It
 *          uses the storage stack to retrieve a WSPixel.
 * </pre>
 */
static void
pushWSPixel(L_HEAP   *lh,
            L_STACK  *stack,
            l_int32   val,
            l_int32   x,
            l_int32   y,
            l_int32   index)
{
L_WSPIXEL  *wsp;

    if (!lh) {
        L_ERROR("heap not defined\n", __func__);
        return;
    }
    if (!stack) {
        L_ERROR("stack not defined\n", __func__);
        return;
    }

        /* Get a wspixel to use */
    if (lstackGetCount(stack) > 0)
        wsp = (L_WSPIXEL *)lstackRemove(stack);
    else
        wsp = (L_WSPIXEL *)LEPT_CALLOC(1, sizeof(L_WSPIXEL));

    wsp->val = (l_float32)val;
    wsp->x = x;
    wsp->y = y;
    wsp->index = index;
    lheapAdd(lh, wsp);
}


/*
 * \brief  popWSPixel()
 *
 * \param[in]     lh        priority queue
 * \param[in]     stack     of reusable WSPixels
 * \param[out]    pval      pixel value
 * \param[out]    px, py    pixel coordinates
 * \param[out]    pindex    label for set to which pixel belongs
 * \return   void
 *
 * <pre>
 * Notes:
 *      (1) This is a wrapper for removing a WSPixel from a heap,
 *          which returns the WSPixel data and saves the WSPixel
 *          on the storage stack.
 * </pre>
 */
static void
popWSPixel(L_HEAP   *lh,
           L_STACK  *stack,
           l_int32  *pval,
           l_int32  *px,
           l_int32  *py,
           l_int32  *pindex)
{
L_WSPIXEL  *wsp;

    if (!lh) {
        L_ERROR("lheap not defined\n", __func__);
        return;
    }
    if (!stack) {
        L_ERROR("stack not defined\n", __func__);
        return;
    }
    if (!pval || !px || !py || !pindex) {
        L_ERROR("data can't be returned\n", __func__);
        return;
    }

    if ((wsp = (L_WSPIXEL *)lheapRemove(lh)) == NULL)
        return;
    *pval = (l_int32)wsp->val;
    *px = wsp->x;
    *py = wsp->y;
    *pindex = wsp->index;
    lstackAdd(stack, wsp);  /* save for re-use */
}


static void
debugPrintLUT(l_int32  *lut,
              l_int32   size,
              l_int32   debug)
{
l_int32  i;

    if (!debug) return;
    lept_stderr("lut: ");
    for (i = 0; i < size; i++)
        lept_stderr( "%d ", lut[i]);
    lept_stderr("\n");
}


static void
debugWshedMerge(L_WSHED *wshed,
                char    *descr,
                l_int32  x,
                l_int32  y,
                l_int32  label,
                l_int32  index)
{
    if (!wshed || (wshed->debug == 0))
         return;
    lept_stderr("%s:\n", descr);
    lept_stderr("   (x, y) = (%d, %d)\n", x, y);
    lept_stderr("   clabel = %d, cindex = %d\n", label, index);
}


/*-----------------------------------------------------------------------*
 *                                 Output                                *
 *-----------------------------------------------------------------------*/
/*!
 * \brief   wshedBasins()
 *
 * \param[in]    wshed
 * \param[out]   ppixa       [optional] mask of watershed basins
 * \param[out]   pnalevels   [optional] watershed levels
 * \return  0 if OK, 1 on error
 */
l_ok
wshedBasins(L_WSHED  *wshed,
            PIXA    **ppixa,
            NUMA    **pnalevels)
{
    if (!wshed)
        return ERROR_INT("wshed not defined", __func__, 1);

    if (ppixa)
        *ppixa = pixaCopy(wshed->pixad, L_CLONE);
    if (pnalevels)
        *pnalevels = numaClone(wshed->nalevels);
    return 0;
}


/*!
 * \brief   wshedRenderFill()
 *
 * \param[in]    wshed
 * \return  pixd   initial image with all basins filled, or NULL on error
 */
PIX *
wshedRenderFill(L_WSHED  *wshed)
{
l_int32  i, n, level, bx, by;
NUMA    *na;
PIX     *pix, *pixd;
PIXA    *pixa;

    if (!wshed)
        return (PIX *)ERROR_PTR("wshed not defined", __func__, NULL);

    wshedBasins(wshed, &pixa, &na);
    pixd = pixCopy(NULL, wshed->pixs);
    n = pixaGetCount(pixa);
    for (i = 0; i < n; i++) {
        pix = pixaGetPix(pixa, i, L_CLONE);
        pixaGetBoxGeometry(pixa, i, &bx, &by, NULL, NULL);
        numaGetIValue(na, i, &level);
        pixPaintThroughMask(pixd, pix, bx, by, level);
        pixDestroy(&pix);
    }

    pixaDestroy(&pixa);
    numaDestroy(&na);
    return pixd;
}


/*!
 * \brief   wshedRenderColors()
 *
 * \param[in]    wshed
 * \return  pixd   initial image with all basins filled, or null on error
 */
PIX *
wshedRenderColors(L_WSHED  *wshed)
{
l_int32  w, h;
PIX     *pixg, *pixt, *pixc, *pixm, *pixd;
PIXA    *pixa;

    if (!wshed)
        return (PIX *)ERROR_PTR("wshed not defined", __func__, NULL);

    wshedBasins(wshed, &pixa, NULL);
    pixg = pixCopy(NULL, wshed->pixs);
    pixGetDimensions(wshed->pixs, &w, &h, NULL);
    pixd = pixConvertTo32(pixg);
    pixt = pixaDisplayRandomCmap(pixa, w, h);
    pixc = pixConvertTo32(pixt);
    pixm = pixaDisplay(pixa, w, h);
    pixCombineMasked(pixd, pixc, pixm);

    pixDestroy(&pixg);
    pixDestroy(&pixt);
    pixDestroy(&pixc);
    pixDestroy(&pixm);
    pixaDestroy(&pixa);
    return pixd;
}
