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
 * \file recogident.c
 * <pre>
 *
 *      Top-level identification
 *         l_int32             recogIdentifyMultiple()
 *
 *      Segmentation and noise removal
 *         l_int32             recogSplitIntoCharacters()
 *
 *      Greedy character splitting
 *         l_int32             recogCorrelationBestRow()
 *         l_int32             recogCorrelationBestChar()
 *         static l_int32      pixCorrelationBestShift()
 *
 *      Low-level identification of single characters
 *         l_int32             recogIdentifyPixa()
 *         l_int32             recogIdentifyPix()
 *         l_int32             recogSkipIdentify()
 *
 *      Operations for handling identification results
 *         static L_RCHA      *rchaCreate()
 *         void                rchaDestroy()
 *         static L_RCH       *rchCreate()
 *         void                rchDestroy()
 *         l_int32             rchaExtract()
 *         l_int32             rchExtract()
 *         static l_int32      transferRchToRcha()
 *
 *      Preprocessing and filtering
 *         l_int32             recogProcessToIdentify()
 *         static PIX         *recogPreSplittingFilter()
 *         static PIX         *recogSplittingFilter()
 *
 *      Postprocessing
 *         SARRAY             *recogExtractNumbers()
 *         PIX                *showExtractNumbers()
 *
 *      Static debug helper
 *         static void         l_showIndicatorSplitValues()
 *
 *  See recogbasic.c for examples of training a recognizer, which is
 *  required before it can be used for identification.
 *
 *  The character splitter repeatedly does a greedy correlation with each
 *  averaged unscaled template, at all pixel locations along the text to
 *  be identified.  The vertical alignment is between the template
 *  centroid and the (moving) windowed centroid, including a delta of
 *  1 pixel above and below.  The best match then removes part of the
 *  input image, leaving 1 or 2 pieces, which, after filtering,
 *  are put in a queue.  The process ends when the queue is empty.
 *  The filtering is based on the size and aspect ratio of the
 *  remaining pieces; the intent is to remove anything that is
 *  unlikely to be text, such as small pieces and line graphics.
 *
 *  After splitting, the selected segments are identified using
 *  the input parameters that were initially specified for the
 *  recognizer.  Unlike the splitter, which uses the averaged
 *  templates from the unscaled input, the recognizer can use
 *  either all training examples or averaged templates, and these
 *  can be either scaled or unscaled.  These choices are specified
 *  when the recognizer is constructed.
 * </pre>
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include <string.h>
#include "allheaders.h"
#include "array_internal.h"

    /* There are two methods for splitting characters: DID and greedy.
     * The default method is DID.  */
#define  SPLIT_WITH_DID   1

    /* Padding on pix1: added before correlations and removed from result */
static const l_int32    LeftRightPadding = 32;

    /* Parameters for filtering and sorting connected components in splitter */
static const l_float32  MinFillFactor = 0.10f;
static const l_int32  DefaultMinHeight = 15;  /* min unscaled height */
static const l_int32  MinOverlap1 = 6;  /* in pass 1 of boxaSort2d() */
static const l_int32  MinOverlap2 = 6;  /* in pass 2 of boxaSort2d() */
static const l_int32  MinHeightPass1 = 5;  /* min height to start pass 1 */


static l_int32 pixCorrelationBestShift(PIX *pix1, PIX *pix2, NUMA *nasum1,
                                       NUMA *namoment1, l_int32 area2,
                                       l_int32 ycent2, l_int32 maxyshift,
                                       l_int32 *tab8, l_int32 *pdelx,
                                       l_int32 *pdely, l_float32 *pscore,
                                       l_int32 debugflag );
static L_RCH *rchCreate(l_int32 index, l_float32 score, char *text,
                        l_int32 sample, l_int32 xloc, l_int32 yloc,
                        l_int32 width);
static L_RCHA *rchaCreate();
static l_int32 transferRchToRcha(L_RCH *rch, L_RCHA *rcha);
static PIX *recogPreSplittingFilter(L_RECOG *recog, PIX *pixs, l_int32 minh,
                                    l_float32 minaf, l_int32 debug);
static l_int32 recogSplittingFilter(L_RECOG *recog, PIX *pixs, l_int32 min,
                                    l_float32 minaf, l_int32 *premove,
                                    l_int32 debug);
static void l_showIndicatorSplitValues(NUMA *na1, NUMA *na2, NUMA *na3,
                                       NUMA *na4, NUMA *na5, NUMA *na6);

/*------------------------------------------------------------------------*
 *                             Identification
 *------------------------------------------------------------------------*/
/*!
 * \brief   recogIdentifyMultiple()
 *
 * \param[in]    recog       with training finished
 * \param[in]    pixs        containing typically a small number of characters
 * \param[in]    minh        remove shorter components; use 0 for default
 * \param[in]    skipsplit   1 to skip the splitting step
 * \param[out]   pboxa       [optional] locations of identified components
 * \param[out]   ppixa       [optional] images of identified components
 * \param[out]   ppixdb      [optional] debug pix: inputs and best fits
 * \param[in]    debugsplit  1 returns pix split debugging images
 * \return  0 if OK; 1 if nothing is found; 2 for other errors.
 *
 * <pre>
 * Notes:
 *      (1) This filters the input pixa and calls recogIdentifyPixa()
 *      (2) Splitting is relatively slow, because it tries to match all
 *          character templates to all locations.  This step can be skipped.
 *      (3) An attempt is made to order the (optionally) returned images
 *          and boxes in 2-dimensional sorted order.  These can then
 *          be used to aggregate identified characters into numbers or words.
 *          One typically wants the pixa, which contains a boxa of the
 *          extracted subimages.
 * </pre>
 */
l_ok
recogIdentifyMultiple(L_RECOG  *recog,
                      PIX      *pixs,
                      l_int32   minh,
                      l_int32   skipsplit,
                      BOXA    **pboxa,
                      PIXA    **ppixa,
                      PIX     **ppixdb,
                      l_int32   debugsplit)
{
l_int32  n;
BOXA    *boxa;
PIX     *pixb;
PIXA    *pixa;

    if (pboxa) *pboxa = NULL;
    if (ppixa) *ppixa = NULL;
    if (ppixdb) *ppixdb = NULL;
    if (!recog)
        return ERROR_INT("recog not defined", __func__, 2);
    if (!recog->train_done)
        return ERROR_INT("training not finished", __func__, 2);
    if (!pixs)
        return ERROR_INT("pixs not defined", __func__, 2);

        /* Binarize if necessary */
    if (pixGetDepth(pixs) > 1)
        pixb = pixConvertTo1(pixs, recog->threshold);
    else
        pixb = pixClone(pixs);

        /* Noise removal and splitting of touching characters */
    recogSplitIntoCharacters(recog, pixb, minh, skipsplit, &boxa, &pixa,
                             debugsplit);
    pixDestroy(&pixb);
    if (!pixa || (n = pixaGetCount(pixa)) == 0) {
        pixaDestroy(&pixa);
        boxaDestroy(&boxa);
        L_WARNING("nothing found\n", __func__);
        return 1;
    }

    recogIdentifyPixa(recog, pixa, ppixdb);
    if (pboxa)
        *pboxa = boxa;
    else
        boxaDestroy(&boxa);
    if (ppixa)
        *ppixa = pixa;
    else
        pixaDestroy(&pixa);
    return 0;
}


/*------------------------------------------------------------------------*
 *                     Segmentation and noise removal                     *
 *------------------------------------------------------------------------*/
/*!
 * \brief   recogSplitIntoCharacters()
 *
 * \param[in]    recog
 * \param[in]    pixs        1 bpp, contains only mostly deskewed text
 * \param[in]    minh        remove shorter components; use 0 for default
 * \param[in]    skipsplit   1 to skip the splitting step
 * \param[out]   pboxa       character bounding boxes
 * \param[out]   ppixa       character images
 * \param[in]    debug       1 for results written to pixadb_split
 * \return  0 if OK, 1 on error or if no components are returned
 *
 * <pre>
 * Notes:
 *      (1) This can be given an image that has an arbitrary number
 *          of text characters.  It optionally splits connected
 *          components based on document image decoding in recogDecode().
 *          The returned pixa includes the boxes from which the
 *          (possibly split) components are extracted.
 *      (2) After noise filtering, the resulting components are put in
 *          row-major (2D) order, and the smaller of overlapping
 *          components are removed if they satisfy conditions of
 *          relative size and fractional overlap.
 *      (3) Note that the splitting function uses unscaled templates
 *          and does not bother returning the class results and scores.
 *          These are more accurately found later using the scaled templates.
 * </pre>
 */
l_ok
recogSplitIntoCharacters(L_RECOG  *recog,
                         PIX      *pixs,
                         l_int32   minh,
                         l_int32   skipsplit,
                         BOXA    **pboxa,
                         PIXA    **ppixa,
                         l_int32   debug)
{
static l_int32  ind = 0;
char     buf[32];
l_int32  i, xoff, yoff, empty, maxw, bw, ncomp, scaling;
BOX     *box;
BOXA    *boxa1, *boxa2, *boxa3, *boxa4, *boxad;
BOXAA   *baa;
PIX     *pix, *pix1, *pix2, *pix3;
PIXA    *pixa;

    lept_mkdir("lept/recog");

    if (pboxa) *pboxa = NULL;
    if (ppixa) *ppixa = NULL;
    if (!pboxa || !ppixa)
        return ERROR_INT("&boxa and &pixa not defined", __func__, 1);
    if (!recog)
        return ERROR_INT("recog not defined", __func__, 1);
    if (!recog->train_done)
        return ERROR_INT("training not finished", __func__, 1);
    if (!pixs || pixGetDepth(pixs) != 1)
        return ERROR_INT("pixs not defined or not 1 bpp", __func__, 1);
    if (minh <= 0) minh = DefaultMinHeight;
    pixZero(pixs, &empty);
    if (empty) return 1;

        /* Small vertical close for consolidation.  Don't do a horizontal
         * closing, because it might join separate characters. */
    pix1 = pixMorphSequence(pixs, "c1.3", 0);

        /* Carefully filter out noise */
    pix2 = recogPreSplittingFilter(recog, pix1, minh, MinFillFactor, debug);
    pixDestroy(&pix1);

        /* Get the 8-connected components to be split/identified */
    boxa1 = pixConnComp(pix2, NULL, 8);
    pixDestroy(&pix2);
    ncomp = boxaGetCount(boxa1);
    if (ncomp == 0) {
        boxaDestroy(&boxa1);
        L_WARNING("all components removed\n", __func__);
        return 1;
    }

        /* Save everything and split the large components */
    boxa2 = boxaCreate(ncomp);
    maxw = recog->maxwidth_u + 5;
    scaling = (recog->scalew > 0 || recog->scaleh > 0) ? TRUE : FALSE;
    pixa = (debug) ? pixaCreate(ncomp) : NULL;
    for (i = 0; i < ncomp; i++) {
        box = boxaGetBox(boxa1, i, L_CLONE);
        boxGetGeometry(box, &xoff, &yoff, &bw, NULL);
            /* Treat as one character if it is small, if the images
             * have been scaled, or if splitting is not to be run. */
        if (bw <= maxw || scaling || skipsplit) {
            boxaAddBox(boxa2, box, L_INSERT);
        } else {
            pix = pixClipRectangle(pixs, box, NULL);
#if SPLIT_WITH_DID
            if (!debug) {
                boxa3 = recogDecode(recog, pix, 2, NULL);
            } else {
                boxa3 = recogDecode(recog, pix, 2, &pix2);
                pixaAddPix(pixa, pix2, L_INSERT);
            }
#else  /* use greedy splitting */
            recogCorrelationBestRow(recog, pix, &boxa3, NULL, NULL,
                                    NULL, debug);
            if (debug) {
                pix2 = pixConvertTo32(pix);
                pixRenderBoxaArb(pix2, boxa3, 2, 255, 0, 0);
                pixaAddPix(pixa, pix2, L_INSERT);
            }
#endif  /* SPLIT_WITH_DID */
            pixDestroy(&pix);
            boxDestroy(&box);
            if (!boxa3) {
                L_ERROR("boxa3 not found for component %d\n", __func__, i);
            } else {
                boxa4 = boxaTransform(boxa3, xoff, yoff, 1.0, 1.0);
                boxaJoin(boxa2, boxa4, 0, -1);
                boxaDestroy(&boxa3);
                boxaDestroy(&boxa4);
            }
        }
    }
    boxaDestroy(&boxa1);
    if (pixa) {  /* debug */
        pix3 = pixaDisplayTiledInColumns(pixa, 1, 1.0, 20, 2);
        snprintf(buf, sizeof(buf), "/tmp/lept/recog/decode-%d.png", ind++);
        pixWrite(buf, pix3, IFF_PNG);
        pixaDestroy(&pixa);
        pixDestroy(&pix3);
    }

        /* Do a 2D sort on the bounding boxes, and flatten the result to 1D.
         * For the 2D sort, to add a box to an existing boxa, we require
         * specified minimum vertical overlaps for the first two passes
         * of the 2D sort.  In pass 1, only components with sufficient
         * height can start a new boxa. */
    baa = boxaSort2d(boxa2, NULL, MinOverlap1, MinOverlap2, MinHeightPass1);
    boxa3 = boxaaFlattenToBoxa(baa, NULL, L_CLONE);
    boxaaDestroy(&baa);
    boxaDestroy(&boxa2);

        /* Remove smaller components of overlapping pairs.
         * We only remove the small component if the overlap is
         * at least half its area and if its area is no more
         * than 30% of the area of the large component.  Because the
         * components are in a flattened 2D sort, we don't need to
         * look far ahead in the array to find all overlapping boxes;
         * 10 boxes is plenty. */
    boxad = boxaHandleOverlaps(boxa3, L_COMBINE, 10, 0.5f, 0.3f, NULL);
    boxaDestroy(&boxa3);

        /* Extract and save the image pieces from the input image. */
    *ppixa = pixClipRectangles(pixs, boxad);
    *pboxa = boxad;
    return 0;
}


/*------------------------------------------------------------------------*
 *                       Greedy character splitting                       *
 *------------------------------------------------------------------------*/
/*!
 * \brief   recogCorrelationBestRow()
 *
 * \param[in]    recog       with LUT's pre-computed
 * \param[in]    pixs        typically of multiple touching characters, 1 bpp
 * \param[out]   pboxa       bounding boxs of best fit character
 * \param[out]   pnascore    [optional] correlation scores
 * \param[out]   pnaindex    [optional] indices of classes
 * \param[out]   psachar     [optional] array of character strings
 * \param[in]    debug       1 for results written to pixadb_split
 * \return  0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *      (1) Supervises character matching for (in general) a c.c with
 *          multiple touching characters.  Finds the best match greedily.
 *          Rejects small parts that are left over after splitting.
 *      (2) Matching is to the average, and without character scaling.
 * </pre>
 */
l_ok
recogCorrelationBestRow(L_RECOG  *recog,
                        PIX      *pixs,
                        BOXA    **pboxa,
                        NUMA    **pnascore,
                        NUMA    **pnaindex,
                        SARRAY  **psachar,
                        l_int32   debug)
{
char      *charstr;
l_int32    index, remove, w, h, bx, bw, bxc, bwc, w1, w2, w3;
l_float32  score;
BOX       *box, *boxc, *boxtrans, *boxl, *boxr, *boxlt, *boxrt;
BOXA      *boxat;
NUMA      *nascoret, *naindext, *nasort;
PIX       *pixb, *pixc, *pixl, *pixr, *pixdb, *pixd;
PIXA      *pixar, *pixadb;
SARRAY    *sachart;

l_int32    iter;

    if (pnascore) *pnascore = NULL;
    if (pnaindex) *pnaindex = NULL;
    if (psachar) *psachar = NULL;
    if (!pboxa)
        return ERROR_INT("&boxa not defined", __func__, 1);
    *pboxa = NULL;
    if (!recog)
        return ERROR_INT("recog not defined", __func__, 1);
    if (!pixs || pixGetDepth(pixs) != 1)
        return ERROR_INT("pixs not defined or not 1 bpp", __func__, 1);
    if (pixGetWidth(pixs) < recog->minwidth_u - 4)
        return ERROR_INT("pixs too narrow", __func__, 1);
    if (!recog->train_done)
        return ERROR_INT("training not finished", __func__, 1);

        /* Binarize and crop to foreground if necessary */
    pixb = recogProcessToIdentify(recog, pixs, 0);

        /* Initialize the arrays */
    boxat = boxaCreate(4);
    nascoret = numaCreate(4);
    naindext = numaCreate(4);
    sachart = sarrayCreate(4);
    pixadb = (debug) ? pixaCreate(4) : NULL;

        /* Initialize the images remaining to be processed with the input.
         * These are stored in pixar, which is used here as a queue,
         * on which we only put image fragments that are large enough to
         * contain at least one character.  */
    pixar = pixaCreate(1);
    pixGetDimensions(pixb, &w, &h, NULL);
    box = boxCreate(0, 0, w, h);
    pixaAddPix(pixar, pixb, L_INSERT);
    pixaAddBox(pixar, box, L_INSERT);

        /* Successively split on the best match until nothing is left.
         * To be safe, we limit the search to 10 characters. */
    for (iter = 0; iter < 11; iter++) {
        if (pixaGetCount(pixar) == 0)
            break;
        if (iter == 10) {
            L_WARNING("more than 10 chars; ending search\n", __func__);
            break;
        }

            /* Pop one from the queue */
        pixaRemovePixAndSave(pixar, 0, &pixc, &boxc);
        boxGetGeometry(boxc, &bxc, NULL, &bwc, NULL);

            /* This is a single component; if noise, remove it */
        recogSplittingFilter(recog, pixc, 0, MinFillFactor, &remove, debug);
        if (debug)
            lept_stderr("iter = %d, removed = %d\n", iter, remove);
        if (remove) {
            pixDestroy(&pixc);
            boxDestroy(&boxc);
            continue;
        }

            /* Find the best character match */
        if (debug) {
            recogCorrelationBestChar(recog, pixc, &box, &score,
                                     &index, &charstr, &pixdb);
            pixaAddPix(pixadb, pixdb, L_INSERT);
        } else {
            recogCorrelationBestChar(recog, pixc, &box, &score,
                                     &index, &charstr, NULL);
        }

            /* Find the box in original coordinates, and append
             * the results to the arrays. */
        boxtrans = boxTransform(box, bxc, 0, 1.0, 1.0);
        boxaAddBox(boxat, boxtrans, L_INSERT);
        numaAddNumber(nascoret, score);
        numaAddNumber(naindext, index);
        sarrayAddString(sachart, charstr, L_INSERT);

            /* Split the current pixc into three regions and save
             * each region if it is large enough. */
        boxGetGeometry(box, &bx, NULL, &bw, NULL);
        w1 = bx;
        w2 = bw;
        w3 = bwc - bx - bw;
        if (debug)
            lept_stderr(" w1 = %d, w2 = %d, w3 = %d\n", w1, w2, w3);
        if (w1 < recog->minwidth_u - 4) {
            if (debug) L_INFO("discarding width %d on left\n", __func__, w1);
        } else {  /* extract and save left region */
            boxl = boxCreate(0, 0, bx + 1, h);
            pixl = pixClipRectangle(pixc, boxl, NULL);
            boxlt = boxTransform(boxl, bxc, 0, 1.0, 1.0);
            pixaAddPix(pixar, pixl, L_INSERT);
            pixaAddBox(pixar, boxlt, L_INSERT);
            boxDestroy(&boxl);
        }
        if (w3 < recog->minwidth_u - 4) {
            if (debug) L_INFO("discarding width %d on right\n", __func__, w3);
        } else {  /* extract and save left region */
            boxr = boxCreate(bx + bw - 1, 0, w3 + 1, h);
            pixr = pixClipRectangle(pixc, boxr, NULL);
            boxrt = boxTransform(boxr, bxc, 0, 1.0, 1.0);
            pixaAddPix(pixar, pixr, L_INSERT);
            pixaAddBox(pixar, boxrt, L_INSERT);
            boxDestroy(&boxr);
        }
        pixDestroy(&pixc);
        boxDestroy(&box);
        boxDestroy(&boxc);
    }
    pixaDestroy(&pixar);


        /* Sort the output results by left-to-right in the boxa */
    *pboxa = boxaSort(boxat, L_SORT_BY_X, L_SORT_INCREASING, &nasort);
    if (pnascore)
        *pnascore = numaSortByIndex(nascoret, nasort);
    if (pnaindex)
        *pnaindex = numaSortByIndex(naindext, nasort);
    if (psachar)
        *psachar = sarraySortByIndex(sachart, nasort);
    numaDestroy(&nasort);
    boxaDestroy(&boxat);
    numaDestroy(&nascoret);
    numaDestroy(&naindext);
    sarrayDestroy(&sachart);

        /* Final debug output */
    if (debug) {
        pixd = pixaDisplayTiledInRows(pixadb, 32, 2000, 1.0, 0, 15, 2);
        pixDisplay(pixd, 400, 400);
        pixaAddPix(recog->pixadb_split, pixd, L_INSERT);
        pixaDestroy(&pixadb);
    }
    return 0;
}


/*!
 * \brief   recogCorrelationBestChar()
 *
 * \param[in]    recog       with LUT's pre-computed
 * \param[in]    pixs        can be of multiple touching characters, 1 bpp
 * \param[out]   pbox        bounding box of best fit character
 * \param[out]   pscore      correlation score
 * \param[out]   pindex      [optional] index of class
 * \param[out]   pcharstr    [optional] character string of class
 * \param[out]   ppixdb      [optional] debug pix showing input and best fit
 * \return  0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *      (1) Basic matching character splitter.  Finds the best match among
 *          all templates to some region of the image.  This can result
 *          in splitting the image into two parts.  This is "image decoding"
 *          without dynamic programming, because we don't use a setwidth
 *          and compute the best matching score for the entire image.
 *      (2) Matching is to the average templates, without character scaling.
 * </pre>
 */
l_ok
recogCorrelationBestChar(L_RECOG    *recog,
                         PIX        *pixs,
                         BOX       **pbox,
                         l_float32  *pscore,
                         l_int32    *pindex,
                         char      **pcharstr,
                         PIX       **ppixdb)
{
l_int32    i, n, w1, h1, w2, area2, ycent2, delx, dely;
l_int32    bestdelx, bestdely, bestindex;
l_float32  score, bestscore;
BOX       *box;
BOXA      *boxa;
NUMA      *nasum, *namoment;
PIX       *pix1, *pix2;

    if (pindex) *pindex = 0;
    if (pcharstr) *pcharstr = NULL;
    if (ppixdb) *ppixdb = NULL;
    if (pbox) *pbox = NULL;
    if (pscore) *pscore = 0.0;
    if (!pbox || !pscore)
        return ERROR_INT("&box and &score not both defined", __func__, 1);
    if (!recog)
        return ERROR_INT("recog not defined", __func__, 1);
    if (!pixs || pixGetDepth(pixs) != 1)
        return ERROR_INT("pixs not defined or not 1 bpp", __func__, 1);
    if (!recog->train_done)
        return ERROR_INT("training not finished", __func__, 1);

        /* Binarize and crop to foreground if necessary.  Add padding
         * to both the left and right side; this is compensated for
         * when reporting the bounding box of the best matched character. */
    pix1 = recogProcessToIdentify(recog, pixs, LeftRightPadding);
    pixGetDimensions(pix1, &w1, &h1, NULL);

        /* Compute vertical sum and moment arrays */
    nasum = pixCountPixelsByColumn(pix1);
    namoment = pixGetMomentByColumn(pix1, 1);

        /* Do shifted correlation against all averaged templates. */
    n = recog->setsize;
    boxa = boxaCreate(n);  /* location of best fits for each character */
    bestscore = 0.0;
    bestindex = bestdelx = bestdely = 0;
    for (i = 0; i < n; i++) {
        pix2 = pixaGetPix(recog->pixa_u, i, L_CLONE);
        w2 = pixGetWidth(pix2);
            /* Note that the slightly expended w1 is typically larger
             * than w2 (the template). */
        if (w1 >= w2) {
            numaGetIValue(recog->nasum_u, i, &area2);
            ptaGetIPt(recog->pta_u, i, NULL, &ycent2);
            pixCorrelationBestShift(pix1, pix2, nasum, namoment, area2, ycent2,
                                    recog->maxyshift, recog->sumtab, &delx,
                                    &dely, &score, 1);
            if (ppixdb) {
                lept_stderr(
                    "Best match template %d: (x,y) = (%d,%d), score = %5.3f\n",
                    i, delx, dely, score);
            }
                  /* Compensate for padding */
            box = boxCreate(delx - LeftRightPadding, 0, w2, h1);
            if (score > bestscore) {
                bestscore = score;
                bestdelx = delx - LeftRightPadding;
                bestdely = dely;
                bestindex = i;
            }
        } else {
            box = boxCreate(0, 0, 1, 1);  /* placeholder */
            if (ppixdb)
                lept_stderr("Component too thin: w1 = %d, w2 = %d\n", w1, w2);
        }
        boxaAddBox(boxa, box, L_INSERT);
        pixDestroy(&pix2);
    }

    *pscore = bestscore;
    *pbox = boxaGetBox(boxa, bestindex, L_COPY);
    if (pindex) *pindex = bestindex;
    if (pcharstr)
        recogGetClassString(recog, bestindex, pcharstr);

    if (ppixdb) {
        L_INFO("Best match: class %d; shifts (%d, %d)\n",
               __func__, bestindex, bestdelx, bestdely);
        pix2 = pixaGetPix(recog->pixa_u, bestindex, L_CLONE);
        *ppixdb = recogShowMatch(recog, pix1, pix2, NULL, -1, 0.0);
        pixDestroy(&pix2);
    }

    pixDestroy(&pix1);
    boxaDestroy(&boxa);
    numaDestroy(&nasum);
    numaDestroy(&namoment);
    return 0;
}


/*!
 * \brief   pixCorrelationBestShift()
 *
 * \param[in]    pix1        1 bpp, the unknown image; typically larger
 * \param[in]    pix2        1 bpp, the matching template image)
 * \param[in]    nasum1      vertical column pixel sums for pix1
 * \param[in]    namoment1   vertical column first moment of pixels for pix1
 * \param[in]    area2       number of on pixels in pix2
 * \param[in]    ycent2      y component of centroid of pix2
 * \param[in]    maxyshift   max y shift of pix2 around the location where
 *                           the centroids of pix2 and a windowed part of pix1
 *                           are vertically aligned
 * \param[in]    tab8        [optional] sum tab for ON pixels in byte;
 *                           can be NULL
 * \param[out]   pdelx       [optional] best x shift of pix2 relative to pix1
 * \param[out]   pdely       [optional] best y shift of pix2 relative to pix1
 * \param[out]   pscore      [optional] maximum score found; can be NULL
 * \param[in]    debugflag   <= 0 to skip; positive to generate output;
 *                           the integer is used to label the debug image.
 * \return  0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *      (1) This maximizes the correlation score between two 1 bpp images,
 *          one of which is typically wider.  In a typical example,
 *          pix1 is a bitmap of 2 or more touching characters and pix2 is
 *          a single character template.  This finds the location of pix2
 *          that gives the largest correlation.
 *      (2) The windowed area of fg pixels and windowed first moment
 *          in the y direction are computed from the input sum and moment
 *          column arrays, %nasum1 and %namoment1
 *      (3) This is a brute force operation.  We compute the correlation
 *          at every x shift for which pix2 fits entirely within pix1,
 *          and where the centroid of pix2 is aligned, within +-maxyshift,
 *          with the centroid of a window of pix1 of the same width.
 *          The correlation is taken over the full height of pix1.
 *          This can be made more efficient.
 * </pre>
 */
static l_int32
pixCorrelationBestShift(PIX        *pix1,
                        PIX        *pix2,
                        NUMA       *nasum1,
                        NUMA       *namoment1,
                        l_int32     area2,
                        l_int32     ycent2,
                        l_int32     maxyshift,
                        l_int32    *tab8,
                        l_int32    *pdelx,
                        l_int32    *pdely,
                        l_float32  *pscore,
                        l_int32     debugflag)
{
l_int32     w1, w2, h1, h2, i, j, nx, shifty, delx, dely;
l_int32     sum, moment, count;
l_int32    *tab, *area1, *arraysum, *arraymoment;
l_float32   maxscore, score;
l_float32  *ycent1;
FPIX       *fpix = NULL;
PIX        *pixt, *pixt1, *pixt2;

    if (pdelx) *pdelx = 0;
    if (pdely) *pdely = 0;
    if (pscore) *pscore = 0.0;
    if (!pix1 || pixGetDepth(pix1) != 1)
        return ERROR_INT("pix1 not defined or not 1 bpp", __func__, 1);
    if (!pix2 || pixGetDepth(pix2) != 1)
        return ERROR_INT("pix2 not defined or not 1 bpp", __func__, 1);
    if (!nasum1 || !namoment1)
        return ERROR_INT("nasum1 and namoment1 not both defined", __func__, 1);
    if (area2 <= 0 || ycent2 <= 0)
        return ERROR_INT("area2 and ycent2 must be > 0", __func__, 1);

       /* If pix1 (the unknown image) is narrower than pix2,
        * don't bother to try the match.  pix1 is already padded with
        * 2 pixels on each side. */
    pixGetDimensions(pix1, &w1, &h1, NULL);
    pixGetDimensions(pix2, &w2, &h2, NULL);
    if (w1 < w2) {
        if (debugflag > 0) {
            L_INFO("skipping match with w1 = %d and w2 = %d\n",
                   __func__, w1, w2);
        }
        return 0;
    }
    nx = w1 - w2 + 1;

    if (debugflag > 0)
        fpix = fpixCreate(nx, 2 * maxyshift + 1);
    if (!tab8)
        tab = makePixelSumTab8();
    else
        tab = tab8;

        /* Set up the arrays for area1 and ycent1.  We have to do this
         * for each template (pix2) because the window width is w2. */
    area1 = (l_int32 *)LEPT_CALLOC(nx, sizeof(l_int32));
    ycent1 = (l_float32 *)LEPT_CALLOC(nx, sizeof(l_int32));
    arraysum = numaGetIArray(nasum1);
    arraymoment = numaGetIArray(namoment1);
    for (i = 0, sum = 0, moment = 0; i < w2; i++) {
        sum += arraysum[i];
        moment += arraymoment[i];
    }
    for (i = 0; i < nx - 1; i++) {
        area1[i] = sum;
        ycent1[i] = (sum == 0) ? ycent2 : (l_float32)moment / (l_float32)sum;
        sum += arraysum[w2 + i] - arraysum[i];
        moment += arraymoment[w2 + i] - arraymoment[i];
    }
    area1[nx - 1] = sum;
    ycent1[nx - 1] = (sum == 0) ? ycent2 : (l_float32)moment / (l_float32)sum;

        /* Find the best match location for pix2.  At each location,
         * to insure that pixels are ON only within the intersection of
         * pix and the shifted pix2:
         *  (1) Start with pixt cleared and equal in size to pix1.
         *  (2) Blit the shifted pix2 onto pixt.  Then all ON pixels
         *      are within the intersection of pix1 and the shifted pix2.
         *  (3) AND pix1 with pixt. */
    pixt = pixCreate(w2, h1, 1);
    maxscore = 0;
    delx = 0;
    dely = 0;  /* amount to shift pix2 relative to pix1 to get alignment */
    for (i = 0; i < nx; i++) {
        shifty = (l_int32)(ycent1[i] - ycent2 + 0.5);
        for (j = -maxyshift; j <= maxyshift; j++) {
            pixClearAll(pixt);
            pixRasterop(pixt, 0, shifty + j, w2, h2, PIX_SRC, pix2, 0, 0);
            pixRasterop(pixt, 0, 0, w2, h1, PIX_SRC & PIX_DST, pix1, i, 0);
            pixCountPixels(pixt, &count, tab);
            score = (l_float32)count * (l_float32)count /
                    ((l_float32)area1[i] * (l_float32)area2);
            if (score > maxscore) {
                maxscore = score;
                delx = i;
                dely = shifty + j;
            }

            if (debugflag > 0)
                fpixSetPixel(fpix, i, maxyshift + j, 1000.0 * score);
        }
    }

    if (debugflag > 0) {
        char  buf[128];
        lept_mkdir("lept/recog");
        pixt1 = fpixDisplayMaxDynamicRange(fpix);
        pixt2 = pixExpandReplicate(pixt1, 5);
        snprintf(buf, sizeof(buf), "/tmp/lept/recog/junkbs_%d.png", debugflag);
        pixWrite(buf, pixt2, IFF_PNG);
        pixDestroy(&pixt1);
        pixDestroy(&pixt2);
        fpixDestroy(&fpix);
    }

    if (pdelx) *pdelx = delx;
    if (pdely) *pdely = dely;
    if (pscore) *pscore = maxscore;
    if (!tab8) LEPT_FREE(tab);
    LEPT_FREE(area1);
    LEPT_FREE(ycent1);
    LEPT_FREE(arraysum);
    LEPT_FREE(arraymoment);
    pixDestroy(&pixt);
    return 0;
}


/*------------------------------------------------------------------------*
 *                          Low-level identification                      *
 *------------------------------------------------------------------------*/
/*!
 * \brief   recogIdentifyPixa()
 *
 * \param[in]    recog
 * \param[in]    pixa     of 1 bpp images to match
 * \param[out]   ppixdb   [optional] pix showing inputs and best fits
 * \return  0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *      (1) This should be called by recogIdentifyMuliple(), which
 *          binarizes and splits characters before sending %pixa here.
 *      (2) This calls recogIdentifyPix(), which does the same operation
 *          on each pix in %pixa, and optionally returns the arrays
 *          of results (scores, class index and character string)
 *          for the best correlation match.
 * </pre>
 */
l_ok
recogIdentifyPixa(L_RECOG  *recog,
                  PIXA     *pixa,
                  PIX     **ppixdb)
{
char      *text;
l_int32    i, n, fail, index, depth;
l_float32  score;
PIX       *pix1, *pix2, *pix3;
PIXA      *pixa1;
L_RCH     *rch;

    if (ppixdb) *ppixdb = NULL;
    if (!recog)
        return ERROR_INT("recog not defined", __func__, 1);
    if (!pixa)
        return ERROR_INT("pixa not defined", __func__, 1);

        /* Run the recognizer on the set of images.  This writes
         * the text string into each pix in pixa. */
    n = pixaGetCount(pixa);
    rchaDestroy(&recog->rcha);
    recog->rcha = rchaCreate();
    pixa1 = (ppixdb) ? pixaCreate(n) : NULL;
    depth = 1;
    for (i = 0; i < n; i++) {
        pix1 = pixaGetPix(pixa, i, L_CLONE);
        pix2 = NULL;
        fail = FALSE;
        if (!ppixdb)
            fail = recogIdentifyPix(recog, pix1, NULL);
        else
            fail = recogIdentifyPix(recog, pix1, &pix2);
        if (fail)
            recogSkipIdentify(recog);
        if ((rch = recog->rch) == NULL) {
            L_ERROR("rch not found for char %d\n", __func__, i);
            pixDestroy(&pix1);
            pixDestroy(&pix2);
            continue;
        }
        rchExtract(rch, NULL, NULL, &text, NULL, NULL, NULL, NULL);
        pixSetText(pix1, text);
        LEPT_FREE(text);
        if (ppixdb) {
            rchExtract(rch, &index, &score, NULL, NULL, NULL, NULL, NULL);
            pix3 = recogShowMatch(recog, pix2, NULL, NULL, index, score);
            if (i == 0) depth = pixGetDepth(pix3);
            pixaAddPix(pixa1, pix3, L_INSERT);
            pixDestroy(&pix2);
        }
        transferRchToRcha(rch, recog->rcha);
        pixDestroy(&pix1);
    }

        /* Package the images for debug */
    if (ppixdb) {
        *ppixdb = pixaDisplayTiledInRows(pixa1, depth, 2500, 1.0, 0, 20, 1);
        pixaDestroy(&pixa1);
    }

    return 0;
}


/*!
 * \brief   recogIdentifyPix()
 *
 * \param[in]    recog     with LUT's pre-computed
 * \param[in]    pixs      of a single character, 1 bpp
 * \param[out]   ppixdb    [optional] debug pix showing input and best fit
 * \return  0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *      (1) Basic recognition function for a single character.
 *      (2) If templ_use == L_USE_ALL_TEMPLATES, which is the default
 *          situation, matching is attempted to every bitmap in the recog,
 *          and the identify of the best match is returned.
 *      (3) For finding outliers, templ_use == L_USE_AVERAGE_TEMPLATES, and
 *          matching is only attemplted to the averaged bitmaps.  For this
 *          case, the index of the bestsample is meaningless (0 is returned
 *          if requested).
 *      (4) The score is related to the confidence (probability of correct
 *          identification), in that a higher score is correlated with
 *          a higher probability.  However, the actual relation between
 *          the correlation (score) and the probability is not known;
 *          we call this a "score" because "confidence" can be misinterpreted
 *          as an actual probability.
 * </pre>
 */
l_ok
recogIdentifyPix(L_RECOG  *recog,
                 PIX      *pixs,
                 PIX     **ppixdb)
{
char      *text;
l_int32    i, j, n, bestindex, bestsample, area1, area2, ret;
l_int32    shiftx, shifty, bestdelx, bestdely, bestwidth, maxyshift;
l_float32  x1, y1, x2, y2, delx, dely, score, maxscore;
NUMA      *numa;
PIX       *pix0, *pix1, *pix2;
PIXA      *pixa;
PTA       *pta;

    if (ppixdb) *ppixdb = NULL;
    if (!recog)
        return ERROR_INT("recog not defined", __func__, 1);
    if (!pixs || pixGetDepth(pixs) != 1)
        return ERROR_INT("pixs not defined or not 1 bpp", __func__, 1);

        /* Do the averaging if required and not yet done. */
    if (recog->templ_use == L_USE_AVERAGE_TEMPLATES && !recog->ave_done) {
        ret = recogAverageSamples(recog, 0);
        if (ret)
            return ERROR_INT("averaging failed", __func__, 1);
    }

        /* Binarize and crop to foreground if necessary */
    if ((pix0 = recogProcessToIdentify(recog, pixs, 0)) == NULL)
        return ERROR_INT("no fg pixels in pix0", __func__, 1);

        /* Optionally scale and/or convert to fixed stroke width */
    pix1 = recogModifyTemplate(recog, pix0);
    pixDestroy(&pix0);
    if (!pix1)
        return ERROR_INT("no fg pixels in pix1", __func__, 1);

        /* Do correlation at all positions within +-maxyshift of
         * the nominal centroid alignment. */
    pixCountPixels(pix1, &area1, recog->sumtab);
    pixCentroid(pix1, recog->centtab, recog->sumtab, &x1, &y1);
    bestindex = bestsample = bestdelx = bestdely = bestwidth = 0;
    maxscore = 0.0;
    maxyshift = recog->maxyshift;
    if (recog->templ_use == L_USE_AVERAGE_TEMPLATES) {
        for (i = 0; i < recog->setsize; i++) {
            numaGetIValue(recog->nasum, i, &area2);
            if (area2 == 0) continue;  /* no template available */
            pix2 = pixaGetPix(recog->pixa, i, L_CLONE);
            ptaGetPt(recog->pta, i, &x2, &y2);
            delx = x1 - x2;
            dely = y1 - y2;
            for (shifty = -maxyshift; shifty <= maxyshift; shifty++) {
                for (shiftx = -maxyshift; shiftx <= maxyshift; shiftx++) {
                    pixCorrelationScoreSimple(pix1, pix2, area1, area2,
                                              delx + shiftx, dely + shifty,
                                              5, 5, recog->sumtab, &score);
                    if (score > maxscore) {
                        bestindex = i;
                        bestdelx = delx + shiftx;
                        bestdely = dely + shifty;
                        maxscore = score;
                    }
                }
            }
            pixDestroy(&pix2);
        }
    } else {  /* use all the samples */
        for (i = 0; i < recog->setsize; i++) {
            pixa = pixaaGetPixa(recog->pixaa, i, L_CLONE);
            n = pixaGetCount(pixa);
            if (n == 0) {
                pixaDestroy(&pixa);
                continue;
            }
            numa = numaaGetNuma(recog->naasum, i, L_CLONE);
            pta = ptaaGetPta(recog->ptaa, i, L_CLONE);
            for (j = 0; j < n; j++) {
                pix2 = pixaGetPix(pixa, j, L_CLONE);
                numaGetIValue(numa, j, &area2);
                ptaGetPt(pta, j, &x2, &y2);
                delx = x1 - x2;
                dely = y1 - y2;
                for (shifty = -maxyshift; shifty <= maxyshift; shifty++) {
                    for (shiftx = -maxyshift; shiftx <= maxyshift; shiftx++) {
                        pixCorrelationScoreSimple(pix1, pix2, area1, area2,
                                                  delx + shiftx, dely + shifty,
                                                  5, 5, recog->sumtab, &score);
                        if (score > maxscore) {
                            bestindex = i;
                            bestsample = j;
                            bestdelx = delx + shiftx;
                            bestdely = dely + shifty;
                            maxscore = score;
                            bestwidth = pixGetWidth(pix2);
                        }
                    }
                }
                pixDestroy(&pix2);
            }
            pixaDestroy(&pixa);
            numaDestroy(&numa);
            ptaDestroy(&pta);
        }
    }

        /* Package up the results */
    recogGetClassString(recog, bestindex, &text);
    rchDestroy(&recog->rch);
    recog->rch = rchCreate(bestindex, maxscore, text, bestsample,
                           bestdelx, bestdely, bestwidth);

    if (ppixdb) {
        if (recog->templ_use == L_USE_AVERAGE_TEMPLATES) {
            L_INFO("Best match: str %s; class %d; sh (%d, %d); score %5.3f\n",
                   __func__, text, bestindex, bestdelx, bestdely, maxscore);
            pix2 = pixaGetPix(recog->pixa, bestindex, L_CLONE);
        } else {  /* L_USE_ALL_TEMPLATES */
            L_INFO("Best match: str %s; sample %d in class %d; score %5.3f\n",
                   __func__, text, bestsample, bestindex, maxscore);
            if (maxyshift > 0 && (L_ABS(bestdelx) > 0 || L_ABS(bestdely) > 0)) {
                L_INFO("  Best shift: (%d, %d)\n",
                       __func__, bestdelx, bestdely);
            }
            pix2 = pixaaGetPix(recog->pixaa, bestindex, bestsample, L_CLONE);
        }
        *ppixdb = recogShowMatch(recog, pix1, pix2, NULL, -1, 0.0);
        pixDestroy(&pix2);
    }

    pixDestroy(&pix1);
    return 0;
}


/*!
 * \brief   recogSkipIdentify()
 *
 * \param[in]    recog
 * \return  0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *      (1) This just writes a "dummy" result with 0 score and empty
 *          string id into the rch.
 * </pre>
 */
l_ok
recogSkipIdentify(L_RECOG  *recog)
{
    if (!recog)
        return ERROR_INT("recog not defined", __func__, 1);

        /* Package up placeholder results */
    rchDestroy(&recog->rch);
    recog->rch = rchCreate(0, 0.0, stringNew(""), 0, 0, 0, 0);
    return 0;
}


/*------------------------------------------------------------------------*
 *             Operations for handling identification results             *
 *------------------------------------------------------------------------*/
/*!
 * \brief   rchaCreate()
 *
 *      Return: 0 if OK, 1 on error
 *
 *  Notes:
 *      (1) Be sure to destroy any existing rcha before assigning this.
 */
static L_RCHA *
rchaCreate()
{
L_RCHA  *rcha;

    rcha = (L_RCHA *)LEPT_CALLOC(1, sizeof(L_RCHA));
    rcha->naindex = numaCreate(0);
    rcha->nascore = numaCreate(0);
    rcha->satext = sarrayCreate(0);
    rcha->nasample = numaCreate(0);
    rcha->naxloc = numaCreate(0);
    rcha->nayloc = numaCreate(0);
    rcha->nawidth = numaCreate(0);
    return rcha;
}


/*!
 * \brief   rchaDestroy()
 *
 * \param[in,out]   prcha     to be nulled
 */
void
rchaDestroy(L_RCHA  **prcha)
{
L_RCHA  *rcha;

    if (prcha == NULL) {
        L_WARNING("&rcha is null!\n", __func__);
        return;
    }
    if ((rcha = *prcha) == NULL)
        return;

    numaDestroy(&rcha->naindex);
    numaDestroy(&rcha->nascore);
    sarrayDestroy(&rcha->satext);
    numaDestroy(&rcha->nasample);
    numaDestroy(&rcha->naxloc);
    numaDestroy(&rcha->nayloc);
    numaDestroy(&rcha->nawidth);
    LEPT_FREE(rcha);
    *prcha = NULL;
}


/*!
 * \brief   rchCreate()
 *
 * \param[in]    index    index of best template
 * \param[in]    score    correlation score of best template
 * \param[in]    text     character string of best template
 * \param[in]    sample   index of best sample; -1 if averages are used
 * \param[in]    xloc     x-location of template: delx + shiftx
 * \param[in]    yloc     y-location of template: dely + shifty
 * \param[in]    width    width of best template
 * \return  0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *      (1) Be sure to destroy any existing rch before assigning this.
 *      (2) This stores the text string, not a copy of it, so the
 *          caller must not destroy the string.
 * </pre>
 */
static L_RCH *
rchCreate(l_int32    index,
          l_float32  score,
          char      *text,
          l_int32    sample,
          l_int32    xloc,
          l_int32    yloc,
          l_int32    width)
{
L_RCH  *rch;

    rch = (L_RCH *)LEPT_CALLOC(1, sizeof(L_RCH));
    rch->index = index;
    rch->score = score;
    rch->text = text;
    rch->sample = sample;
    rch->xloc = xloc;
    rch->yloc = yloc;
    rch->width = width;
    return rch;
}


/*!
 * \brief   rchDestroy()
 *
 * \param[in,out] prch to be nulled
 */
void
rchDestroy(L_RCH  **prch)
{
L_RCH  *rch;

    if (prch == NULL) {
        L_WARNING("&rch is null!\n", __func__);
        return;
    }
    if ((rch = *prch) == NULL)
        return;
    LEPT_FREE(rch->text);
    LEPT_FREE(rch);
    *prch = NULL;
}


/*!
 * \brief   rchaExtract()
 *
 * \param[in]    rcha
 * \param[out]   pnaindex    [optional] indices of best templates
 * \param[out]   pnascore    [optional] correl scores of best templates
 * \param[out]   psatext     [optional] character strings of best templates
 * \param[out]   pnasample   [optional] indices of best samples
 * \param[out]   pnaxloc     [optional] x-locations of templates
 * \param[out]   pnayloc     [optional] y-locations of templates
 * \param[out]   pnawidth    [optional] widths of best templates
 * \return  0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *      (1) This returns clones of the number and string arrays.  They must
 *          be destroyed by the caller.
 * </pre>
 */
l_ok
rchaExtract(L_RCHA   *rcha,
            NUMA    **pnaindex,
            NUMA    **pnascore,
            SARRAY  **psatext,
            NUMA    **pnasample,
            NUMA    **pnaxloc,
            NUMA    **pnayloc,
            NUMA    **pnawidth)
{
    if (pnaindex) *pnaindex = NULL;
    if (pnascore) *pnascore = NULL;
    if (psatext) *psatext = NULL;
    if (pnasample) *pnasample = NULL;
    if (pnaxloc) *pnaxloc = NULL;
    if (pnayloc) *pnayloc = NULL;
    if (pnawidth) *pnawidth = NULL;
    if (!rcha)
        return ERROR_INT("rcha not defined", __func__, 1);

    if (pnaindex) *pnaindex = numaClone(rcha->naindex);
    if (pnascore) *pnascore = numaClone(rcha->nascore);
    if (psatext) *psatext = sarrayClone(rcha->satext);
    if (pnasample) *pnasample = numaClone(rcha->nasample);
    if (pnaxloc) *pnaxloc = numaClone(rcha->naxloc);
    if (pnayloc) *pnayloc = numaClone(rcha->nayloc);
    if (pnawidth) *pnawidth = numaClone(rcha->nawidth);
    return 0;
}


/*!
 * \brief   rchExtract()
 *
 * \param[in]    rch
 * \param[out]   pindex    [optional] index of best template
 * \param[out]   pscore    [optional] correlation score of best template
 * \param[out]   ptext     [optional] character string of best template
 * \param[out]   psample   [optional] index of best sample
 * \param[out]   pxloc     [optional] x-location of template
 * \param[out]   pyloc     [optional] y-location of template
 * \param[out]   pwidth    [optional] width of best template
 * \return  0 if OK, 1 on error
 */
l_ok
rchExtract(L_RCH      *rch,
           l_int32    *pindex,
           l_float32  *pscore,
           char      **ptext,
           l_int32    *psample,
           l_int32    *pxloc,
           l_int32    *pyloc,
           l_int32    *pwidth)
{
    if (pindex) *pindex = 0;
    if (pscore) *pscore = 0.0;
    if (ptext) *ptext = NULL;
    if (psample) *psample = 0;
    if (pxloc) *pxloc = 0;
    if (pyloc) *pyloc = 0;
    if (pwidth) *pwidth = 0;
    if (!rch)
        return ERROR_INT("rch not defined", __func__, 1);

    if (pindex) *pindex = rch->index;
    if (pscore) *pscore = rch->score;
    if (ptext) *ptext = stringNew(rch->text);  /* new string: owned by caller */
    if (psample) *psample = rch->sample;
    if (pxloc) *pxloc = rch->xloc;
    if (pyloc) *pyloc = rch->yloc;
    if (pwidth) *pwidth = rch->width;
    return 0;
}


/*!
 * \brief   transferRchToRcha()
 *
 * \param[in]    rch     source of data
 * \param[in]    rcha    append to arrays in this destination
 * \return  0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *      (1) This is used to transfer the results of a single character
 *          identification to an rcha array for the array of characters.
 * </pre>
 */
static l_int32
transferRchToRcha(L_RCH   *rch,
                  L_RCHA  *rcha)
{

    if (!rch)
        return ERROR_INT("rch not defined", __func__, 1);
    if (!rcha)
        return ERROR_INT("rcha not defined", __func__, 1);

    numaAddNumber(rcha->naindex, rch->index);
    numaAddNumber(rcha->nascore, rch->score);
    sarrayAddString(rcha->satext, rch->text, L_COPY);
    numaAddNumber(rcha->nasample, rch->sample);
    numaAddNumber(rcha->naxloc, rch->xloc);
    numaAddNumber(rcha->nayloc, rch->yloc);
    numaAddNumber(rcha->nawidth, rch->width);
    return 0;
}


/*------------------------------------------------------------------------*
 *                        Preprocessing and filtering                     *
 *------------------------------------------------------------------------*/
/*!
 * \brief   recogProcessToIdentify()
 *
 * \param[in]    recog     with LUT's pre-computed
 * \param[in]    pixs      typ. single character, possibly d > 1 and uncropped
 * \param[in]    pad       extra pixels added to left and right sides
 * \return  pixd 1 bpp, clipped to foreground, or NULL if there
 *                    are no fg pixels or on error.
 *
 * <pre>
 * Notes:
 *      (1) This is a lightweight operation to insure that the input
 *          image is 1 bpp, properly cropped, and padded on each side.
 *          If bpp > 1, the image is thresholded.
 * </pre>
 */
PIX *
recogProcessToIdentify(L_RECOG  *recog,
                       PIX      *pixs,
                       l_int32   pad)
{
l_int32  canclip;
PIX     *pix1, *pix2, *pixd;

    if (!recog)
        return (PIX *)ERROR_PTR("recog not defined", __func__, NULL);
    if (!pixs)
        return (PIX *)ERROR_PTR("pixs not defined", __func__, NULL);

    if (pixGetDepth(pixs) != 1)
        pix1 = pixThresholdToBinary(pixs, recog->threshold);
    else
        pix1 = pixClone(pixs);
    pixTestClipToForeground(pix1, &canclip);
    if (canclip)
        pixClipToForeground(pix1, &pix2, NULL);
    else
        pix2 = pixClone(pix1);
    pixDestroy(&pix1);
    if (!pix2)
        return (PIX *)ERROR_PTR("no foreground pixels", __func__, NULL);

    pixd = pixAddBorderGeneral(pix2, pad, pad, 0, 0, 0);
    pixDestroy(&pix2);
    return pixd;
}


/*!
 * \brief   recogPreSplittingFilter()
 *
 * \param[in]    recog
 * \param[in]    pixs     1 bpp, many connected components
 * \param[in]    minh     minimum height of components to be retained
 * \param[in]    minaf    minimum area fraction (|fg|/(w*h)) to be retained
 * \param[in]    debug    1 to output indicator arrays
 * \return  pixd with filtered components removed or NULL on error
 */
static PIX *
recogPreSplittingFilter(L_RECOG   *recog,
                        PIX       *pixs,
                        l_int32    minh,
                        l_float32  minaf,
                        l_int32    debug)
{
l_int32  scaling, minsplitw, maxsplith, maxasp;
BOXA    *boxas;
NUMA    *naw, *nah, *na1, *na1c, *na2, *na3, *na4, *na5, *na6, *na7;
PIX     *pixd;
PIXA    *pixas;

    if (!recog)
        return (PIX *)ERROR_PTR("recog not defined", __func__, NULL);
    if (!pixs)
        return (PIX *)ERROR_PTR("pixs not defined", __func__, NULL);

        /* If there is scaling, do not remove components based on the
         * values of min_splitw and max_splith. */
    scaling = (recog->scalew > 0 || recog->scaleh > 0) ? TRUE : FALSE;
    minsplitw = (scaling) ? 1 : recog->min_splitw - 3;
    maxsplith = (scaling) ? 150 : recog->max_splith;
    maxasp = recog->max_wh_ratio;

        /* Generate an indicator array of connected components to remove:
         *    short stuff
         *    tall stuff
         *    components with large width/height ratio
         *    components with small area fill fraction  */
    boxas = pixConnComp(pixs, &pixas, 8);
    pixaFindDimensions(pixas, &naw, &nah);
    na1 = numaMakeThresholdIndicator(naw, minsplitw, L_SELECT_IF_LT);
    na1c = numaCopy(na1);
    na2 = numaMakeThresholdIndicator(nah, minh, L_SELECT_IF_LT);
    na3 = numaMakeThresholdIndicator(nah, maxsplith, L_SELECT_IF_GT);
    na4 = pixaFindWidthHeightRatio(pixas);
    na5 = numaMakeThresholdIndicator(na4, maxasp, L_SELECT_IF_GT);
    na6 = pixaFindAreaFraction(pixas);
    na7 = numaMakeThresholdIndicator(na6, minaf, L_SELECT_IF_LT);
    numaLogicalOp(na1, na1, na2, L_UNION);
    numaLogicalOp(na1, na1, na3, L_UNION);
    numaLogicalOp(na1, na1, na5, L_UNION);
    numaLogicalOp(na1, na1, na7, L_UNION);
    pixd = pixCopy(NULL, pixs);
    pixRemoveWithIndicator(pixd, pixas, na1);
    if (debug)
        l_showIndicatorSplitValues(na1c, na2, na3, na5, na7, na1);
    numaDestroy(&naw);
    numaDestroy(&nah);
    numaDestroy(&na1);
    numaDestroy(&na1c);
    numaDestroy(&na2);
    numaDestroy(&na3);
    numaDestroy(&na4);
    numaDestroy(&na5);
    numaDestroy(&na6);
    numaDestroy(&na7);
    boxaDestroy(&boxas);
    pixaDestroy(&pixas);
    return pixd;
}


/*!
 * \brief   recogSplittingFilter()
 *
 * \param[in]    recog
 * \param[in]    pixs     1 bpp, single connected component
 * \param[in]    minh     minimum height of component; 0 for default
 * \param[in]    minaf    minimum area fraction (|fg|/(w*h)) to be retained
 * \param[out]   premove  0 to save, 1 to remove
 * \param[in]    debug    1 to output indicator arrays
 * \return  0 if OK, 1 on error
 */
static l_int32
recogSplittingFilter(L_RECOG   *recog,
                     PIX       *pixs,
                     l_int32    minh,
                     l_float32  minaf,
                     l_int32   *premove,
                     l_int32    debug)
{
l_int32    w, h;
l_float32  aspratio, fract;

    if (!premove)
        return ERROR_INT("&remove not defined", __func__, 1);
    *premove = 0;
    if (!recog)
        return ERROR_INT("recog not defined", __func__, 1);
    if (!pixs)
        return ERROR_INT("pixs not defined", __func__, 1);
    if (minh <= 0) minh = DefaultMinHeight;

        /* Remove from further consideration:
         *    small stuff
         *    components with large width/height ratio
         *    components with small area fill fraction */
    pixGetDimensions(pixs, &w, &h, NULL);
    if (w < recog->min_splitw) {
        if (debug) L_INFO("w = %d < %d\n", __func__, w, recog->min_splitw);
        *premove = 1;
        return 0;
    }
    if (h < minh) {
        if (debug) L_INFO("h = %d < %d\n", __func__, h, minh);
        *premove = 1;
        return 0;
    }
    aspratio = (l_float32)w / (l_float32)h;
    if (aspratio > recog->max_wh_ratio) {
        if (debug) L_INFO("w/h = %5.3f too large\n", __func__, aspratio);
        *premove = 1;
        return 0;
    }
    pixFindAreaFraction(pixs, recog->sumtab, &fract);
    if (fract < minaf) {
        if (debug) L_INFO("area fill fract %5.3f < %5.3f\n",
                          __func__, fract, minaf);
        *premove = 1;
        return 0;
    }

    return 0;
}


/*------------------------------------------------------------------------*
 *                              Postprocessing                            *
 *------------------------------------------------------------------------*/
/*!
 * \brief   recogExtractNumbers()
 *
 * \param[in]    recog
 * \param[in]    boxas         location of components
 * \param[in]    scorethresh   min score for which we accept a component
 * \param[in]    spacethresh   max horizontal distance allowed between digits;
 *                             use -1 for default
 * \param[out]   pbaa          [optional] bounding boxes of identified numbers
 * \param[out]   pnaa          [optional] scores of identified digits
 * \return  sa of identified numbers, or NULL on error
 *
 * <pre>
 * Notes:
 *      (1) This extracts digit data after recogaIdentifyMultiple() or
 *          lower-level identification has taken place.
 *      (2) Each string in the returned sa contains a sequence of ascii
 *          digits in a number.
 *      (3) The horizontal distance between boxes (limited by %spacethresh)
 *          is the negative of the horizontal overlap.
 *      (4) Components with a score less than %scorethresh, which may
 *          be hyphens or other small characters, will signal the
 *          end of the current sequence of digits in the number.  A typical
 *          value for %scorethresh is 0.60.
 *      (5) We allow two digits to be combined if these conditions apply:
 *            (a) the first is to the left of the second
 *            (b) the second has a horizontal separation less than %spacethresh
 *            (c) the vertical overlap >= 0 (vertical separation < 0)
 *            (d) both have a score that exceeds %scorethresh
 *      (6) Each numa in the optionally returned naa contains the digit
 *          scores of a number.  Each boxa in the optionally returned baa
 *          contains the bounding boxes of the digits in the number.
 * </pre>
 */
SARRAY *
recogExtractNumbers(L_RECOG   *recog,
                    BOXA      *boxas,
                    l_float32  scorethresh,
                    l_int32    spacethresh,
                    BOXAA    **pbaa,
                    NUMAA    **pnaa)
{
char      *str, *text;
l_int32    i, n, x1, x2, h_ovl, v_ovl, h_sep, v_sep;
l_float32  score;
BOX       *box, *prebox;
BOXA      *ba = NULL;
BOXAA     *baa;
NUMA      *nascore, *na = NULL;
NUMAA     *naa;
SARRAY    *satext, *sa = NULL, *saout;

    if (pbaa) *pbaa = NULL;
    if (pnaa) *pnaa = NULL;
    if (!recog || !recog->rcha)
        return (SARRAY *)ERROR_PTR("recog and rcha not both defined",
                                   __func__, NULL);
    if (!boxas)
        return (SARRAY *)ERROR_PTR("boxas not defined", __func__, NULL);

    if (spacethresh < 0)
        spacethresh = L_MAX(recog->maxheight_u, 20);
    rchaExtract(recog->rcha, NULL, &nascore, &satext, NULL, NULL, NULL, NULL);
    if (!nascore || !satext) {
        numaDestroy(&nascore);
        sarrayDestroy(&satext);
        return (SARRAY *)ERROR_PTR("nascore and satext not both returned",
                                   __func__, NULL);
    }

    saout = sarrayCreate(0);
    naa = numaaCreate(0);
    baa = boxaaCreate(0);
    prebox = NULL;
    n = numaGetCount(nascore);
    for (i = 0; i < n; i++) {
        numaGetFValue(nascore, i, &score);
        text = sarrayGetString(satext, i, L_NOCOPY);
        if (prebox == NULL) {  /* no current run */
            if (score < scorethresh) {
                continue;
            } else {  /* start a number run */
                sa = sarrayCreate(0);
                ba = boxaCreate(0);
                na = numaCreate(0);
                sarrayAddString(sa, text, L_COPY);
                prebox = boxaGetBox(boxas, i, L_CLONE);
                boxaAddBox(ba, prebox, L_COPY);
                numaAddNumber(na, score);
            }
        } else {  /* in a current number run */
            box = boxaGetBox(boxas, i, L_CLONE);
            boxGetGeometry(prebox, &x1, NULL, NULL, NULL);
            boxGetGeometry(box, &x2, NULL, NULL, NULL);
            boxOverlapDistance(box, prebox, &h_ovl, &v_ovl);
            h_sep = -h_ovl;
            v_sep = -v_ovl;
            boxDestroy(&prebox);
            if (x1 < x2 && h_sep <= spacethresh &&
                v_sep < 0 && score >= scorethresh) {  /* add to number */
                sarrayAddString(sa, text, L_COPY);
                boxaAddBox(ba, box, L_COPY);
                numaAddNumber(na, score);
                prebox = box;
            } else {  /* save the completed number */
                str = sarrayToString(sa, 0);
                sarrayAddString(saout, str, L_INSERT);
                sarrayDestroy(&sa);
                boxaaAddBoxa(baa, ba, L_INSERT);
                numaaAddNuma(naa, na, L_INSERT);
                boxDestroy(&box);
                if (score >= scorethresh) {  /* start a new number */
                    i--;
                    continue;
                }
            }
        }
    }

    if (prebox) {  /* save the last number */
        str = sarrayToString(sa, 0);
        sarrayAddString(saout, str, L_INSERT);
        boxaaAddBoxa(baa, ba, L_INSERT);
        numaaAddNuma(naa, na, L_INSERT);
        sarrayDestroy(&sa);
        boxDestroy(&prebox);
    }

    numaDestroy(&nascore);
    sarrayDestroy(&satext);
    if (sarrayGetCount(saout) == 0) {
        sarrayDestroy(&saout);
        boxaaDestroy(&baa);
        numaaDestroy(&naa);
        L_INFO("saout has no identified text\n", __func__);
        return NULL;
    }

    if (pbaa)
        *pbaa = baa;
    else
        boxaaDestroy(&baa);
    if (pnaa)
        *pnaa = naa;
    else
        numaaDestroy(&naa);
    return saout;
}

/*!
 * \brief   showExtractNumbers()
 *
 * \param[in]    pixs     input 1 bpp image
 * \param[in]    sa       recognized text strings
 * \param[in]    baa      boxa array for location of characters in each string
 * \param[in]    naa      numa array for scores of characters in each string
 * \param[out]   ppixdb   [optional] input pixs with identified chars outlined
 * \return  pixa   of identified strings with text and scores, or NULL on error
 *
 * <pre>
 * Notes:
 *      (1) This is a debugging routine on digit identification; e.g.:
 *            recogIdentifyMultiple(recog, pixs, 0, 1, &boxa, NULL, NULL, 0);
 *            sa = recogExtractNumbers(recog, boxa, 0.8, -1, &baa, &naa);
 *            pixa = showExtractNumbers(pixs, sa, baa, naa, NULL);
 * </pre>
 */
PIXA *
showExtractNumbers(PIX     *pixs,
                   SARRAY  *sa,
                   BOXAA   *baa,
                   NUMAA   *naa,
                   PIX    **ppixdb)
{
char       buf[128];
char      *textstr, *scorestr;
l_int32    i, j, n, nchar, len;
l_float32  score;
L_BMF     *bmf;
BOX       *box1, *box2;
BOXA      *ba;
NUMA      *na;
PIX       *pix1, *pix2, *pix3, *pix4;
PIXA      *pixa;

    if (ppixdb) *ppixdb = NULL;
    if (!pixs)
        return (PIXA *)ERROR_PTR("pixs not defined", __func__, NULL);
    if (!sa)
        return (PIXA *)ERROR_PTR("sa not defined", __func__, NULL);
    if (!baa)
        return (PIXA *)ERROR_PTR("baa not defined", __func__, NULL);
    if (!naa)
        return (PIXA *)ERROR_PTR("naa not defined", __func__, NULL);

    n = sarrayGetCount(sa);
    pixa = pixaCreate(n);
    bmf = bmfCreate(NULL, 6);
    if (ppixdb) *ppixdb = pixConvertTo8(pixs, 1);
    for (i = 0; i < n; i++) {
        textstr = sarrayGetString(sa, i, L_NOCOPY);
        ba = boxaaGetBoxa(baa, i, L_CLONE);
        na = numaaGetNuma(naa, i, L_CLONE);
        boxaGetExtent(ba, NULL, NULL, &box1);
        box2 = boxAdjustSides(NULL, box1, -5, 5, -5, 5);
        if (ppixdb) pixRenderBoxArb(*ppixdb, box2, 3, 255, 0, 0);
        pix1 = pixClipRectangle(pixs, box1, NULL);
        len = strlen(textstr) + 1;
        pix2 = pixAddBlackOrWhiteBorder(pix1, 14 * len, 14 * len,
                                        5, 3, L_SET_WHITE);
        pix3 = pixConvertTo8(pix2, 1);
        nchar = numaGetCount(na);
        scorestr = NULL;
        for (j = 0; j < nchar; j++) {
             numaGetFValue(na, j, &score);
             snprintf(buf, sizeof(buf), "%d", (l_int32)(100 * score));
             stringJoinIP(&scorestr, buf);
             if (j < nchar - 1) stringJoinIP(&scorestr, ",");
        }
        snprintf(buf, sizeof(buf), "%s: %s\n", textstr, scorestr);
        pix4 = pixAddTextlines(pix3, bmf, buf, 0xff000000, L_ADD_BELOW);
        pixaAddPix(pixa, pix4, L_INSERT);
        boxDestroy(&box1);
        boxDestroy(&box2);
        pixDestroy(&pix1);
        pixDestroy(&pix2);
        pixDestroy(&pix3);
        boxaDestroy(&ba);
        numaDestroy(&na);
        LEPT_FREE(scorestr);
    }

    bmfDestroy(&bmf);
    return pixa;
}


/*------------------------------------------------------------------------*
 *                        Static debug helper                             *
 *------------------------------------------------------------------------*/
/*!
 * \brief   l_showIndicatorSplitValues()
 *
 * \param[in]   na1, na2, na3, na4, na5, na6      6 indicator array
 *
 * <pre>
 * Notes:
 *      (1) The values indicate that specific criteria has been met
 *          for component removal by pre-splitting filter..
 *          The 'result' line shows which components have been removed.
 * </pre>
 */
static void
l_showIndicatorSplitValues(NUMA  *na1,
                           NUMA  *na2,
                           NUMA  *na3,
                           NUMA  *na4,
                           NUMA  *na5,
                           NUMA  *na6)
{
l_int32  i, n;

    n = numaGetCount(na1);
    lept_stderr("================================================\n");
    lept_stderr("lt minw:    ");
    for (i = 0; i < n; i++)
        lept_stderr("%4d ", (l_int32)na1->array[i]);
    lept_stderr("\nlt minh:    ");
    for (i = 0; i < n; i++)
        lept_stderr("%4d ", (l_int32)na2->array[i]);
    lept_stderr("\ngt maxh:    ");
    for (i = 0; i < n; i++)
        lept_stderr("%4d ", (l_int32)na3->array[i]);
    lept_stderr("\ngt maxasp:  ");
    for (i = 0; i < n; i++)
        lept_stderr("%4d ", (l_int32)na4->array[i]);
    lept_stderr("\nlt minaf:   ");
    for (i = 0; i < n; i++)
        lept_stderr("%4d ", (l_int32)na5->array[i]);
    lept_stderr("\n------------------------------------------------");
    lept_stderr("\nresult:     ");
    for (i = 0; i < n; i++)
        lept_stderr("%4d ", (l_int32)na6->array[i]);
    lept_stderr("\n================================================\n");
}
