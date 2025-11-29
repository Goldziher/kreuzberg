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
 * \file recogdid.c
 * <pre>
 *
 *      Top-level identification
 *         BOXA             *recogDecode()
 *
 *      Generate decoding arrays
 *         static l_int32    recogPrepareForDecoding()
 *         static l_int32    recogMakeDecodingArray()
 *
 *      Dynamic programming for best path
 *         static l_int32    recogRunViterbi()
 *         static l_int32    recogRescoreDidResult()
 *         static PIX       *recogShowPath()
 *
 *      Create/destroy temporary DID data
 *         l_int32           recogCreateDid()
 *         l_int32           recogDestroyDid()
 *
 *      Various helpers
 *         l_int32           recogDidExists()
 *         L_RDID           *recogGetDid()
 *         static l_int32    recogGetWindowedArea()
 *         l_int32           recogSetChannelParams()
 *         static l_int32    recogTransferRchToDid()
 *
 *  See recogbasic.c for examples of training a recognizer, which is
 *  required before it can be used for document image decoding.
 *
 *  Gary Kopec pioneered this hidden markov approach to "Document Image
 *  Decoding" (DID) in the early 1990s.  It is based on estimation
 *  using a generative model of the image generation process, and
 *  provides the most likely decoding of an image if the model is correct.
 *  Given the model, it finds the maximum a posteriori (MAP) "message"
 *  given the observed image.  The model describes how to generate
 *  an image from a message, and the MAP message is derived from the
 *  observed image using Bayes' theorem.  This approach can also be used
 *  to build the model, using the iterative expectation/maximization
 *  method from labeled but errorful data.
 *
 *  In a little more detail: The model comprises three things: the ideal
 *  printed character templates, the independent bit-flip noise model, and
 *  the character setwidths.  When a character is printed, the setwidth
 *  is the distance in pixels that you move forward before being able
 *  to print the next character.  It is typically slightly less than the
 *  width of the character template: if too small, an extra character can be
 *  hallucinated; if too large, it will not be able to match the next
 *  character template on the line.  The model assumes that the probabilities
 *  of bit flip depend only on the assignment of the pixel to background
 *  or template foreground.  The multilevel templates have different
 *  bit flip probabilities for each level.  Because a character image
 *  is composed of many pixels, each of which can be independently flipped,
 *  the actual probability of seeing any rendering is exceedingly small,
 *  being composed of the product of the probabilities for each pixel.
 *  The log likelihood is used both to avoid numeric underflow and,
 *  more importantly, because it results in a summation of independent
 *  pixel probabilities.  That summation can be shown, in Kopec's
 *  original paper, to consist of a sum of two terms: (a) the number of
 *  fg pixels in the bit-and of the observed image with the ideal
 *  template and (b) the number of fg pixels in the template.  Each
 *  has a coefficient that depends only on the bit-flip probabilities
 *  for the fg and bg.  A beautiful result, and computationally simple!
 *  One nice feature of this approach is that the result of the decoding
 *  is not very sensitive to the values  used for the bit flip probabilities.
 *
 *  The procedure for finding the best decoding (MAP) for a given image goes
 *  under several names: Viterbi, dynamic programming, hidden markov model.
 *  It is called a "hidden markov model" because the templates are assumed
 *  to be printed serially and we don't know what they are -- the identity
 *  of the templates must be inferred from the observed image.
 *  The possible decodings form a dense trellis over the pixel positions,
 *  where at each pixel position you have the possibility of having any
 *  of the characters printed there (with some reference point) or having
 *  a single pixel wide space inserted there.  Thus, before the trellis
 *  can be traversed, we must do the work of finding the log probability,
 *  at each pixel location, that each of the templates was printed there.
 *  Armed with those arrays of data, the dynamic programming procedure
 *  moves from left to right, one pixel at a time, recursively finding
 *  the path with the highest log probability that gets to that pixel
 *  position (and noting which template was printed to arrive there).
 *  After reaching the right side of the image, we can simply backtrack
 *  along the path, jumping over each template that lies on the highest
 *  scoring path.  This best path thus only goes through a few of the
 *  pixel positions.
 *
 *  There are two refinements to the original Kopec paper.  In the first,
 *  one uses multiple, non-overlapping fg templates, each with its own
 *  bit flip probability.  This makes sense, because the probability
 *  that a fg boundary pixel flips to bg is greater than that of a fg
 *  pixel not on the boundary.  And the flip probability of a fg boundary
 *  pixel is smaller than that of a bg boundary pixel, which in turn
 *  is greater than that of a bg pixel not on a boundary (the latter
 *  is taken to be the true background).  Then the simplest realistic
 *  multiple template model has three templates that are not background.
 *
 *  In the second refinement, a heuristic (strict upper bound) is used
 *  iteratively in the Viterbi process to compute the log probabilities.
 *  Using the heuristic, you find the best path, and then score all nodes
 *  on that path with the actual probability, which is guaranteed to
 *  be a smaller number.  You run this iteratively, rescoring just the best
 *  found path each time.  After each rescoring, the path may change because
 *  the local scores have been reduced.  However, the process converges
 *  rapidly, and when it doesn't change, it must be the best path because
 *  it is properly scored (even if neighboring paths are heuristically
 *  scored).  The heuristic score is found column-wise by assuming
 *  that all the fg pixels in the template are on fg pixels in the image --
 *  we just take the minimum of the number of pixels in the template
 *  and image column.  This can easily give a 10-fold reduction in
 *  computation because the heuristic score can be computed much faster
 *  than the exact score.
 *
 *  For reference, the classic paper on the approach by Kopec is:
 *  * "Document Image Decoding Using Markov Source Models", IEEE Trans.
 *    PAMI, Vol 16, No. 6, June 1994, pp 602-617.
 *  A refinement of the method for multilevel templates by Kopec is:
 *  * "Multilevel Character Templates for Document Image Decoding",
 *    Proc. SPIE 3027, Document Recognition IV, p. 168ff, 1997.
 *  Further refinements for more efficient decoding are given in these
 *  two papers, which are both stored on leptonica.org:
 *  * "Document Image Decoding using Iterated Complete Path Search", Minka,
 *    Bloomberg and Popat, Proc. SPIE Vol 4307, p. 250-258, Document
 *    Recognition and Retrieval VIII, San Jose, CA 2001.
 *  * "Document Image Decoding using Iterated Complete Path Search with
 *    Subsampled Heuristic Scoring", Bloomberg, Minka and Popat, ICDAR 2001,
 *    p. 344-349, Sept. 2001, Seattle.
 * </pre>
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include <string.h>
#include <math.h>
#include "allheaders.h"

static l_int32 recogPrepareForDecoding(L_RECOG *recog, PIX *pixs,
                                       l_int32 debug);
static l_int32 recogMakeDecodingArray(L_RECOG *recog, l_int32 index,
                                      l_int32 debug);
static l_int32 recogRunViterbi(L_RECOG *recog, PIX **ppixdb);
static l_int32 recogRescoreDidResult(L_RECOG *recog, PIX **ppixdb);
static PIX *recogShowPath(L_RECOG *recog, l_int32 select);
static l_int32 recogGetWindowedArea(L_RECOG *recog, l_int32 index,
                                    l_int32 x, l_int32 *pdely, l_int32 *pwsum);
static l_int32 recogTransferRchToDid(L_RECOG *recog, l_int32 x, l_int32 y);

    /* Parameters for modeling the decoding */
static const l_float32  SetwidthFraction = 0.95f;
static const l_int32    MaxYShift = 1;

    /* Channel parameters.  alpha[0] is the probability that a bg pixel
     * is OFF.  alpha[1] is the probability that level 1 fg is ON.
     * The actual values are not too critical, but they must be larger
     * than 0.5 and smaller than 1.0.  For more accuracy in template
     * matching, use a 4-level template, where levels 2 and 3 are
     * boundary pixels in the fg and bg, respectively. */
static const l_float32  DefaultAlpha2[] = {0.95f, 0.9f};
static const l_float32  DefaultAlpha4[] = {0.95f, 0.9f, 0.75f, 0.25f};


/*------------------------------------------------------------------------*
 *                       Top-level identification                         *
 *------------------------------------------------------------------------*/
/*!
 * \brief   recogDecode()
 *
 * \param[in]    recog      with LUT's pre-computed
 * \param[in]    pixs       typically of multiple touching characters, 1 bpp
 * \param[in]    nlevels    of templates; 2 for now
 * \param[out]   ppixdb     [optional] debug result; can be null
 * \return  boxa  segmentation of pixs into characters, or NULL on error
 *
 * <pre>
 * Notes:
 *      (1) The input pixs has been filtered so that it is likely to be
 *          composed of more than one touching character.  Specifically,
 *          its height can only slightly exceed that of the tallest
 *          unscaled template, the width is somewhat larger than the
 *          width of the widest unscaled template, and the w/h aspect ratio
 *          is bounded by max_wh_ratio.
 *      (2) This uses the DID mechanism with labeled templates to
 *          segment the input %pixs.  The resulting segmentation is
 *          returned.  (It is given by did->boxa).
 *      (3) In debug mode, the Viterbi path is rescored based on all
 *          the templates.  In non-debug mode, the same procedure is
 *          carried out by recogIdentifyPix() on the result of the
 *          segmentation.
 * </pre>
 */
BOXA  *
recogDecode(L_RECOG  *recog,
            PIX      *pixs,
            l_int32   nlevels,
            PIX     **ppixdb)
{
l_int32  debug;
PIX     *pix1;
PIXA    *pixa;

    if (ppixdb) *ppixdb = NULL;
    if (!recog)
        return (BOXA *)ERROR_PTR("recog not defined", __func__, NULL);
    if (!pixs || pixGetDepth(pixs) != 1)
        return (BOXA *)ERROR_PTR("pixs undefined or not 1 bpp", __func__, NULL);
    if (!recog->train_done)
        return (BOXA *)ERROR_PTR("training not finished", __func__, NULL);
    if (nlevels != 2)
        return (BOXA *)ERROR_PTR("nlevels != 2 (for now)", __func__, NULL);

    debug = (ppixdb) ? 1 : 0;
    if (recogPrepareForDecoding(recog, pixs, debug))
        return (BOXA *)ERROR_PTR("error making arrays", __func__, NULL);
    recogSetChannelParams(recog, nlevels);

        /* Normal path; just run Viterbi */
    if (!debug) {
        if (recogRunViterbi(recog, NULL) == 0)
            return boxaCopy(recog->did->boxa, L_COPY);
        else
            return (BOXA *)ERROR_PTR("error in Viterbi", __func__, NULL);
    }

        /* Debug path */
    if (recogRunViterbi(recog, &pix1))
        return (BOXA *)ERROR_PTR("error in viterbi", __func__, NULL);
    pixa = pixaCreate(2);
    pixaAddPix(pixa, pix1, L_INSERT);
    if (recogRescoreDidResult(recog, &pix1)) {
        pixaDestroy(&pixa);
        return (BOXA *)ERROR_PTR("error in rescoring", __func__, NULL);
    }
    pixaAddPix(pixa, pix1, L_INSERT);
    *ppixdb = pixaDisplayTiledInRows(pixa, 32, 2 * pixGetWidth(pix1) + 100,
                                     1.0, 0, 30, 2);
    pixaDestroy(&pixa);
    return boxaCopy(recog->did->boxa, L_COPY);
}


/*------------------------------------------------------------------------*
 *                       Generate decoding arrays                         *
 *------------------------------------------------------------------------*/
/*!
 * \brief   recogPrepareForDecoding()
 *
 * \param[in]    recog    with LUT's pre-computed
 * \param[in]    pixs     typically of multiple touching characters, 1 bpp
 * \param[in]    debug    1 for debug output; 0 otherwise
 * \return  0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *      (1) Binarizes and crops input %pixs.
 *      (2) Removes previous L_RDID struct and makes a new one.
 *      (3) Generates the bit-and sum arrays for each character template
 *          at each pixel position in %pixs.  These are used in the
 *          Viterbi dynamic programming step.
 *      (4) The values are saved in the scoring arrays at the left edge
 *          of the template.  They are used in the Viterbi process
 *          at the setwidth position (which is near the RHS of the template
 *          as it is positioned on pixs) in the generated trellis.
 * </pre>
 */
static l_int32
recogPrepareForDecoding(L_RECOG  *recog,
                        PIX      *pixs,
                        l_int32   debug)
{
l_int32  i, ret;
PIX     *pix1;
L_RDID  *did;

    if (!recog)
        return ERROR_INT("recog not defined", __func__, 1);
    if (!pixs || pixGetDepth(pixs) != 1)
        return ERROR_INT("pixs not defined or not 1 bpp", __func__, 1);
    if (!recog->train_done)
        return ERROR_INT("training not finished", __func__, 1);

    if (!recog->ave_done) {
        ret = recogAverageSamples(recog, 0);
        if (!ret)
            return ERROR_INT("averaging of samples failed", __func__, 1);
    }

        /* Binarize and crop to foreground if necessary */
    if ((pix1 = recogProcessToIdentify(recog, pixs, 0)) == NULL)
        return ERROR_INT("pix1 not made", __func__, 1);

        /* Remove any existing RecogDID and set up a new one */
    recogDestroyDid(recog);
    if (recogCreateDid(recog, pix1)) {
        pixDestroy(&pix1);
        return ERROR_INT("decoder not made", __func__, 1);
    }

        /* Compute vertical sum and first moment arrays */
    did = recogGetDid(recog);  /* owned by recog */
    did->nasum = pixCountPixelsByColumn(pix1);
    did->namoment = pixGetMomentByColumn(pix1, 1);

        /* Generate the arrays */
    for (i = 0; i < recog->did->narray; i++)
        recogMakeDecodingArray(recog, i, debug);

    pixDestroy(&pix1);
    return 0;
}


/*!
 * \brief   recogMakeDecodingArray()
 *
 * \param[in]    recog
 * \param[in]    index     of averaged template
 * \param[in]    debug     1 for debug output; 0 otherwise
 * \return  0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *      (1) Generates the bit-and sum array for a character template along pixs.
 *      (2) The values are saved in the scoring arrays at the left edge
 *          of the template as it is positioned on pixs.
 * </pre>
 */
static l_int32
recogMakeDecodingArray(L_RECOG  *recog,
                       l_int32   index,
                       l_int32   debug)
{
l_int32   i, j, w1, h1, w2, h2, nx, ycent2, count, maxcount, maxdely;
l_int32   sum, moment, dely, shifty;
l_int32  *counta, *delya, *ycent1, *arraysum, *arraymoment, *sumtab;
NUMA     *nasum, *namoment;
PIX      *pix1, *pix2, *pix3;
L_RDID   *did;

    if (!recog)
        return ERROR_INT("recog not defined", __func__, 1);
    if ((did = recogGetDid(recog)) == NULL)
        return ERROR_INT("did not defined", __func__, 1);
    if (index < 0 || index >= did->narray)
        return ERROR_INT("invalid index", __func__, 1);

        /* Check that pix1 is large enough for this template. */
    pix1 = did->pixs;  /* owned by did; do not destroy */
    pixGetDimensions(pix1, &w1, &h1, NULL);
    pix2 = pixaGetPix(recog->pixa_u, index, L_CLONE);
    pixGetDimensions(pix2, &w2, &h2, NULL);
    if (w1 < w2) {
        L_INFO("w1 = %d < w2 = %d for index %d\n", __func__, w1, w2, index);
        pixDestroy(&pix2);
        return 0;
    }

    nasum = did->nasum;
    namoment = did->namoment;
    ptaGetIPt(recog->pta_u, index, NULL, &ycent2);
    sumtab = recog->sumtab;
    counta = did->counta[index];
    delya = did->delya[index];

        /* Set up the array for ycent1.  This gives the y-centroid location
         * for a window of width w2, starting at location i. */
    nx = w1 - w2 + 1;  /* number of positions w2 can be placed in w1 */
    ycent1 = (l_int32 *)LEPT_CALLOC(nx, sizeof(l_int32));
    arraysum = numaGetIArray(nasum);
    arraymoment = numaGetIArray(namoment);
    for (i = 0, sum = 0, moment = 0; i < w2; i++) {
        sum += arraysum[i];
        moment += arraymoment[i];
    }
    for (i = 0; i < nx - 1; i++) {
        ycent1[i] = (sum == 0) ? ycent2 : (l_float32)moment / (l_float32)sum;
        sum += arraysum[w2 + i] - arraysum[i];
        moment += arraymoment[w2 + i] - arraymoment[i];
    }
    ycent1[nx - 1] = (sum == 0) ? ycent2 : (l_float32)moment / (l_float32)sum;

        /* Compute the bit-and sum between the template pix2 and pix1, at
         * locations where the left side of pix2 goes from 0 to nx - 1
         * in pix1.  Do this around the vertical alignment of the pix2
         * centroid and the windowed pix1 centroid.
         *  (1) Start with pix3 cleared and approximately equal in size to pix1.
         *  (2) Blit the y-shifted pix2 onto pix3.  Then all ON pixels
         *      are within the intersection of pix1 and the shifted pix2.
         *  (3) AND pix1 with pix3. */
    pix3 = pixCreate(w2, h1, 1);
    for (i = 0; i < nx; i++) {
        shifty = (l_int32)(ycent1[i] - ycent2 + 0.5);
        maxcount = 0;
        maxdely = 0;
        for (j = -MaxYShift; j <= MaxYShift; j++) {
            pixClearAll(pix3);
            dely = shifty + j;  /* amount pix2 is shifted relative to pix1 */
            pixRasterop(pix3, 0, dely, w2, h2, PIX_SRC, pix2, 0, 0);
            pixRasterop(pix3, 0, 0, w2, h1, PIX_SRC & PIX_DST, pix1, i, 0);
            pixCountPixels(pix3, &count, sumtab);
            if (count > maxcount) {
                maxcount = count;
                maxdely = dely;
            }
        }
        counta[i] = maxcount;
        delya[i] = maxdely;
    }
    did->fullarrays = TRUE;

    pixDestroy(&pix2);
    pixDestroy(&pix3);
    LEPT_FREE(ycent1);
    LEPT_FREE(arraysum);
    LEPT_FREE(arraymoment);
    return 0;
}


/*------------------------------------------------------------------------*
 *                  Dynamic programming for best path
 *------------------------------------------------------------------------*/
/*!
 * \brief   recogRunViterbi()
 *
 * \param[in]    recog     with LUT's pre-computed
 * \param[out]   ppixdb    [optional] debug result; can be null
 * \return  0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *      (1) This can be used when the templates are unscaled.  It works by
 *          matching the average, unscaled templates of each class to
 *          all positions.
 *      (2) It is recursive, in that
 *          (a) we compute the score successively at all pixel positions x,
 *          (b) to compute the score at x in the trellis, for each
 *              template we look backwards to (x - setwidth) to get the
 *              score if that template were to be printed with its
 *              setwidth location at x.  We save at x the template and
 *              score that maximizes the sum of the score at (x - setwidth)
 *              and the log-likelihood for the template to be printed with
 *              its LHS there.
 *      (3) The primary output is a boxa of the locations for splitting
 *          the input image.  These locations are used later to split the
 *          image and send the pieces individually for recognition.
 *          This can be done in either recogIdentifyMultiple(), or
 *          for debugging in recogRescoreDidResult().
 * </pre>
 */
static l_int32
recogRunViterbi(L_RECOG  *recog,
                PIX     **ppixdb)
{
l_int32     i, w1, w2, h1, xnz, x, narray, minsetw;
l_int32     first, templ, xloc, dely, counts, area1;
l_int32     besttempl, spacetempl;
l_int32    *setw, *didtempl;
l_int32    *area2;  /* must be freed */
l_float32   prevscore, matchscore, maxscore, correl;
l_float32  *didscore;
BOX        *box;
PIX        *pix1;
L_RDID     *did;

    if (ppixdb) *ppixdb = NULL;
    if (!recog)
        return ERROR_INT("recog not defined", __func__, 1);
    if ((did = recogGetDid(recog)) == NULL)
        return ERROR_INT("did not defined", __func__, 1);
    if (did->fullarrays == 0)
        return ERROR_INT("did full arrays not made", __func__, 1);

        /* Compute the minimum setwidth. Bad templates with very small
         * width can cause havoc because the setwidth is too small. */
    w1 = did->size;
    narray = did->narray;
    spacetempl = narray;
    setw = did->setwidth;
    minsetw = 100000;
    for (i = 0; i < narray; i++) {
        if (setw[i] < minsetw)
            minsetw = setw[i];
    }
    if (minsetw <= 2)
        return ERROR_INT("minsetw <= 2; bad templates", __func__, 1);

        /* The score array is initialized to 0.0.  As we proceed to
         * the left, the log likelihood for the partial paths goes
         * negative, and we prune for the max (least negative) path.
         * No matches will be computed until we reach x = min(setwidth);
         * until then first == TRUE after looping over templates. */
    didscore = did->trellisscore;
    didtempl = did->trellistempl;
    area2 = numaGetIArray(recog->nasum_u);
    besttempl = 0;  /* just tells compiler it is initialized */
    maxscore = 0.0;  /* ditto */
    for (x = minsetw; x < w1; x++) {  /* will always get a score */
        first = TRUE;
        for (i = 0; i < narray; i++) {
            if (x - setw[i] < 0) continue;
            matchscore = didscore[x - setw[i]] +
                         did->gamma[1] * did->counta[i][x - setw[i]] +
                         did->beta[1] * area2[i];
            if (first) {
                maxscore = matchscore;
                besttempl = i;
                first = FALSE;
            } else {
                if (matchscore > maxscore) {
                    maxscore = matchscore;
                    besttempl = i;
                }
            }
        }

            /* We can also put down a single pixel space, with no cost
             * because all pixels are bg. */
        prevscore = didscore[x - 1];
        if (prevscore > maxscore) {  /* 1 pixel space is best */
            maxscore = prevscore;
            besttempl = spacetempl;
        }
        didscore[x] = maxscore;
        didtempl[x] = besttempl;
    }

        /* Backtrack to get the best path.
         * Skip over (i.e., ignore) all single pixel spaces. */
    for (x = w1 - 1; x >= 0; x--) {
        if (didtempl[x] != spacetempl) break;
    }
    h1 = pixGetHeight(did->pixs);
    while (x > 0) {
        if (didtempl[x] == spacetempl) {  /* skip over spaces */
            x--;
            continue;
        }
        templ = didtempl[x];
        xloc = x - setw[templ];
        if (xloc < 0) break;
        counts = did->counta[templ][xloc];  /* bit-and counts */
        recogGetWindowedArea(recog, templ, xloc, &dely, &area1);
        correl = ((l_float32)(counts) * counts) /
                  (l_float32)(area2[templ] * area1);
        pix1 = pixaGetPix(recog->pixa_u, templ, L_CLONE);
        w2 = pixGetWidth(pix1);
        numaAddNumber(did->natempl, templ);
        numaAddNumber(did->naxloc, xloc);
        numaAddNumber(did->nadely, dely);
        numaAddNumber(did->nawidth, pixGetWidth(pix1));
        numaAddNumber(did->nascore, correl);
        xnz = L_MAX(xloc, 0);
        box = boxCreate(xnz, dely, w2, h1);
        boxaAddBox(did->boxa, box, L_INSERT);
        pixDestroy(&pix1);
        x = xloc;
    }

    if (ppixdb) {
        numaWriteStderr(did->natempl);
        numaWriteStderr(did->naxloc);
        numaWriteStderr(did->nadely);
        numaWriteStderr(did->nawidth);
        numaWriteStderr(did->nascore);
        boxaWriteStderr(did->boxa);
        *ppixdb = recogShowPath(recog, 0);
    }

    LEPT_FREE(area2);
    return 0;
}


/*!
 * \brief   recogRescoreDidResult()
 *
 * \param[in]    recog     with LUT's pre-computed
 * \param[out]   ppixdb    [optional] debug result; can be null
 * \return  0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *      (1) This does correlation matching with all unscaled templates,
 *          using the character segmentation determined by the Viterbi path.
 * </pre>
 */
static l_int32
recogRescoreDidResult(L_RECOG  *recog,
                      PIX     **ppixdb)
{
l_int32    i, n, sample, x, dely, index;
char      *text = NULL;
l_float32  score;
BOX       *box1;
PIX       *pixs, *pix1;
L_RDID    *did;

    if (ppixdb) *ppixdb = NULL;
    if (!recog)
        return ERROR_INT("recog not defined", __func__, 1);
    if ((did = recogGetDid(recog)) == NULL)
        return ERROR_INT("did not defined", __func__, 1);
    if (did->fullarrays == 0)
        return ERROR_INT("did full arrays not made", __func__, 1);
    if ((n = numaGetCount(did->naxloc)) == 0)
        return ERROR_INT("no elements in path", __func__, 1);

    pixs = did->pixs;
    for (i = 0; i < n; i++) {
        box1 = boxaGetBox(did->boxa, i, L_COPY);
        boxGetGeometry(box1, &x, &dely, NULL, NULL);
        pix1 = pixClipRectangle(pixs, box1, NULL);
        recogIdentifyPix(recog, pix1, NULL);
        recogTransferRchToDid(recog, x, dely);
        if (ppixdb) {
            rchExtract(recog->rch, &index, &score, &text,
                       &sample, NULL, NULL, NULL);
            lept_stderr("text = %s, index = %d, sample = %d,"
                        " score = %5.3f\n", text, index, sample, score);
        }
        pixDestroy(&pix1);
        boxDestroy(&box1);
        LEPT_FREE(text);
    }

    if (ppixdb)
        *ppixdb = recogShowPath(recog, 1);

    return 0;
}


/*!
 * \brief   recogShowPath()
 *
 * \param[in]    recog     with LUT's pre-computed
 * \param[in]    select    0 for Viterbi; 1 for rescored
 * \return  pix debug output), or NULL on error
 */
static PIX *
recogShowPath(L_RECOG  *recog,
              l_int32   select)
{
char       textstr[16];
l_int32    i, j, n, index, xloc, dely;
l_float32  score;
L_BMF     *bmf;
NUMA      *natempl_s, *nasample_s = NULL, *nascore_s, *naxloc_s, *nadely_s;
PIX       *pixs, *pix0, *pix1, *pix2, *pix3, *pix4, *pix5;
L_RDID    *did;

    if (!recog)
        return (PIX *)ERROR_PTR("recog not defined", __func__, NULL);
    if ((did = recogGetDid(recog)) == NULL)
        return (PIX *)ERROR_PTR("did not defined", __func__, NULL);

    bmf = bmfCreate(NULL, 8);
    pixs = pixScale(did->pixs, 4.0, 4.0);
    pix0 = pixAddBorderGeneral(pixs, 0, 0, 0, 40, 0);
    pix1 = pixConvertTo32(pix0);
    if (select == 0) {  /* Viterbi */
        natempl_s = did->natempl;
        nascore_s = did->nascore;
        naxloc_s = did->naxloc;
        nadely_s = did->nadely;
    } else {  /* rescored */
        natempl_s = did->natempl_r;
        nasample_s = did->nasample_r;
        nascore_s = did->nascore_r;
        naxloc_s = did->naxloc_r;
        nadely_s = did->nadely_r;
    }

    n = numaGetCount(natempl_s);
    for (i = 0; i < n; i++) {
        numaGetIValue(natempl_s, i, &index);
        if (select == 0) {
            pix2 = pixaGetPix(recog->pixa_u, index, L_CLONE);
        } else {
            numaGetIValue(nasample_s, i, &j);
            pix2 = pixaaGetPix(recog->pixaa_u, index, j, L_CLONE);
        }
        pix3 = pixScale(pix2, 4.0, 4.0);
        pix4 = pixErodeBrick(NULL, pix3, 5, 5);
        pixXor(pix4, pix4, pix3);
        numaGetFValue(nascore_s, i, &score);
        snprintf(textstr, sizeof(textstr), "%5.3f", score);
        pix5 = pixAddTextlines(pix4, bmf, textstr, 1, L_ADD_BELOW);
        numaGetIValue(naxloc_s, i, &xloc);
        numaGetIValue(nadely_s, i, &dely);
        pixPaintThroughMask(pix1, pix5, 4 * xloc, 4 * dely, 0xff000000);
        pixDestroy(&pix2);
        pixDestroy(&pix3);
        pixDestroy(&pix4);
        pixDestroy(&pix5);
    }
    pixDestroy(&pixs);
    pixDestroy(&pix0);
    bmfDestroy(&bmf);
    return pix1;
}


/*------------------------------------------------------------------------*
 *                  Create/destroy temporary DID data                     *
 *------------------------------------------------------------------------*/
/*!
 * \brief   recogCreateDid()
 *
 * \param[in]    recog
 * \param[in]    pixs     of 1 bpp image to match
 * \return  0 if OK, 1 on error
 */
l_ok
recogCreateDid(L_RECOG  *recog,
               PIX      *pixs)
{
l_int32  i;
PIX     *pix1;
L_RDID  *did;

    if (!recog)
        return ERROR_INT("recog not defined", __func__, 1);
    if (!pixs)
        return ERROR_INT("pixs not defined", __func__, 1);

    recogDestroyDid(recog);

    did = (L_RDID *)LEPT_CALLOC(1, sizeof(L_RDID));
    recog->did = did;
    did->pixs = pixClone(pixs);
    did->narray = recog->setsize;
    did->size = pixGetWidth(pixs);
    did->natempl = numaCreate(5);
    did->naxloc = numaCreate(5);
    did->nadely = numaCreate(5);
    did->nawidth = numaCreate(5);
    did->boxa = boxaCreate(5);
    did->nascore = numaCreate(5);
    did->natempl_r = numaCreate(5);
    did->nasample_r = numaCreate(5);
    did->naxloc_r = numaCreate(5);
    did->nadely_r = numaCreate(5);
    did->nawidth_r = numaCreate(5);
    did->nascore_r = numaCreate(5);

        /* Make the arrays */
    did->setwidth = (l_int32 *)LEPT_CALLOC(did->narray, sizeof(l_int32));
    did->counta = (l_int32 **)LEPT_CALLOC(did->narray, sizeof(l_int32 *));
    did->delya = (l_int32 **)LEPT_CALLOC(did->narray, sizeof(l_int32 *));
    did->beta = (l_float32 *)LEPT_CALLOC(5, sizeof(l_float32));
    did->gamma = (l_float32 *)LEPT_CALLOC(5, sizeof(l_float32));
    did->trellisscore = (l_float32 *)LEPT_CALLOC(did->size, sizeof(l_float32));
    did->trellistempl = (l_int32 *)LEPT_CALLOC(did->size, sizeof(l_int32));
    for (i = 0; i < did->narray; i++) {
        did->counta[i] = (l_int32 *)LEPT_CALLOC(did->size, sizeof(l_int32));
        did->delya[i] = (l_int32 *)LEPT_CALLOC(did->size, sizeof(l_int32));
    }

        /* Populate the setwidth array */
    for (i = 0; i < did->narray; i++) {
        pix1 = pixaGetPix(recog->pixa_u, i, L_CLONE);
        did->setwidth[i] = (l_int32)(SetwidthFraction * pixGetWidth(pix1));
        pixDestroy(&pix1);
    }

    return 0;
}


/*!
 * \brief   recogDestroyDid()
 *
 * \param[in]    recog
 * \return  0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *      (1) As the signature indicates, this is owned by the recog, and can
 *          only be destroyed using this function.
 * </pre>
 */
l_ok
recogDestroyDid(L_RECOG  *recog)
{
l_int32  i;
L_RDID  *did;

    if (!recog)
        return ERROR_INT("recog not defined", __func__, 1);

    if ((did = recog->did) == NULL) return 0;
    if (!did->counta || !did->delya)
        return ERROR_INT("ptr array is null; shouldn't happen!", __func__, 1);

    for (i = 0; i < did->narray; i++) {
        LEPT_FREE(did->counta[i]);
        LEPT_FREE(did->delya[i]);
    }
    LEPT_FREE(did->setwidth);
    LEPT_FREE(did->counta);
    LEPT_FREE(did->delya);
    LEPT_FREE(did->beta);
    LEPT_FREE(did->gamma);
    LEPT_FREE(did->trellisscore);
    LEPT_FREE(did->trellistempl);
    pixDestroy(&did->pixs);
    numaDestroy(&did->nasum);
    numaDestroy(&did->namoment);
    numaDestroy(&did->natempl);
    numaDestroy(&did->naxloc);
    numaDestroy(&did->nadely);
    numaDestroy(&did->nawidth);
    boxaDestroy(&did->boxa);
    numaDestroy(&did->nascore);
    numaDestroy(&did->natempl_r);
    numaDestroy(&did->nasample_r);
    numaDestroy(&did->naxloc_r);
    numaDestroy(&did->nadely_r);
    numaDestroy(&did->nawidth_r);
    numaDestroy(&did->nascore_r);
    LEPT_FREE(did);
    recog->did = NULL;
    return 0;
}


/*------------------------------------------------------------------------*
 *                            Various helpers                             *
 *------------------------------------------------------------------------*/
/*!
 * \brief   recogDidExists()
 *
 * \param[in]    recog
 * \return  1 if recog->did exists; 0 if not or on error.
 */
l_int32
recogDidExists(L_RECOG  *recog)
{
    if (!recog)
        return ERROR_INT("recog not defined", __func__, 0);
    return (recog->did) ? 1 : 0;
}


/*!
 * \brief   recogGetDid()
 *
 * \param[in]    recog
 * \return  did still owned by the recog, or NULL on error
 *
 * <pre>
 * Notes:
 *      (1) This also makes sure the arrays are defined.
 * </pre>
 */
L_RDID *
recogGetDid(L_RECOG  *recog)
{
l_int32  i;
L_RDID  *did;

    if (!recog)
        return (L_RDID *)ERROR_PTR("recog not defined", __func__, NULL);
    if ((did = recog->did) == NULL)
        return (L_RDID *)ERROR_PTR("did not defined", __func__, NULL);
    if (!did->counta || !did->delya)
        return (L_RDID *)ERROR_PTR("did array ptrs not defined",
                                   __func__, NULL);
    for (i = 0; i < did->narray; i++) {
        if (!did->counta[i] || !did->delya[i])
            return (L_RDID *)ERROR_PTR("did arrays not defined",
                                       __func__, NULL);
    }

    return did;
}


/*!
 * \brief   recogGetWindowedArea()
 *
 * \param[in]    recog
 * \param[in]    index     of template
 * \param[in]    x pixel   position of left hand edge of template
 * \param[out]   pdely     y shift of template relative to pix1
 * \param[out]   pwsum     number of fg pixels in window of pixs
 * \return  0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *      (1) This is called after the best path has been found through
 *          the trellis, in order to produce a correlation that can be used
 *          to evaluate the confidence we have in the identification.
 *          The correlation is |1 & 2|^2 / (|1| * |2|).
 *          |1 & 2| is given by the count array, |2| is found from
 *          nasum_u[], and |1| is wsum returned from this function.
 * </pre>
 */
static l_int32
recogGetWindowedArea(L_RECOG  *recog,
                     l_int32   index,
                     l_int32   x,
                     l_int32  *pdely,
                     l_int32  *pwsum)
{
l_int32  w1, h1, w2, h2;
PIX     *pix1, *pix2, *pixt;
L_RDID  *did;

    if (pdely) *pdely = 0;
    if (pwsum) *pwsum = 0;
    if (!pdely || !pwsum)
        return ERROR_INT("&dely and &wsum not both defined", __func__, 1);
    if (!recog)
        return ERROR_INT("recog not defined", __func__, 1);
    if ((did = recogGetDid(recog)) == NULL)
        return ERROR_INT("did not defined", __func__, 1);
    if (index < 0 || index >= did->narray)
        return ERROR_INT("invalid index", __func__, 1);
    pix1 = did->pixs;
    pixGetDimensions(pix1, &w1, &h1, NULL);
    if (x >= w1)
        return ERROR_INT("invalid x position", __func__, 1);

    pix2 = pixaGetPix(recog->pixa_u, index, L_CLONE);
    pixGetDimensions(pix2, &w2, &h2, NULL);
    if (w1 < w2) {
        L_INFO("template %d too small\n", __func__, index);
        pixDestroy(&pix2);
        return 0;
    }

    *pdely = did->delya[index][x];
    pixt = pixCreate(w2, h1, 1);
    pixRasterop(pixt, 0, *pdely, w2, h2, PIX_SRC, pix2, 0, 0);
    pixRasterop(pixt, 0, 0, w2, h1, PIX_SRC & PIX_DST, pix1, x, 0);
    pixCountPixels(pixt, pwsum, recog->sumtab);
    pixDestroy(&pix2);
    pixDestroy(&pixt);
    return 0;
}


/*!
 * \brief   recogSetChannelParams()
 *
 * \param[in]    recog
 * \param[in]    nlevels
 * \return  0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *      (1) This converts the independent bit-flip probabilities in the
 *          "channel" into log-likelihood coefficients on image sums.
 *          These coefficients are only defined for the non-background
 *          template levels.  Thus for nlevels = 2 (one fg, one bg),
 *          only beta[1] and gamma[1] are used.  For nlevels = 4 (three
 *          fg templates), we use beta[1-3] and gamma[1-3].
 * </pre>
 */
l_ok
recogSetChannelParams(L_RECOG  *recog,
                      l_int32   nlevels)
{
l_int32           i;
const l_float32  *da;
L_RDID           *did;

    if (!recog)
        return ERROR_INT("recog not defined", __func__, 1);
    if ((did = recogGetDid(recog)) == NULL)
        return ERROR_INT("did not defined", __func__, 1);
    if (nlevels == 2)
        da = DefaultAlpha2;
    else if (nlevels == 4)
        da = DefaultAlpha4;
    else
        return ERROR_INT("nlevels not 2 or 4", __func__, 1);

    for (i = 1; i < nlevels; i++) {
        did->beta[i] = log((1.0 - da[i]) / da[0]);
        did->gamma[i] = log(da[0] * da[i] / ((1.0 - da[0]) * (1.0 - da[i])));
/*        lept_stderr("beta[%d] = %7.3f, gamma[%d] = %7.3f\n",
                      i, did->beta[i], i, did->gamma[i]);  */
    }

    return 0;
}


/*!
 * \brief   recogTransferRchToDid()
 *
 * \param[in]    recog   with rch and did defined
 * \param[in]    x       left edge of extracted region, relative to decoded line
 * \param[in]    y       top edge of extracted region, relative to input image
 * \return  0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *      (1) This is used to transfer the results for a single character match
 *          to the rescored did arrays.
 * </pre>
 */
static l_int32
recogTransferRchToDid(L_RECOG  *recog,
                      l_int32   x,
                      l_int32   y)
{
L_RDID  *did;
L_RCH   *rch;

    if (!recog)
        return ERROR_INT("recog not defined", __func__, 1);
    if ((did = recogGetDid(recog)) == NULL)
        return ERROR_INT("did not defined", __func__, 1);
    if ((rch = recog->rch) == NULL)
        return ERROR_INT("rch not defined", __func__, 1);

    numaAddNumber(did->natempl_r, rch->index);
    numaAddNumber(did->nasample_r, rch->sample);
    numaAddNumber(did->naxloc_r, rch->xloc + x);
    numaAddNumber(did->nadely_r, rch->yloc + y);
    numaAddNumber(did->nawidth_r, rch->width);
    numaAddNumber(did->nascore_r, rch->score);
    return 0;
}
