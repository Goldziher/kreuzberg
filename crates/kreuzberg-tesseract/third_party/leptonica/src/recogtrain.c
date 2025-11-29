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
 * \file recogtrain.c
 * <pre>
 *
 *      Training on labeled data
 *         l_int32             recogTrainLabeled()
 *         PIX                *recogProcessLabeled()
 *         l_int32             recogAddSample()
 *         PIX                *recogModifyTemplate()
 *         l_int32             recogAverageSamples()
 *         l_int32             pixaAccumulateSamples()
 *         l_int32             recogTrainingFinished()
 *         static l_int32      recogTemplatesAreOK()
 *         PIXA               *recogFilterPixaBySize()
 *         PIXAA              *recogSortPixaByClass()
 *         l_int32             recogRemoveOutliers1()
 *         PIXA               *pixaRemoveOutliers1()
 *         l_int32             recogRemoveOutliers2()
 *         PIXA               *pixaRemoveOutliers2()
 *
 *      Training on unlabeled data
 *         L_RECOG             recogTrainFromBoot()
 *
 *      Padding the digit training set
 *         l_int32             recogPadDigitTrainingSet()
 *         l_int32             recogIsPaddingNeeded()
 *         static SARRAY      *recogAddMissingClassStrings()
 *         PIXA               *recogAddDigitPadTemplates()
 *         static l_int32      recogCharsetAvailable()
 *
 *      Making a boot digit recognizer
 *         L_RECOG            *recogMakeBootDigitRecog()
 *         PIXA               *recogMakeBootDigitTemplates()
 *
 *      Debugging
 *         l_int32             recogShowContent()
 *         l_int32             recogDebugAverages()
 *         l_int32             recogShowAverageTemplates()
 *         static PIX         *pixDisplayOutliers()
 *         PIX                *recogDisplayOutlier()
 *         PIX                *recogShowMatchesInRange()
 *         PIX                *recogShowMatch()
 *
 *  These abbreviations are for the type of template to be used:
 *    * SI (for the scanned images)
 *    * WNL (for width-normalized lines, formed by first skeletonizing
 *           the scanned images, and then dilating to a fixed width)
 *  These abbreviations are for the type of recognizer:
 *    * BAR (book-adapted recognizer; the best type; can do identification
 *           with unscaled images and separation of touching characters.
 *    * BSR (bootstrap recognizer; used if more labeled templates are
 *           required for a BAR, either for finding more templates from
 *           the book, or making a hybrid BAR/BSR.
 *
 *  The recog struct typically holds two versions of the input templates
 *  (e.g. from a pixa) that were used to generate it.  One version is
 *  the unscaled input templates.  The other version is the one that
 *  will be used by the recog to identify unlabeled data.  That version
 *  depends on the input parameters when the recog is created.  The choices
 *  for the latter version, and their suggested use, are:
 *  (1) unscaled SI -- typical for BAR, generated from book images
 *  (2) unscaled WNL -- ditto
 *  (3) scaled SI -- typical for recognizers containing template
 *      images from sources other than the book to be recognized
 *  (4) scaled WNL -- ditto
 *  For cases (3) and (4), we recommend scaling to fixed height; e.g.,
 *  scalew = 0, scaleh = 40.
 *  When using WNL, we recommend using a width of 5 in the template
 *  and 4 in the unlabeled data.
 *  It appears that better results for a BAR are usually obtained using
 *  SI than WNL, but more experimentation is needed.
 *
 *  This utility is designed to build recognizers that are specifically
 *  adapted from a large amount of material, such as a book.  These
 *  use labeled templates taken from the material, and not scaled.
 *  In addition, two special recognizers are useful:
 *  (1) Bootstrap recognizer (BSR).  This uses height-scaled templates,
 *      that have been extended with several repetitions in one of two ways:
 *      (a) aniotropic width scaling (for either SI or WNL)
 *      (b) iterative erosions/dilations (for SI).
 *  (2) Outlier removal.  This uses height scaled templates.  It can be
 *      implemented without using templates that are aligned averages of all
 *      templates in a class.
 *
 *  Recognizers are inexpensive to generate, for example, from a pixa
 *  of labeled templates.  The general process of building a BAR is
 *  to start with labeled templates, e.g., in a pixa, make a BAR, and
 *  analyze new samples from the book to augment the BAR until it has
 *  enough samples for each character class.  Along the way, samples
 *  from a BSR may be added for help in training.  If not enough samples
 *  are available for the BAR, it can finally be augmented with BSR
 *  samples, in which case the resulting hybrid BAR/BSR recognizer
 *  must work on scaled images.
 *
 *  Here are the steps in doing recog training:
 *  A. Generate a BAR from any existing labeled templates
 *    (1) Create a recog and add the templates, using recogAddSample().
 *        This stores the unscaled templates.
 *        [Note: this can be done in one step if the labeled templates are put
 *         into a pixa:
 *           L_Recog *rec = recogCreateFromPixa(pixa, ...);  ]
 *    (2) Call recogTrainingFinished() to generate the (sometimes modified)
 *        templates to be used for correlation.
 *    (3) Optionally, remove outliers.
 *    If there are sufficient samples in the classes, we're done. Otherwise,
 *  B. Try to get more samples from the book to pad the BAR.
 *     (1) Save the unscaled, labeled templates from the BAR.
 *     (2) Supplement the BAR with bootstrap templates to make a hybrid BAR/BSR.
 *     (3) Do recognition on more unlabeled images, scaled to a fixed height
 *     (4) Add the unscaled, labeled images to the saved set.
 *     (5) Optionally, remove outliers.
 *     If there are sufficient samples in the classes, we're done. Otherwise,
 *  C. For classes without a sufficient number of templates, we can
 *     supplement the BAR with templates from a BSR (a hybrid RAR/BSR),
 *     and do recognition scaled to a fixed height.
 *
 *  Here are several methods that can be used for identifying outliers:
 *  (1) Compute average templates for each class and remove a candidate
 *      that is poorly correlated with the average.  This is the most
 *      simple method.  recogRemoveOutliers1() uses this, supplemented with
 *      a second threshold and a target number of templates to be saved.
 *  (2) Compute average templates for each class and remove a candidate
 *      that is more highly correlated with the average of some other class.
 *      This does not require setting a threshold for the correlation.
 *      recogRemoveOutliers2() uses this method, supplemented with a minimum
 *      correlation score.
 *  (3) For each candidate, find the average correlation with other
 *      members of its class, and remove those that have a relatively
 *      low average correlation.  This is similar to (1), gives comparable
 *      results and because it does not use average templates, it requires
 *      a bit more computation.
 * </pre>
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include <string.h>
#include "allheaders.h"
#include "pix_internal.h"

    /* Static functions */
static l_int32 recogTemplatesAreOK(L_RECOG *recog, l_int32 minsize,
                                   l_float32 minfract, l_int32 *pok);
static SARRAY *recogAddMissingClassStrings(L_RECOG  *recog);
static l_int32 recogCharsetAvailable(l_int32 type);
static PIX *pixDisplayOutliers(PIXA *pixas, NUMA *nas);
static PIX *recogDisplayOutlier(L_RECOG *recog, l_int32 iclass, l_int32 jsamp,
                                l_int32 maxclass, l_float32 maxscore);

    /* Default parameters that are used in recogTemplatesAreOK() and
     * in outlier removal functions, and that use template set size
     * to decide if the set of templates (before outliers are removed)
     * is valid.  Values are set to accept most sets of sample templates. */
static const l_int32    DefaultMinSetSize = 1;  /* minimum number of
                                       samples for a valid class */
static const l_float32  DefaultMinSetFract = 0.4f;  /* minimum fraction
                               of classes required for a valid recog */

    /* Defaults in pixaRemoveOutliers1() and pixaRemoveOutliers2() */
static const l_float32  DefaultMinScore = 0.75; /* keep everything above */
static const l_int32    DefaultMinTarget = 3;  /* to be kept if possible */
static const l_float32  LowerScoreThreshold = 0.5;  /* templates can be
                 * kept down to this score to if needed to retain the
                 * desired minimum number of templates */


/*------------------------------------------------------------------------*
 *                                Training                                *
 *------------------------------------------------------------------------*/
/*!
 * \brief   recogTrainLabeled()
 *
 * \param[in]    recog     in training mode
 * \param[in]    pixs      if depth > 1, will be thresholded to 1 bpp
 * \param[in]    box       [optional] cropping box
 * \param[in]    text      [optional] if null, use text field in pix
 * \param[in]    debug     1 to display images of samples not captured
 * \return  0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *      (1) Training is restricted to the addition of a single
 *          character in an arbitrary (e.g., UTF8) charset
 *      (2) If box != null, it should represent the location in %pixs
 *          of the character image.
 * </pre>
 */
l_ok
recogTrainLabeled(L_RECOG  *recog,
                  PIX      *pixs,
                  BOX      *box,
                  char     *text,
                  l_int32   debug)
{
l_int32  ret;
PIX     *pix;

    if (!recog)
        return ERROR_INT("recog not defined", __func__, 1);
    if (!pixs)
        return ERROR_INT("pixs not defined", __func__, 1);

        /* Prepare the sample to be added. This step also acts
         * as a filter, and can invalidate pixs as a template. */
    ret = recogProcessLabeled(recog, pixs, box, text, &pix);
    if (ret) {
        pixDestroy(&pix);
        L_WARNING("failure to get sample '%s' for training\n", __func__,
                  text);
        return 1;
    }

    recogAddSample(recog, pix, debug);
    pixDestroy(&pix);
    return 0;
}


/*!
 * \brief   recogProcessLabeled()
 *
 * \param[in]    recog   in training mode
 * \param[in]    pixs    if depth > 1, will be thresholded to 1 bpp
 * \param[in]    box     [optional] cropping box
 * \param[in]    text    [optional] if null, use text field in pix
 * \param[out]   ppix    addr of pix, 1 bpp, labeled
 * \return  0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *      (1) This crops and binarizes the input image, generating a pix
 *          of one character where the charval is inserted into the pix.
 * </pre>
 */
l_ok
recogProcessLabeled(L_RECOG  *recog,
                    PIX      *pixs,
                    BOX      *box,
                    char     *text,
                    PIX     **ppix)
{
char    *textdata;
l_int32  textinpix, textin, nsets;
NUMA    *na;
PIX     *pix1, *pix2, *pix3, *pix4;

    if (!ppix)
        return ERROR_INT("&pix not defined", __func__, 1);
    *ppix = NULL;
    if (!recog)
        return ERROR_INT("recog not defined", __func__, 1);
    if (!pixs)
        return ERROR_INT("pixs not defined", __func__, 1);

        /* Find the text; this will be stored with the output images */
    textin = text && (text[0] != '\0');
    textinpix = (pixs->text && (pixs->text[0] != '\0'));
    if (!textin && !textinpix) {
        L_ERROR("no text: %d\n", __func__, recog->num_samples);
        return 1;
    }
    textdata = (textin) ? text : pixs->text;  /* do not free */

        /* Crop and binarize if necessary */
    if (box)
        pix1 = pixClipRectangle(pixs, box, NULL);
    else
        pix1 = pixClone(pixs);
    if (pixGetDepth(pix1) > 1)
        pix2 = pixConvertTo1(pix1, recog->threshold);
    else
        pix2 = pixClone(pix1);
    pixDestroy(&pix1);

        /* Remove isolated noise, using as a criterion all components
         * that are removed by a vertical opening of size 5. */
    pix3 = pixMorphSequence(pix2, "o1.5", 0);  /* seed */
    pixSeedfillBinary(pix3, pix3, pix2, 8);  /* fill from seed; clip to pix2 */
    pixDestroy(&pix2);

        /* Clip to foreground */
    pixClipToForeground(pix3, &pix4, NULL);
    pixDestroy(&pix3);
    if (!pix4)
        return ERROR_INT("pix4 is empty", __func__, 1);

        /* Verify that if there is more than 1 c.c., they all have
         * horizontal overlap */
    na = pixCountByColumn(pix4, NULL);
    numaCountNonzeroRuns(na, &nsets);
    numaDestroy(&na);
    if (nsets > 1) {
        L_WARNING("found %d sets of horiz separated c.c.; skipping\n",
                  __func__, nsets);
        pixDestroy(&pix4);
        return 1;
    }

    pixSetText(pix4, textdata);
    *ppix = pix4;
    return 0;
}


/*!
 * \brief   recogAddSample()
 *
 * \param[in]    recog
 * \param[in]    pix         a single character, 1 bpp
 * \param[in]    debug
 * \return  0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *      (1) The pix is 1 bpp, with the character string label embedded.
 *      (2) The pixaa_u array of the recog is initialized to accept
 *          up to 256 different classes.  When training is finished,
 *          the arrays are truncated to the actual number of classes.
 *          To pad an existing recog from the boot recognizers, training
 *          is started again; if samples from a new class are added,
 *          the pixaa_u array is extended by adding a pixa to hold them.
 * </pre>
 */
l_ok
recogAddSample(L_RECOG  *recog,
               PIX      *pix,
               l_int32   debug)
{
char    *text;
l_int32  npa, charint, index;
PIXA    *pixa1;
PIXAA   *paa;

    if (!recog)
        return ERROR_INT("recog not defined", __func__, 1);
    if (!pix || pixGetDepth(pix) != 1)
        return ERROR_INT("pix not defined or not 1 bpp\n", __func__, 1);
    if (recog->train_done)
        return ERROR_INT("not added: training has been completed", __func__, 1);
    paa = recog->pixaa_u;

        /* Make sure the character is in the set */
    text = pixGetText(pix);
    if (l_convertCharstrToInt(text, &charint) == 1) {
        L_ERROR("invalid text: %s\n", __func__, text);
        return 1;
    }

        /* Determine the class array index.  Check if the class
         * already exists, and if not, add it. */
    if (recogGetClassIndex(recog, charint, text, &index) == 1) {
            /* New class must be added */
        npa = pixaaGetCount(paa, NULL);
        if (index > npa) {
            L_ERROR("oops: bad index %d > npa %d!!\n", __func__, index, npa);
            return 1;
        }
        if (index == npa) {  /* paa needs to be extended */
            L_INFO("Adding new class and pixa: index = %d, text = %s\n",
                   __func__, index, text);
            pixa1 = pixaCreate(10);
            pixaaAddPixa(paa, pixa1, L_INSERT);
        }
    }
    if (debug) {
        L_INFO("Identified text label: %s\n", __func__, text);
        L_INFO("Identified: charint = %d, index = %d\n",
               __func__, charint, index);
    }

        /* Insert the unscaled character image into the right pixa.
         * (Unscaled images are required to split touching characters.) */
    recog->num_samples++;
    pixaaAddPix(paa, index, pix, NULL, L_COPY);
    return 0;
}


/*!
 * \brief   recogModifyTemplate()
 *
 * \param[in]    recog
 * \param[in]    pixs   1 bpp, to be optionally scaled and turned into
 *                      strokes of fixed width
 * \return  pixd   modified pix if OK, NULL on error
 */
PIX *
recogModifyTemplate(L_RECOG  *recog,
                    PIX      *pixs)
{
l_int32  w, h, empty;
PIX     *pix1, *pix2;

    if (!recog)
        return (PIX *)ERROR_PTR("recog not defined", __func__, NULL);
    if (!pixs)
        return (PIX *)ERROR_PTR("pixs not defined", __func__, NULL);

        /* Scale first */
    pixGetDimensions(pixs, &w, &h, NULL);
    if ((recog->scalew == 0 || recog->scalew == w) &&
        (recog->scaleh == 0 || recog->scaleh == h)) {  /* no scaling */
        pix1 = pixCopy(NULL, pixs);
    } else {
        pix1 = pixScaleToSize(pixs, recog->scalew, recog->scaleh);
    }
    if (!pix1)
        return (PIX *)ERROR_PTR("pix1 not made", __func__, NULL);

        /* Then optionally convert to lines */
    if (recog->linew <= 0) {
        pix2 = pixClone(pix1);
    } else {
        pix2 = pixSetStrokeWidth(pix1, recog->linew, 1, 8);
    }
    pixDestroy(&pix1);
    if (!pix2)
        return (PIX *)ERROR_PTR("pix2 not made", __func__, NULL);

        /* Make sure we still have some pixels */
    pixZero(pix2, &empty);
    if (empty) {
        pixDestroy(&pix2);
        return (PIX *)ERROR_PTR("modified template has no pixels",
                                __func__, NULL);
    }
    return pix2;
}


/*!
 * \brief   recogAverageSamples()
 *
 * \param[in]   recog    addr of existing recog
 * \param[in]   debug
 * \return  0 on success, 1 on failure
 *
 * <pre>
 * Notes:
 *      (1) This is only called in two situations:
 *          (a) When splitting characters using either the DID method
 *              recogDecode() or the the greedy splitter
 *              recogCorrelationBestRow()
 *          (b) By a special recognizer that is used to remove outliers.
 *          Both unscaled and scaled inputs are averaged.
 *      (2) If the data in any class is nonexistent (no samples), or
 *          very bad (no fg pixels in the average), or if the ratio
 *          of max/min average unscaled class template heights is
 *          greater than max_ht_ratio, this function fails.  The caller
 *          must check the return value of the recog, and destroy the
 *          recog on failure.
 *      (3) Set debug = 1 to view the resulting templates and their centroids.
 * </pre>
 */
l_int32
recogAverageSamples(L_RECOG  *recog,
                    l_int32   debug)
{
l_int32    i, nsamp, size, area, bx, by, badclass;
l_float32  x, y, hratio;
BOX       *box;
PIXA      *pixa1;
PIX       *pix1, *pix2, *pix3;
PTA       *pta1;

    if (!recog)
        return ERROR_INT("recog not defined", __func__, 1);

    if (recog->ave_done) {
        if (debug)  /* always do this if requested */
            recogShowAverageTemplates(recog);
        return 0;
    }

        /* Remove any previous averaging data */
    size = recog->setsize;
    pixaDestroy(&recog->pixa_u);
    ptaDestroy(&recog->pta_u);
    numaDestroy(&recog->nasum_u);
    recog->pixa_u = pixaCreate(size);
    recog->pta_u = ptaCreate(size);
    recog->nasum_u = numaCreate(size);

    pixaDestroy(&recog->pixa);
    ptaDestroy(&recog->pta);
    numaDestroy(&recog->nasum);
    recog->pixa = pixaCreate(size);
    recog->pta = ptaCreate(size);
    recog->nasum = numaCreate(size);

        /* Unscaled bitmaps: compute averaged bitmap, centroid, and fg area.
         * Note that when we threshold to 1 bpp the 8 bpp averaged template
         * that is returned from the accumulator, it will not be cropped
         * to the foreground.  We must crop it, because the correlator
         * makes that assumption and will return a zero value if the
         * width or height of the two images differs by several pixels.
         * But cropping to fg can cause the value of the centroid to
         * change, if bx > 0 or by > 0. */
    badclass = FALSE;
    for (i = 0; i < size; i++) {
        pixa1 = pixaaGetPixa(recog->pixaa_u, i, L_CLONE);
        pta1 = ptaaGetPta(recog->ptaa_u, i, L_CLONE);
        nsamp = pixaGetCount(pixa1);
        nsamp = L_MIN(nsamp, 256);  /* we only use the first 256 */
        if (nsamp == 0) {  /* no information for this class */
            L_ERROR("no samples in class %d\n", __func__, i);
            badclass = TRUE;
            pixaDestroy(&pixa1);
            ptaDestroy(&pta1);
            break;
        } else {
            pixaAccumulateSamples(pixa1, pta1, &pix1, &x, &y);
            pix2 = pixThresholdToBinary(pix1, L_MAX(1, nsamp / 2));
            pixInvert(pix2, pix2);
            pixClipToForeground(pix2, &pix3, &box);
            if (!box) {
                L_ERROR("no fg pixels in average for uclass %d\n", __func__, i);
                badclass = TRUE;
                pixDestroy(&pix1);
                pixDestroy(&pix2);
                pixaDestroy(&pixa1);
                ptaDestroy(&pta1);
                break;
            } else {
                boxGetGeometry(box, &bx, &by, NULL, NULL);
                pixaAddPix(recog->pixa_u, pix3, L_INSERT);
                ptaAddPt(recog->pta_u, x - bx, y - by);  /* correct centroid */
                pixCountPixels(pix3, &area, recog->sumtab);
                numaAddNumber(recog->nasum_u, area);  /* foreground */
                boxDestroy(&box);
            }
            pixDestroy(&pix1);
            pixDestroy(&pix2);
        }
        pixaDestroy(&pixa1);
        ptaDestroy(&pta1);
    }

        /* Are any classes bad?  */
    if (badclass)
        return ERROR_INT("at least 1 bad class", __func__, 1);

        /* Get the range of sizes of the unscaled average templates.
         * Reject if the height ratio is too large.  */
    pixaSizeRange(recog->pixa_u, &recog->minwidth_u, &recog->minheight_u,
                  &recog->maxwidth_u, &recog->maxheight_u);
    hratio = (l_float32)recog->maxheight_u / (l_float32)recog->minheight_u;
    if (hratio > recog->max_ht_ratio) {
        L_ERROR("ratio of max/min height of average templates = %4.1f\n",
                __func__, hratio);
        return 1;
    }

        /* Scaled bitmaps: compute averaged bitmap, centroid, and fg area */
    for (i = 0; i < size; i++) {
        pixa1 = pixaaGetPixa(recog->pixaa, i, L_CLONE);
        pta1 = ptaaGetPta(recog->ptaa, i, L_CLONE);
        nsamp = pixaGetCount(pixa1);
        nsamp = L_MIN(nsamp, 256);  /* we only use the first 256 */
        pixaAccumulateSamples(pixa1, pta1, &pix1, &x, &y);
        pix2 = pixThresholdToBinary(pix1, L_MAX(1, nsamp / 2));
        pixInvert(pix2, pix2);
        pixClipToForeground(pix2, &pix3, &box);
        if (!box) {
            L_ERROR("no fg pixels in average for class %d\n", __func__, i);
            badclass = TRUE;
            pixDestroy(&pix1);
            pixDestroy(&pix2);
            pixaDestroy(&pixa1);
            ptaDestroy(&pta1);
            break;
        } else {
            boxGetGeometry(box, &bx, &by, NULL, NULL);
            pixaAddPix(recog->pixa, pix3, L_INSERT);
            ptaAddPt(recog->pta, x - bx, y - by);  /* correct centroid */
            pixCountPixels(pix3, &area, recog->sumtab);
            numaAddNumber(recog->nasum, area);  /* foreground */
            boxDestroy(&box);
        }
        pixDestroy(&pix1);
        pixDestroy(&pix2);
        pixaDestroy(&pixa1);
        ptaDestroy(&pta1);
    }

    if (badclass)
        return ERROR_INT("no fg pixels in at least 1 class", __func__, 1);

        /* Get the range of widths of the scaled average templates */
    pixaSizeRange(recog->pixa, &recog->minwidth, NULL, &recog->maxwidth, NULL);

       /* Get dimensions useful for splitting */
    recog->min_splitw = L_MAX(5, recog->minwidth_u - 5);
    recog->max_splith = recog->maxheight_u + 12;  /* allow for skew */

    if (debug)
        recogShowAverageTemplates(recog);

    recog->ave_done = TRUE;
    return 0;
}


/*!
 * \brief   pixaAccumulateSamples()
 *
 * \param[in]    pixa     of samples from the same class, 1 bpp
 * \param[in]    pta      [optional] of centroids of the samples
 * \param[out]   ppixd    accumulated samples, 8 bpp
 * \param[out]   px       [optional] average x coordinate of centroids
 * \param[out]   py       [optional] average y coordinate of centroids
 * \return  0 on success, 1 on failure
 *
 * <pre>
 * Notes:
 *      (1) This generates an aligned (by centroid) sum of the input pix.
 *      (2) We use only the first 256 samples; that's plenty.
 *      (3) If pta is not input, we generate two tables, and discard
 *          after use.  If this is called many times, it is better
 *          to precompute the pta.
 * </pre>
 */
l_int32
pixaAccumulateSamples(PIXA       *pixa,
                      PTA        *pta,
                      PIX       **ppixd,
                      l_float32  *px,
                      l_float32  *py)
{
l_int32    i, n, maxw, maxh, xdiff, ydiff;
l_int32   *centtab, *sumtab;
l_float32  xc, yc, xave, yave;
PIX       *pix1, *pix2, *pixsum;
PTA       *ptac;

    if (px) *px = 0;
    if (py) *py = 0;
    if (!ppixd)
        return ERROR_INT("&pixd not defined", __func__, 1);
    *ppixd = NULL;
    if (!pixa)
        return ERROR_INT("pixa not defined", __func__, 1);

    n = pixaGetCount(pixa);
    if (pta && ptaGetCount(pta) != n)
        return ERROR_INT("pta count differs from pixa count", __func__, 1);
    n = L_MIN(n, 256);  /* take the first 256 only */
    if (n == 0)
        return ERROR_INT("pixa array empty", __func__, 1);

        /* Find the centroids */
    if (pta) {
        ptac = ptaClone(pta);
    } else {  /* generate them here */
        ptac = ptaCreate(n);
        centtab = makePixelCentroidTab8();
        sumtab = makePixelSumTab8();
        for (i = 0; i < n; i++) {
            pix1 = pixaGetPix(pixa, i, L_CLONE);
            pixCentroid(pix1, centtab, sumtab, &xc, &yc);
            ptaAddPt(ptac, xc, yc);
        }
        LEPT_FREE(centtab);
        LEPT_FREE(sumtab);
    }

        /* Find the average value of the centroids */
    xave = yave = 0;
    for (i = 0; i < n; i++) {
        ptaGetPt(pta, i, &xc, &yc);
        xave += xc;
        yave += yc;
    }
    xave = xave / (l_float32)n;
    yave = yave / (l_float32)n;
    if (px) *px = xave;
    if (py) *py = yave;

        /* Place all pix with their centroids located at the average
         * centroid value, and sum the results.  Make the accumulator
         * image slightly larger than the largest sample to insure
         * that all pixels are represented in the accumulator.  */
    pixaSizeRange(pixa, NULL, NULL, &maxw, &maxh);
    pixsum = pixInitAccumulate(maxw + 5, maxh + 5, 0);
    pix1 = pixCreate(maxw, maxh, 1);
    for (i = 0; i < n; i++) {
        pix2 = pixaGetPix(pixa, i, L_CLONE);
        ptaGetPt(ptac, i, &xc, &yc);
        xdiff = (l_int32)(xave - xc);
        ydiff = (l_int32)(yave - yc);
        pixClearAll(pix1);
        pixRasterop(pix1, xdiff, ydiff, maxw, maxh, PIX_SRC,
                    pix2, 0, 0);
        pixAccumulate(pixsum, pix1, L_ARITH_ADD);
        pixDestroy(&pix2);
    }
    *ppixd = pixFinalAccumulate(pixsum, 0, 8);

    pixDestroy(&pix1);
    pixDestroy(&pixsum);
    ptaDestroy(&ptac);
    return 0;
}


/*!
 * \brief   recogTrainingFinished()
 *
 * \param[in]    precog       addr of recog
 * \param[in]    modifyflag   1 to use recogModifyTemplate(); 0 otherwise
 * \param[in]    minsize      set to -1 for default
 * \param[in]    minfract     set to -1.0 for default
 * \return  0 if OK, 1 on error (input recog will be destroyed)
 *
 * <pre>
 * Notes:
 *      (1) This must be called after all training samples have been added.
 *      (2) If the templates are not good enough, the recog input is destroyed.
 *      (3) Usually, %modifyflag == 1, because we want to apply
 *          recogModifyTemplate() to generate the actual templates
 *          that will be used.  The one exception is when reading a
 *          serialized recog: there we want to put the same set of
 *          templates in both the unscaled and modified pixaa.
 *          See recogReadStream() to see why we do this.
 *      (4) See recogTemplatesAreOK() for %minsize and %minfract usage.
 *      (5) The following things are done here:
 *          (a) Allocate (or reallocate) storage for (possibly) modified
 *              bitmaps, centroids, and fg areas.
 *          (b) Generate the (possibly) modified bitmaps.
 *          (c) Compute centroid and fg area data for both unscaled and
 *              modified bitmaps.
 *          (d) Truncate the pixaa, ptaa and numaa arrays down from
 *              256 to the actual size.
 *      (6) Putting these operations here makes it simple to recompute
 *          the recog with different modifications on the bitmaps.
 *      (7) Call recogShowContent() to display the templates, both
 *          unscaled and modified.
 * </pre>
 */
l_ok
recogTrainingFinished(L_RECOG  **precog,
                      l_int32    modifyflag,
                      l_int32    minsize,
                      l_float32  minfract)
{
l_int32    ok, i, j, size, nc, ns, area;
l_float32  xave, yave;
PIX       *pix, *pixd;
PIXA      *pixa;
PIXAA     *paa;
PTA       *pta;
PTAA      *ptaa;
L_RECOG   *recog;

    if (!precog)
        return ERROR_INT("&recog not defined", __func__, 1);
    if ((recog = *precog) == NULL)
        return ERROR_INT("recog not defined", __func__, 1);
    if (recog->train_done) return 0;

        /* Test the input templates */
    recogTemplatesAreOK(recog, minsize, minfract, &ok);
    if (!ok) {
        recogDestroy(precog);
        return ERROR_INT("bad templates", __func__, 1);
    }

        /* Generate the storage for the possibly-scaled training bitmaps */
    size = recog->maxarraysize;
    paa = pixaaCreate(size);
    pixa = pixaCreate(1);
    pixaaInitFull(paa, pixa);
    pixaDestroy(&pixa);
    pixaaDestroy(&recog->pixaa);
    recog->pixaa = paa;

        /* Generate the storage for the unscaled centroid training data */
    ptaa = ptaaCreate(size);
    pta = ptaCreate(0);
    ptaaInitFull(ptaa, pta);
    ptaaDestroy(&recog->ptaa_u);
    recog->ptaa_u = ptaa;

        /* Generate the storage for the possibly-scaled centroid data */
    ptaa = ptaaCreate(size);
    ptaaInitFull(ptaa, pta);
    ptaDestroy(&pta);
    ptaaDestroy(&recog->ptaa);
    recog->ptaa = ptaa;

        /* Generate the storage for the fg area data */
    numaaDestroy(&recog->naasum_u);
    numaaDestroy(&recog->naasum);
    recog->naasum_u = numaaCreateFull(size, 0);
    recog->naasum = numaaCreateFull(size, 0);

    paa = recog->pixaa_u;
    nc = recog->setsize;
    for (i = 0; i < nc; i++) {
        pixa = pixaaGetPixa(paa, i, L_CLONE);
        ns = pixaGetCount(pixa);
        for (j = 0; j < ns; j++) {
                /* Save centroid and area data for the unscaled pix */
            pix = pixaGetPix(pixa, j, L_CLONE);
            pixCentroid(pix, recog->centtab, recog->sumtab, &xave, &yave);
            ptaaAddPt(recog->ptaa_u, i, xave, yave);
            pixCountPixels(pix, &area, recog->sumtab);
            numaaAddNumber(recog->naasum_u, i, area);  /* foreground */

                /* Insert the (optionally) scaled character image, and
                 * save centroid and area data for it */
            if (modifyflag == 1)
                pixd = recogModifyTemplate(recog, pix);
            else
                pixd = pixClone(pix);
            if (pixd) {
                pixaaAddPix(recog->pixaa, i, pixd, NULL, L_INSERT);
                pixCentroid(pixd, recog->centtab, recog->sumtab, &xave, &yave);
                ptaaAddPt(recog->ptaa, i, xave, yave);
                pixCountPixels(pixd, &area, recog->sumtab);
                numaaAddNumber(recog->naasum, i, area);
            } else {
                L_ERROR("failed: modified template for class %d, sample %d\n",
                        __func__, i, j);
            }
            pixDestroy(&pix);
        }
        pixaDestroy(&pixa);
    }

        /* Truncate the arrays to those with non-empty containers */
    pixaaTruncate(recog->pixaa_u);
    pixaaTruncate(recog->pixaa);
    ptaaTruncate(recog->ptaa_u);
    ptaaTruncate(recog->ptaa);
    numaaTruncate(recog->naasum_u);
    numaaTruncate(recog->naasum);

    recog->train_done = TRUE;
    return 0;
}


/*!
 * \brief   recogTemplatesAreOK()
 *
 * \param[in]    recog
 * \param[in]    minsize     set to -1 for default
 * \param[in]    minfract    set to -1.0 for default
 * \param[out]   pok         set to 1 if template set is valid; 0 otherwise
 * \return  1 on error; 0 otherwise.  An invalid template set is not an error.
 *
 * <pre>
 * Notes:
 *      (1) This is called by recogTrainingFinished().  A return value of 0
 *          will cause recogTrainingFinished() to destroy the recog.
 *      (2) %minsize is the minimum number of samples required for
 *          the class; -1 uses the default
 *      (3) %minfract is the minimum fraction of classes required for
 *          the recog to be usable; -1.0 uses the default
 * </pre>
 */
static l_int32
recogTemplatesAreOK(L_RECOG   *recog,
                    l_int32    minsize,
                    l_float32  minfract,
                    l_int32   *pok)
{
l_int32    i, n, validsets, nt;
l_float32  ratio;
NUMA      *na;

    if (!pok)
        return ERROR_INT("&ok not defined", __func__, 1);
    *pok = 0;
    if (!recog)
        return ERROR_INT("recog not defined", __func__, 1);

    minsize = (minsize < 0) ? DefaultMinSetSize : minsize;
    minfract = (minfract < 0) ? DefaultMinSetFract : minfract;
    n = pixaaGetCount(recog->pixaa_u, &na);
    validsets = 0;
    for (i = 0, validsets = 0; i < n; i++) {
        numaGetIValue(na, i, &nt);
        if (nt >= minsize)
            validsets++;
    }
    numaDestroy(&na);
    ratio = (l_float32)validsets / (l_float32)recog->charset_size;
    *pok = (ratio >= minfract) ? 1 : 0;
    return 0;
}


/*!
 * \brief   recogFilterPixaBySize()
 *
 * \param[in]   pixas         labeled templates
 * \param[in]   setsize       size of character set (number of classes)
 * \param[in]   maxkeep       max number of templates to keep in a class
 * \param[in]   max_ht_ratio  max allowed height ratio (see below)
 * \param[out]  pna           [optional] debug output, giving the number
 *                            in each class after filtering; use NULL to skip
 * \return  pixa   filtered templates, or NULL on error
 *
 * <pre>
 * Notes:
 *      (1) The basic assumption is that the most common and larger
 *          templates in each class are more likely to represent the
 *          characters we are interested in.  For example, larger digits
 *          are more likely to represent page numbers, and smaller digits
 *          could be data in tables.  Therefore, we bias the first
 *          stage of filtering toward the larger characters by removing
 *          very small ones, and select based on proximity of the
 *          remaining characters to median height.
 *      (2) For each of the %setsize classes, order the templates
 *          increasingly by height.  Take the rank 0.9 height.  Eliminate
 *          all templates that are shorter by more than %max_ht_ratio.
 *          Of the remaining ones, select up to %maxkeep that are closest
 *          in rank order height to the median template.
 * </pre>
 */
PIXA *
recogFilterPixaBySize(PIXA      *pixas,
                      l_int32    setsize,
                      l_int32    maxkeep,
                      l_float32  max_ht_ratio,
                      NUMA     **pna)
{
l_int32    i, j, h90, hj, j1, j2, j90, n, nc;
l_float32  ratio;
NUMA      *na;
PIXA      *pixa1, *pixa2, *pixa3, *pixa4, *pixa5;
PIXAA     *paa;

    if (pna) *pna = NULL;
    if (!pixas)
        return (PIXA *)ERROR_PTR("pixas not defined", __func__, NULL);

    if ((paa = recogSortPixaByClass(pixas, setsize)) == NULL)
        return (PIXA *)ERROR_PTR("paa not made", __func__, NULL);
    nc = pixaaGetCount(paa, NULL);
    na = (pna) ? numaCreate(0) : NULL;
    if (pna) *pna = na;
    pixa5 = pixaCreate(0);
    for (i = 0; i < nc; i++) {
        pixa1 = pixaaGetPixa(paa, i, L_CLONE);
        if ((n = pixaGetCount(pixa1)) == 0) {
            pixaDestroy(&pixa1);
            continue;
        }
        pixa2 = pixaSort(pixa1, L_SORT_BY_HEIGHT, L_SORT_INCREASING, NULL,
                         L_COPY);
        j90 = (l_int32)(0.9 * n);
        pixaGetPixDimensions(pixa2, j90, NULL, &h90, NULL);
        pixa3 = pixaCreate(n);
        for (j = 0; j < n; j++) {
            pixaGetPixDimensions(pixa2, j, NULL, &hj, NULL);
            ratio = (l_float32)h90 / (l_float32)hj;
            if (ratio <= max_ht_ratio)
                pixaAddPix(pixa3, pixaGetPix(pixa2, j, L_COPY), L_INSERT);
        }
        n = pixaGetCount(pixa3);
        if (n <= maxkeep) {
            pixa4 = pixaCopy(pixa3, L_CLONE);
        } else {
            j1 = (n - maxkeep) / 2;
            j2 = j1 + maxkeep - 1;
            pixa4 = pixaSelectRange(pixa3, j1, j2, L_CLONE);
        }
        if (na) numaAddNumber(na, pixaGetCount(pixa4));
        pixaJoin(pixa5, pixa4, 0, -1);
        pixaDestroy(&pixa1);
        pixaDestroy(&pixa2);
        pixaDestroy(&pixa3);
        pixaDestroy(&pixa4);
    }

    pixaaDestroy(&paa);
    return pixa5;
}


/*!
 * \brief   recogSortPixaByClass()
 *
 * \param[in]   pixa       labeled templates
 * \param[in]   setsize    size of character set (number of classes)
 * \return  paa   pixaa where each pixa has templates for one class,
 *                or null on error
 */
PIXAA *
recogSortPixaByClass(PIXA    *pixa,
                     l_int32  setsize)
{
PIXAA    *paa;
L_RECOG  *recog;

    if (!pixa)
        return (PIXAA *)ERROR_PTR("pixa not defined", __func__, NULL);

    if ((recog = recogCreateFromPixaNoFinish(pixa, 0, 0, 0, 0, 0)) == NULL)
        return (PIXAA *)ERROR_PTR("recog not made", __func__, NULL);
    paa = recog->pixaa_u;   /* grab the paa of unscaled templates */
    recog->pixaa_u = NULL;
    recogDestroy(&recog);
    return paa;
}


/*!
 * \brief   recogRemoveOutliers1()
 *
 * \param[in]   precog       addr of recog with unscaled labeled templates
 * \param[in]   minscore     keep everything with at least this score
 * \param[in]   mintarget    minimum desired number to retain if possible
 * \param[in]   minsize      minimum number of samples required for a class
 * \param[out]  ppixsave     [optional debug] saved templates, with scores
 * \param[out]  ppixrem      [optional debug] removed templates, with scores
 * \return  0 if OK, 1 on error.
 *
 * <pre>
 * Notes:
 *      (1) This is a convenience wrapper when using default parameters
 *          for the recog.  See pixaRemoveOutliers1() for details.
 *      (2) If this succeeds, the new recog replaces the input recog;
 *          if it fails, the input recog is destroyed.
 * </pre>
 */
l_ok
recogRemoveOutliers1(L_RECOG  **precog,
                     l_float32  minscore,
                     l_int32    mintarget,
                     l_int32    minsize,
                     PIX      **ppixsave,
                     PIX      **ppixrem)
{
PIXA     *pixa1, *pixa2;
L_RECOG  *recog;

    if (!precog)
        return ERROR_INT("&recog not defined", __func__, 1);
    if (*precog == NULL)
        return ERROR_INT("recog not defined", __func__, 1);

        /* Extract the unscaled templates */
    pixa1 = recogExtractPixa(*precog);
    recogDestroy(precog);

    pixa2 = pixaRemoveOutliers1(pixa1, minscore, mintarget, minsize,
                                ppixsave, ppixrem);
    pixaDestroy(&pixa1);
    if (!pixa2)
        return ERROR_INT("failure to remove outliers", __func__, 1);

    recog = recogCreateFromPixa(pixa2, 0, 0, 0, 150, 1);
    pixaDestroy(&pixa2);
    if (!recog)
        return ERROR_INT("failure to make recog from pixa sans outliers",
                          __func__, 1);

    *precog = recog;
    return 0;
}


/*!
 * \brief   pixaRemoveOutliers1()
 *
 * \param[in]   pixas        unscaled labeled templates
 * \param[in]   minscore     keep everything with at least this score;
 *                           use -1.0 for default.
 * \param[in]   mintarget    minimum desired number to retain if possible;
 *                           use -1 for default.
 * \param[in]   minsize      minimum number of samples required for a class;
 *                           use -1 for default.
 * \param[out]  ppixsave     [optional debug] saved templates, with scores
 * \param[out]  ppixrem      [optional debug] removed templates, with scores
 * \return  pixa   of unscaled templates to be kept, or NULL on error
 *
 * <pre>
 * Notes:
 *      (1) Removing outliers is particularly important when recognition
 *          goes against all the samples in the training set, as opposed
 *          to the averages for each class.  The reason is that we get
 *          an identification error if a mislabeled template is a best
 *          match for an input sample.
 *      (2) Because the score values depend strongly on the quality
 *          of the character images, to avoid losing too many samples
 *          we supplement a minimum score for retention with a score
 *          necessary to acquire the minimum target number of templates.
 *          To do this we are willing to use a lower threshold,
 *          LowerScoreThreshold, on the score.  Consequently, with
 *          poor quality templates, we may keep samples with a score
 *          less than %minscore, but never less than LowerScoreThreshold.
 *          And if the number of samples is less than %minsize, we do
 *          not use any.
 *      (3) This is meant to be used on a BAR, where the templates all
 *          come from the same book; use minscore ~0.75.
 *      (4) Method: make a scaled recog from the input %pixas.  Then,
 *          for each class: generate the averages, match each
 *          scaled template against the average, and save unscaled
 *          templates that had a sufficiently good match.
 * </pre>
 */
PIXA *
pixaRemoveOutliers1(PIXA      *pixas,
                    l_float32  minscore,
                    l_int32    mintarget,
                    l_int32    minsize,
                    PIX      **ppixsave,
                    PIX      **ppixrem)
{
l_int32    i, j, debug, n, area1, area2;
l_float32  x1, y1, x2, y2, minfract, score, rankscore, threshscore;
NUMA      *nasum, *narem, *nasave, *nascore;
PIX       *pix1, *pix2;
PIXA      *pixa, *pixarem, *pixad;
PTA       *pta;
L_RECOG   *recog;

    if (ppixsave) *ppixsave = NULL;
    if (ppixrem) *ppixrem = NULL;
    if (!pixas)
        return (PIXA *)ERROR_PTR("pixas not defined", __func__, NULL);
    minscore = L_MIN(minscore, 1.0);
    if (minscore <= 0.0)
        minscore = DefaultMinScore;
    mintarget = L_MIN(mintarget, 3);
    if (mintarget <= 0)
        mintarget = DefaultMinTarget;
    if (minsize < 0)
        minsize = DefaultMinSetSize;

        /* Make a special height-scaled recognizer with average templates */
    debug = (ppixsave || ppixrem) ? 1 : 0;
    recog = recogCreateFromPixa(pixas, 0, 40, 0, 128, 1);
    if (!recog)
        return (PIXA *)ERROR_PTR("bad pixas; recog not made", __func__, NULL);
    if (recogAverageSamples(recog, debug) != 0) {
        recogDestroy(&recog);
        return (PIXA *)ERROR_PTR("bad templates", __func__, NULL);
    }

    nasave = (ppixsave) ? numaCreate(0) : NULL;
    pixarem = (ppixrem) ? pixaCreate(0) : NULL;
    narem = (ppixrem) ? numaCreate(0) : NULL;

    pixad = pixaCreate(0);
    for (i = 0; i < recog->setsize; i++) {
            /* Access the average template and values for scaled
             * images in this class */
        pix1 = pixaGetPix(recog->pixa, i, L_CLONE);
        ptaGetPt(recog->pta, i, &x1, &y1);
        numaGetIValue(recog->nasum, i, &area1);

            /* Get the scores for each sample in the class */
        pixa = pixaaGetPixa(recog->pixaa, i, L_CLONE);
        pta = ptaaGetPta(recog->ptaa, i, L_CLONE);  /* centroids */
        nasum = numaaGetNuma(recog->naasum, i, L_CLONE);  /* fg areas */
        n = pixaGetCount(pixa);
        nascore = numaCreate(n);
        for (j = 0; j < n; j++) {
            pix2 = pixaGetPix(pixa, j, L_CLONE);
            ptaGetPt(pta, j, &x2, &y2);  /* centroid average */
            numaGetIValue(nasum, j, &area2);  /* fg sum average */
            pixCorrelationScoreSimple(pix1, pix2, area1, area2,
                                      x1 - x2, y1 - y2, 5, 5,
                                      recog->sumtab, &score);
            numaAddNumber(nascore, score);
            if (debug && score == 0.0)  /* typ. large size difference */
                lept_stderr("Got 0 score for i = %d, j = %d\n", i, j);
            pixDestroy(&pix2);
        }
        pixDestroy(&pix1);

            /* Find the rankscore, corresponding to the 1.0 - minfract.
             * To attempt to maintain the minfract of templates, use as a
             * cutoff the minimum of minscore and the rank score.  However,
             * no template is saved with an actual score less than
             * that at least one template is kept. */
        minfract = (l_float32)mintarget / (l_float32)n;
        numaGetRankValue(nascore, 1.0 - minfract, NULL, 0, &rankscore);
        threshscore = L_MAX(LowerScoreThreshold,
                            L_MIN(minscore, rankscore));
        if (debug) {
            L_INFO("minscore = %4.2f, rankscore = %4.2f, threshscore = %4.2f\n",
                   __func__, minscore, rankscore, threshscore);
        }

            /* Save templates that are at or above threshold.
             * Toss any classes with less than %minsize templates. */
        for (j = 0; j < n; j++) {
            numaGetFValue(nascore, j, &score);
            pix1 = pixaaGetPix(recog->pixaa_u, i, j, L_COPY);
            if (score >= threshscore && n >= minsize) {
                pixaAddPix(pixad, pix1, L_INSERT);
                if (nasave) numaAddNumber(nasave, score);
            } else if (debug) {
                pixaAddPix(pixarem, pix1, L_INSERT);
                numaAddNumber(narem, score);
            } else {
                pixDestroy(&pix1);
            }
        }

        pixaDestroy(&pixa);
        ptaDestroy(&pta);
        numaDestroy(&nasum);
        numaDestroy(&nascore);
    }

    if (ppixsave) {
        *ppixsave = pixDisplayOutliers(pixad, nasave);
        numaDestroy(&nasave);
    }
    if (ppixrem) {
        *ppixrem = pixDisplayOutliers(pixarem, narem);
        pixaDestroy(&pixarem);
        numaDestroy(&narem);
    }
    recogDestroy(&recog);
    return pixad;
}


/*!
 * \brief   recogRemoveOutliers2()
 *
 * \param[in]   precog      addr of recog with unscaled labeled templates
 * \param[in]   minscore    keep everything with at least this score
 * \param[in]   minsize     minimum number of samples required for a class
 * \param[out]  ppixsave    [optional debug] saved templates, with scores
 * \param[out]  ppixrem     [optional debug] removed templates, with scores
 * \return  0 if OK, 1 on error.
 *
 * <pre>
 * Notes:
 *      (1) This is a convenience wrapper when using default parameters
 *          for the recog.  See pixaRemoveOutliers2() for details.
 *      (2) If this succeeds, the new recog replaces the input recog;
 *          if it fails, the input recog is destroyed.
 * </pre>
 */
l_ok
recogRemoveOutliers2(L_RECOG  **precog,
                     l_float32  minscore,
                     l_int32    minsize,
                     PIX      **ppixsave,
                     PIX      **ppixrem)
{
PIXA     *pixa1, *pixa2;
L_RECOG  *recog;

    if (!precog)
        return ERROR_INT("&recog not defined", __func__, 1);
    if (*precog == NULL)
        return ERROR_INT("recog not defined", __func__, 1);

        /* Extract the unscaled templates */
    pixa1 = recogExtractPixa(*precog);
    recogDestroy(precog);

    pixa2 = pixaRemoveOutliers2(pixa1, minscore, minsize, ppixsave, ppixrem);
    pixaDestroy(&pixa1);
    if (!pixa2)
        return ERROR_INT("failure to remove outliers", __func__, 1);

    recog = recogCreateFromPixa(pixa2, 0, 0, 0, 150, 1);
    pixaDestroy(&pixa2);
    if (!recog)
        return ERROR_INT("failure to make recog from pixa sans outliers",
                          __func__, 1);

    *precog = recog;
    return 0;
}


/*!
 * \brief   pixaRemoveOutliers2()
 *
 * \param[in]   pixas       unscaled labeled templates
 * \param[in]   minscore    keep everything with at least this score;
 *                          use -1.0 for default.
 * \param[in]   minsize     minimum number of samples required for a class;
 *                          use -1 for default.
 * \param[out]  ppixsave    [optional debug] saved templates, with scores
 * \param[out]  ppixrem     [optional debug] removed templates, with scores
 * \return  pixa   of unscaled templates to be kept, or NULL on error
 *
 * <pre>
 * Notes:
 *      (1) Removing outliers is particularly important when recognition
 *          goes against all the samples in the training set, as opposed
 *          to the averages for each class.  The reason is that we get
 *          an identification error if a mislabeled template is a best
 *          match for an input sample.
 *      (2) This method compares each template against the average templates
 *          of each class, and discards any template that has a higher
 *          correlation to a class different from its own.  It also
 *          sets a lower bound on correlation scores with its class average.
 *      (3) This is meant to be used on a BAR, where the templates all
 *          come from the same book; use minscore ~0.75.
 * </pre>
 */
PIXA *
pixaRemoveOutliers2(PIXA      *pixas,
                    l_float32  minscore,
                    l_int32    minsize,
                    PIX      **ppixsave,
                    PIX      **ppixrem)
{
l_int32    i, j, k, n, area1, area2, maxk, debug;
l_float32  x1, y1, x2, y2, score, maxscore;
NUMA      *nan, *nascore, *nasave;
PIX       *pix1, *pix2, *pix3;
PIXA      *pixarem, *pixad;
L_RECOG   *recog;

    if (ppixsave) *ppixsave = NULL;
    if (ppixrem) *ppixrem = NULL;
    if (!pixas)
        return (PIXA *)ERROR_PTR("pixas not defined", __func__, NULL);
    minscore = L_MIN(minscore, 1.0);
    if (minscore <= 0.0)
        minscore = DefaultMinScore;
    if (minsize < 0)
        minsize = DefaultMinSetSize;

        /* Make a special height-scaled recognizer with average templates */
    debug = (ppixsave || ppixrem) ? 1 : 0;
    recog = recogCreateFromPixa(pixas, 0, 40, 0, 128, 1);
    if (!recog)
        return (PIXA *)ERROR_PTR("bad pixas; recog not made", __func__, NULL);
    if (recogAverageSamples(recog, debug) != 0) {
        recogDestroy(&recog);
        return (PIXA *)ERROR_PTR("bad templates", __func__, NULL);
    }

    nasave = (ppixsave) ? numaCreate(0) : NULL;
    pixarem = (ppixrem) ? pixaCreate(0) : NULL;

    pixad = pixaCreate(0);
    pixaaGetCount(recog->pixaa, &nan);  /* number of templates in each class */
    for (i = 0; i < recog->setsize; i++) {
            /* Get the scores for each sample in the class, when comparing
             * with averages from all the classes. */
        numaGetIValue(nan, i, &n);
        for (j = 0; j < n; j++) {
            pix1 = pixaaGetPix(recog->pixaa, i, j, L_CLONE);
            ptaaGetPt(recog->ptaa, i, j, &x1, &y1);  /* centroid */
            numaaGetValue(recog->naasum, i, j, NULL, &area1);  /* fg sum */
            nascore = numaCreate(n);
            for (k = 0; k < recog->setsize; k++) {  /* average templates */
                pix2 = pixaGetPix(recog->pixa, k, L_CLONE);
                ptaGetPt(recog->pta, k, &x2, &y2);  /* average centroid */
                numaGetIValue(recog->nasum, k, &area2);  /* average fg sum */
                pixCorrelationScoreSimple(pix1, pix2, area1, area2,
                                          x1 - x2, y1 - y2, 5, 5,
                                          recog->sumtab, &score);
                numaAddNumber(nascore, score);
                pixDestroy(&pix2);
            }

                /* Save templates that are in the correct class and
                 * at or above threshold.  Toss any classes with less
                 * than %minsize templates. */
            numaGetMax(nascore, &maxscore, &maxk);
            if (maxk == i && maxscore >= minscore && n >= minsize) {
                    /* save it */
                pix3 = pixaaGetPix(recog->pixaa_u, i, j, L_COPY);
                pixaAddPix(pixad, pix3, L_INSERT);
                if (nasave) numaAddNumber(nasave, maxscore);
            } else if (ppixrem) {  /* outlier */
                pix3 = recogDisplayOutlier(recog, i, j, maxk, maxscore);
                pixaAddPix(pixarem, pix3, L_INSERT);
            }
            numaDestroy(&nascore);
            pixDestroy(&pix1);
        }
    }

    if (ppixsave) {
        *ppixsave = pixDisplayOutliers(pixad, nasave);
        numaDestroy(&nasave);
    }
    if (ppixrem) {
        *ppixrem = pixaDisplayTiledInRows(pixarem, 32, 1500, 1.0, 0, 20, 2);
        pixaDestroy(&pixarem);
    }

    numaDestroy(&nan);
    recogDestroy(&recog);
    return pixad;
}


/*------------------------------------------------------------------------*
 *                       Training on unlabeled data                       *
 *------------------------------------------------------------------------*/
/*!
 * \brief   recogTrainFromBoot()
 *
 * \param[in]    recogboot   labeled boot recognizer
 * \param[in]    pixas       set of unlabeled input characters
 * \param[in]    minscore    min score for accepting the example; e.g., 0.75
 * \param[in]    threshold   for binarization, if needed
 * \param[in]    debug       1 for debug output saved to recogboot; 0 otherwise
 * \return  pixad   labeled version of input pixas, trained on a BSR,
 *                  or NULL on error
 *
 * <pre>
 * Notes:
 *      (1) This takes %pixas of unscaled single characters and %recboot,
 *          a bootstrep recognizer (BSR) that has been set up with parameters
 *            * scaleh: scale all templates to this height
 *            * linew: width of normalized strokes, or 0 if using
 *              the input image
 *          It modifies the pix in %pixas accordingly and correlates
 *          with the templates in the BSR.  It returns those input
 *          images in %pixas whose best correlation with the BSR is at
 *          or above %minscore.  The returned pix have added text labels
 *          for the text string of the class to which the best
 *          correlated template belongs.
 *      (2) Identification occurs in scaled mode (typically with h = 40),
 *          optionally using a width-normalized line images derived
 *          from those in %pixas.
 * </pre>
 */
PIXA  *
recogTrainFromBoot(L_RECOG   *recogboot,
                   PIXA      *pixas,
                   l_float32  minscore,
                   l_int32    threshold,
                   l_int32    debug)
{
char      *text;
l_int32    i, n, same, maxd, scaleh, linew;
l_float32  score;
PIX       *pix1, *pix2, *pixdb = NULL;
PIXA      *pixa1, *pixa2, *pixa3, *pixad;

    if (!recogboot)
        return (PIXA *)ERROR_PTR("recogboot not defined", __func__, NULL);
    if (!pixas)
        return (PIXA *)ERROR_PTR("pixas not defined", __func__, NULL);

        /* Make sure all input pix are 1 bpp */
    if ((n = pixaGetCount(pixas)) == 0)
        return (PIXA *)ERROR_PTR("no pix in pixa", __func__, NULL);
    pixaVerifyDepth(pixas, &same, &maxd);
    if (maxd == 1) {
        pixa1 = pixaCopy(pixas, L_COPY);
    } else {
        pixa1 = pixaCreate(n);
        for (i = 0; i < n; i++) {
            pix1 = pixaGetPix(pixas, i, L_CLONE);
            pix2 = pixConvertTo1(pix1, threshold);
            pixaAddPix(pixa1, pix2, L_INSERT);
            pixDestroy(&pix1);
        }
    }

        /* Scale the input images to match the BSR */
    scaleh = recogboot->scaleh;
    linew = recogboot->linew;
    pixa2 = pixaCreate(n);
    for (i = 0; i < n; i++) {
        pix1 = pixaGetPix(pixa1, i, L_CLONE);
        pix2 = pixScaleToSize(pix1, 0, scaleh);
        pixaAddPix(pixa2, pix2, L_INSERT);
        pixDestroy(&pix1);
    }
    pixaDestroy(&pixa1);

        /* Optionally convert to width-normalized line */
    if (linew > 0)
        pixa3 = pixaSetStrokeWidth(pixa2, linew, 4, 8);
    else
        pixa3 = pixaCopy(pixa2, L_CLONE);
    pixaDestroy(&pixa2);

        /* Identify using recogboot */
    n = pixaGetCount(pixa3);
    pixad = pixaCreate(n);
    for (i = 0; i < n; i++) {
        pix1 = pixaGetPix(pixa3, i, L_COPY);
        pixSetText(pix1, NULL);  /* remove any existing text or labelling */
        if (!debug) {
            recogIdentifyPix(recogboot, pix1, NULL);
        } else {
            recogIdentifyPix(recogboot, pix1, &pixdb);
            pixaAddPix(recogboot->pixadb_boot, pixdb, L_INSERT);
        }
        rchExtract(recogboot->rch, NULL, &score, &text, NULL, NULL, NULL, NULL);
        if (score >= minscore) {
            pix2 = pixaGetPix(pixas, i, L_COPY);
            pixSetText(pix2, text);
            pixaAddPix(pixad, pix2, L_INSERT);
            pixaAddPix(recogboot->pixadb_boot, pixdb, L_COPY);
        }
        LEPT_FREE(text);
        pixDestroy(&pix1);
    }
    pixaDestroy(&pixa3);

    return pixad;
}


/*------------------------------------------------------------------------*
 *                     Padding the digit training set                     *
 *------------------------------------------------------------------------*/
/*!
 * \brief   recogPadDigitTrainingSet()
 *
 * \param[in,out]   precog    trained; if padding is needed, it is replaced
 *                            by a a new padded recog
 * \param[in]       scaleh    must be > 0; suggest ~40.
 * \param[in]       linew     use 0 for original scanned images
 * \return       0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *      (1) This is a no-op if padding is not needed.  However,
 *          if it is, this replaces the input recog with a new recog,
 *          padded appropriately with templates from a boot recognizer,
 *          and set up with correlation templates derived from
 *          %scaleh and %linew.
 * </pre>
 */
l_ok
recogPadDigitTrainingSet(L_RECOG  **precog,
                         l_int32    scaleh,
                         l_int32    linew)
{
PIXA     *pixa;
L_RECOG  *recog1, *recog2;
SARRAY   *sa;

    if (!precog)
        return ERROR_INT("&recog not defined", __func__, 1);
    recog1 = *precog;

    recogIsPaddingNeeded(recog1, &sa);
    if (!sa) return 0;

        /* Get a new pixa with the padding templates added */
    pixa = recogAddDigitPadTemplates(recog1, sa);
    sarrayDestroy(&sa);
    if (!pixa)
        return ERROR_INT("pixa not made", __func__, 1);

        /* Need to use templates that are scaled to a fixed height. */
    if (scaleh <= 0) {
        L_WARNING("templates must be scaled to fixed height; using %d\n",
                  __func__, 40);
        scaleh = 40;
    }

        /* Create a hybrid recog, composed of templates from both
         * the original and bootstrap sources. */
    recog2 = recogCreateFromPixa(pixa, 0, scaleh, linew, recog1->threshold,
                                 recog1->maxyshift);
    pixaDestroy(&pixa);
    recogDestroy(precog);
    *precog = recog2;
    return 0;
}


/*!
 * \brief   recogIsPaddingNeeded()
 *
 * \param[in]    recog   trained
 * \param[out]   psa     addr of returned string containing text value
 * \return       1 on error; 0 if OK, whether or not additional padding
 *               templates are required.
 *
 * <pre>
 * Notes:
 *      (1) This returns a string array in &sa containing character values
 *          for which extra templates are needed; this sarray is
 *          used by recogGetPadTemplates().  It returns NULL
 *          if no padding templates are needed.
 * </pre>
 */
l_int32
recogIsPaddingNeeded(L_RECOG  *recog,
                     SARRAY  **psa)
{
char      *str;
l_int32    i, nt, min_nopad, nclass, allclasses;
l_float32  minval;
NUMA      *naclass;
SARRAY    *sa;

    if (!psa)
        return ERROR_INT("&sa not defined", __func__, 1);
    *psa = NULL;
    if (!recog)
        return ERROR_INT("recog not defined", __func__, 1);

        /* Do we have samples from all classes? */
    nclass = pixaaGetCount(recog->pixaa_u, &naclass);  /* unscaled bitmaps */
    allclasses = (nclass == recog->charset_size) ? 1 : 0;

        /* Are there enough samples in each class already? */
    min_nopad = recog->min_nopad;
    numaGetMin(naclass, &minval, NULL);
    if (allclasses && (minval >= min_nopad)) {
        numaDestroy(&naclass);
        return 0;
    }

        /* Are any classes not represented? */
    sa = recogAddMissingClassStrings(recog);
    *psa = sa;

        /* Are any other classes under-represented? */
    for (i = 0; i < nclass; i++) {
        numaGetIValue(naclass, i, &nt);
        if (nt < min_nopad) {
            str = sarrayGetString(recog->sa_text, i, L_COPY);
            sarrayAddString(sa, str, L_INSERT);
        }
    }
    numaDestroy(&naclass);
    return 0;
}


/*!
 * \brief   recogAddMissingClassStrings()
 *
 * \param[in]    recog   trained
 * \return       sa  of class string missing in %recog, or NULL on error
 *
 * <pre>
 * Notes:
 *      (1) This returns an empty %sa if there is at least one template
 *          in each class in %recog.
 * </pre>
 */
static SARRAY  *
recogAddMissingClassStrings(L_RECOG  *recog)
{
char    *text;
char     str[4];
l_int32  i, nclass, index, ival;
NUMA    *na;
SARRAY  *sa;

    if (!recog)
        return (SARRAY *)ERROR_PTR("recog not defined", __func__, NULL);

        /* Only handling digits */
    nclass = pixaaGetCount(recog->pixaa_u, NULL);  /* unscaled bitmaps */
    if (recog->charset_type != 1 || nclass == 10)
        return sarrayCreate(0);  /* empty */

        /* Make an indicator array for missing classes */
    na = numaCreate(0);
    sa = sarrayCreate(0);
    for (i = 0; i < recog->charset_size; i++)
         numaAddNumber(na, 1);
    for (i = 0; i < nclass; i++) {
        text = sarrayGetString(recog->sa_text, i, L_NOCOPY);
        index = text[0] - '0';
        numaSetValue(na, index, 0);
    }

        /* Convert to string and add to output */
    for (i = 0; i < nclass; i++) {
        numaGetIValue(na, i, &ival);
        if (ival == 1) {
            str[0] = '0' + i;
            str[1] = '\0';
            sarrayAddString(sa, str, L_COPY);
        }
    }
    numaDestroy(&na);
    return sa;
}


/*!
 * \brief   recogAddDigitPadTemplates()
 *
 * \param[in]    recog   trained
 * \param[in]    sa      set of text strings that need to be padded
 * \return  pixa   of all templates from %recog and the additional pad
 *                 templates from a boot recognizer; or NULL on error
 *
 * <pre>
 * Notes:
 *      (1) Call recogIsPaddingNeeded() first, which returns %sa of
 *          template text strings for classes where more templates
 *          are needed.
 * </pre>
 */
PIXA  *
recogAddDigitPadTemplates(L_RECOG  *recog,
                          SARRAY   *sa)
{
char    *str, *text;
l_int32  i, j, n, nt;
PIX     *pix;
PIXA    *pixa1, *pixa2;

    if (!recog)
        return (PIXA *)ERROR_PTR("recog not defined", __func__, NULL);
    if (!sa)
        return (PIXA *)ERROR_PTR("sa not defined", __func__, NULL);
    if (recogCharsetAvailable(recog->charset_type) == FALSE)
        return (PIXA *)ERROR_PTR("boot charset not available", __func__, NULL);

        /* Make boot recog templates */
    pixa1 = recogMakeBootDigitTemplates(0, 0);
    n = pixaGetCount(pixa1);

        /* Extract the unscaled templates from %recog */
    pixa2 = recogExtractPixa(recog);

        /* Add selected boot recog templates based on the text strings in sa */
    nt = sarrayGetCount(sa);
    for (i = 0; i < n; i++) {
        pix = pixaGetPix(pixa1, i, L_CLONE);
        text = pixGetText(pix);
        for (j = 0; j < nt; j++) {
            str = sarrayGetString(sa, j, L_NOCOPY);
            if (!strcmp(text, str)) {
                pixaAddPix(pixa2, pix, L_COPY);
                break;
            }
        }
        pixDestroy(&pix);
    }

    pixaDestroy(&pixa1);
    return pixa2;
}


/*!
 * \brief   recogCharsetAvailable()
 *
 * \param[in]    type     of charset for padding
 * \return  1 if available; 0 if not.
 */
static l_int32
recogCharsetAvailable(l_int32  type)
{
l_int32  ret;

    switch (type)
    {
    case L_ARABIC_NUMERALS:
        ret = TRUE;
        break;
    case L_LC_ROMAN_NUMERALS:
    case L_UC_ROMAN_NUMERALS:
    case L_LC_ALPHA:
    case L_UC_ALPHA:
        L_INFO("charset type %d not available\n", __func__, type);
        ret = FALSE;
        break;
    default:
        L_INFO("charset type %d is unknown\n", __func__, type);
        ret = FALSE;
        break;
    }

    return ret;
}


/*------------------------------------------------------------------------*
 *                      Making a boot digit recognizer                    *
 *------------------------------------------------------------------------*/
/*!
 * \brief   recogMakeBootDigitRecog()
 *
 * \param[in]    nsamp       number of samples of each digit; or 0
 * \param[in]    scaleh      scale all heights to this; typ. use 40
 * \param[in]    linew       normalized line width; typ. use 5; 0 to skip
 * \param[in]    maxyshift   from nominal centroid alignment; typically 0 or 1
 * \param[in]    debug       1 for showing templates; 0 otherwise
 * \return  recog, or NULL on error
 *
 * <pre>
 * Notes:
 *     (1) This takes a set of pre-computed, labeled pixa of single
 *         digits, and generates a recognizer from them.
 *         The templates used in the recognizer can be modified by:
 *         - scaling (isotropically to fixed height)
 *         - generating a skeleton and thickening so that all strokes
 *           have the same width.
 *     (2) The resulting templates are scaled versions of either the
 *         input bitmaps or images with fixed line widths.  To use the
 *         input bitmaps, set %linew = 0; otherwise, set %linew to the
 *         desired line width.
 *     (3) If %nsamp == 0, this uses and extends the output from
 *         three boot generators:
 *            l_bootnum_gen1, l_bootnum_gen2, l_bootnum_gen3.
 *         Otherwise, it uses exactly %nsamp templates of each digit,
 *         extracted by l_bootnum_gen4.
 * </pre>
 */
L_RECOG  *
recogMakeBootDigitRecog(l_int32  nsamp,
                        l_int32  scaleh,
                        l_int32  linew,
                        l_int32  maxyshift,
                        l_int32  debug)

{
PIXA     *pixa;
L_RECOG  *recog;

        /* Get the templates, extended by horizontal scaling */
    pixa = recogMakeBootDigitTemplates(nsamp, debug);

        /* Make the boot recog; recogModifyTemplate() will scale the
         * templates and optionally turn them into strokes of fixed width. */
    recog = recogCreateFromPixa(pixa, 0, scaleh, linew, 128, maxyshift);
    pixaDestroy(&pixa);
    if (debug)
        recogShowContent(stderr, recog, 0, 1);

    return recog;
}


/*!
 * \brief   recogMakeBootDigitTemplates()
 *
 * \param[in]    nsamp     number of samples of each digit; or 0
 * \param[in]    debug     1 for display of templates
 * \return  pixa   of templates; or NULL on error
 *
 * <pre>
 * Notes:
 *     (1) See recogMakeBootDigitRecog().
 * </pre>
 */
PIXA  *
recogMakeBootDigitTemplates(l_int32  nsamp,
                            l_int32  debug)
{
NUMA  *na1;
PIX   *pix1, *pix2, *pix3;
PIXA  *pixa1, *pixa2, *pixa3;

    if (nsamp > 0) {
        pixa1 = l_bootnum_gen4(nsamp);
        if (debug) {
            pix1 = pixaDisplayTiledWithText(pixa1, 1500, 1.0, 10,
                                            2, 6, 0xff000000);
            pixDisplay(pix1, 0, 0);
            pixDestroy(&pix1);
        }
        return pixa1;
    }

        /* Else, generate from 3 pixa */
    pixa1 = l_bootnum_gen1();
    pixa2 = l_bootnum_gen2();
    pixa3 = l_bootnum_gen3();
    if (debug) {
        pix1 = pixaDisplayTiledWithText(pixa1, 1500, 1.0, 10, 2, 6, 0xff000000);
        pix2 = pixaDisplayTiledWithText(pixa2, 1500, 1.0, 10, 2, 6, 0xff000000);
        pix3 = pixaDisplayTiledWithText(pixa3, 1500, 1.0, 10, 2, 6, 0xff000000);
        pixDisplay(pix1, 0, 0);
        pixDisplay(pix2, 600, 0);
        pixDisplay(pix3, 1200, 0);
        pixDestroy(&pix1);
        pixDestroy(&pix2);
        pixDestroy(&pix3);
    }
    pixaJoin(pixa1, pixa2, 0, -1);
    pixaJoin(pixa1, pixa3, 0, -1);
    pixaDestroy(&pixa2);
    pixaDestroy(&pixa3);

        /* Extend by horizontal scaling */
    na1 = numaCreate(4);
    numaAddNumber(na1, 0.9f);
    numaAddNumber(na1, 1.1f);
    numaAddNumber(na1, 1.2f);
    pixa2 = pixaExtendByScaling(pixa1, na1, L_HORIZ, 1);

    pixaDestroy(&pixa1);
    numaDestroy(&na1);
    return pixa2;
}


/*------------------------------------------------------------------------*
 *                               Debugging                                *
 *------------------------------------------------------------------------*/
/*!
 * \brief   recogShowContent()
 *
 * \param[in]    fp       file stream
 * \param[in]    recog
 * \param[in]    index    for naming of output files of template images
 * \param[in]    display  1 for showing template images; 0 otherwise
 * \return  0 if OK, 1 on error
 */
l_ok
recogShowContent(FILE     *fp,
                 L_RECOG  *recog,
                 l_int32   index,
                 l_int32   display)
{
char     buf[128];
l_int32  i, val, count;
PIX     *pix;
NUMA    *na;

    if (!fp)
        return ERROR_INT("stream not defined", __func__, 1);
    if (!recog)
        return ERROR_INT("recog not defined", __func__, 1);

    fprintf(fp, "Debug print of recog contents\n");
    fprintf(fp, "  Setsize: %d\n", recog->setsize);
    fprintf(fp, "  Binarization threshold: %d\n", recog->threshold);
    fprintf(fp, "  Maximum matching y-jiggle: %d\n", recog->maxyshift);
    if (recog->linew <= 0)
        fprintf(fp, "  Using image templates for matching\n");
    else
        fprintf(fp, "  Using templates with fixed line width for matching\n");
    if (recog->scalew == 0)
        fprintf(fp, "  No width scaling of templates\n");
    else
        fprintf(fp, "  Template width scaled to %d\n", recog->scalew);
    if (recog->scaleh == 0)
        fprintf(fp, "  No height scaling of templates\n");
    else
        fprintf(fp, "  Template height scaled to %d\n", recog->scaleh);
    fprintf(fp, "  Number of samples in each class:\n");
    pixaaGetCount(recog->pixaa_u, &na);
    for (i = 0; i < recog->setsize; i++) {
        l_dnaGetIValue(recog->dna_tochar, i, &val);
        numaGetIValue(na, i, &count);
        if (val < 128)
            fprintf(fp, "    class %d, char %c:   %d\n", i, val, count);
        else
            fprintf(fp, "    class %d, val %d:   %d\n", i, val, count);
    }
    numaDestroy(&na);

    if (display) {
        lept_mkdir("lept/recog");
        pix = pixaaDisplayByPixa(recog->pixaa_u, 50, 1.0, 20, 20, 0);
        snprintf(buf, sizeof(buf), "/tmp/lept/recog/templates_u.%d.png", index);
        pixWriteDebug(buf, pix, IFF_PNG);
        pixDisplay(pix, 0, 200 * index);
        pixDestroy(&pix);
        if (recog->train_done) {
            pix = pixaaDisplayByPixa(recog->pixaa, 50, 1.0, 20, 20, 0);
            snprintf(buf, sizeof(buf),
                     "/tmp/lept/recog/templates.%d.png", index);
            pixWriteDebug(buf, pix, IFF_PNG);
            pixDisplay(pix, 800, 200 * index);
            pixDestroy(&pix);
        }
    }
    return 0;
}


/*!
 * \brief   recogDebugAverages()
 *
 * \param[in]    recog    addr of recog
 * \param[in]    debug     0 no output; 1 for images; 2 for text; 3 for both
 * \return  0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *      (1) Generates an image that pairs each of the input images used
 *          in training with the average template that it is best
 *          correlated to.  This is written into the recog.
 *      (2) It also generates pixa_tr of all the input training images,
 *          which can be used, e.g., in recogShowMatchesInRange().
 *      (3) Returns an error if the averaging function finds bad classes.
 * </pre>
 */
l_ok
recogDebugAverages(L_RECOG  *recog,
                   l_int32   debug)
{
l_int32    i, j, n, np, index;
l_float32  score;
PIX       *pix1, *pix2, *pix3;
PIXA      *pixa, *pixat;
PIXAA     *paa1, *paa2;

    if (!recog)
        return ERROR_INT("recog not defined", __func__, 1);

        /* Mark the training as finished if necessary, and make sure
         * that the average templates have been built. */
    if (recogAverageSamples(recog, 0) != 0)
        return ERROR_INT("averaging failed", __func__, 1);

        /* Save a pixa of all the training examples */
    paa1 = recog->pixaa;
    if (!recog->pixa_tr)
        recog->pixa_tr = pixaaFlattenToPixa(paa1, NULL, L_CLONE);

        /* Destroy any existing image and make a new one */
    if (recog->pixdb_ave)
        pixDestroy(&recog->pixdb_ave);
    n = pixaaGetCount(paa1, NULL);
    paa2 = pixaaCreate(n);
    for (i = 0; i < n; i++) {
        pixa = pixaCreate(0);
        pixat = pixaaGetPixa(paa1, i, L_CLONE);
        np = pixaGetCount(pixat);
        for (j = 0; j < np; j++) {
            pix1 = pixaaGetPix(paa1, i, j, L_CLONE);
            recogIdentifyPix(recog, pix1, &pix2);
            rchExtract(recog->rch, &index, &score, NULL, NULL, NULL,
                       NULL, NULL);
            if (debug >= 2)
                lept_stderr("index = %d, score = %7.3f\n", index, score);
            pix3 = pixAddBorder(pix2, 2, 1);
            pixaAddPix(pixa, pix3, L_INSERT);
            pixDestroy(&pix1);
            pixDestroy(&pix2);
        }
        pixaaAddPixa(paa2, pixa, L_INSERT);
        pixaDestroy(&pixat);
    }
    recog->pixdb_ave = pixaaDisplayByPixa(paa2, 50, 1.0, 20, 20, 0);
    if (debug % 2) {
        lept_mkdir("lept/recog");
        pixWriteDebug("/tmp/lept/recog/templ_match.png", recog->pixdb_ave,
                      IFF_PNG);
        pixDisplay(recog->pixdb_ave, 100, 100);
    }

    pixaaDestroy(&paa2);
    return 0;
}


/*!
 * \brief   recogShowAverageTemplates()
 *
 * \param[in]    recog
 * \return  0 on success, 1 on failure
 *
 * <pre>
 * Notes:
 *      (1) This debug routine generates a display of the averaged templates,
 *          both scaled and unscaled, with the centroid visible in red.
 * </pre>
 */
l_int32
recogShowAverageTemplates(L_RECOG  *recog)
{
l_int32    i, size;
l_float32  x, y;
PIX       *pix1, *pix2, *pixr;
PIXA      *pixat, *pixadb;

    if (!recog)
        return ERROR_INT("recog not defined", __func__, 1);

    lept_stderr("min/max width_u = (%d,%d); min/max height_u = (%d,%d)\n",
                recog->minwidth_u, recog->maxwidth_u,
                recog->minheight_u, recog->maxheight_u);
    lept_stderr("min splitw = %d, max splith = %d\n",
                recog->min_splitw, recog->max_splith);

    pixaDestroy(&recog->pixadb_ave);

    pixr = pixCreate(3, 3, 32);  /* 3x3 red square for centroid location */
    pixSetAllArbitrary(pixr, 0xff000000);
    pixadb = pixaCreate(2);

        /* Unscaled bitmaps */
    size = recog->setsize;
    pixat = pixaCreate(size);
    for (i = 0; i < size; i++) {
        if ((pix1 = pixaGetPix(recog->pixa_u, i, L_CLONE)) == NULL)
            continue;
        pix2 = pixConvertTo32(pix1);
        ptaGetPt(recog->pta_u, i, &x, &y);
        pixRasterop(pix2, (l_int32)(x - 0.5), (l_int32)(y - 0.5), 3, 3,
                    PIX_SRC, pixr, 0, 0);
        pixaAddPix(pixat, pix2, L_INSERT);
        pixDestroy(&pix1);
    }
    pix1 = pixaDisplayTiledInRows(pixat, 32, 3000, 1.0, 0, 20, 0);
    pixaAddPix(pixadb, pix1, L_INSERT);
    pixDisplay(pix1, 100, 100);
    pixaDestroy(&pixat);

        /* Scaled bitmaps */
    pixat = pixaCreate(size);
    for (i = 0; i < size; i++) {
        if ((pix1 = pixaGetPix(recog->pixa, i, L_CLONE)) == NULL)
            continue;
        pix2 = pixConvertTo32(pix1);
        ptaGetPt(recog->pta, i, &x, &y);
        pixRasterop(pix2, (l_int32)(x - 0.5), (l_int32)(y - 0.5), 3, 3,
                    PIX_SRC, pixr, 0, 0);
        pixaAddPix(pixat, pix2, L_INSERT);
        pixDestroy(&pix1);
    }
    pix1 = pixaDisplayTiledInRows(pixat, 32, 3000, 1.0, 0, 20, 0);
    pixaAddPix(pixadb, pix1, L_INSERT);
    pixDisplay(pix1, 100, 100);
    pixaDestroy(&pixat);
    pixDestroy(&pixr);
    recog->pixadb_ave = pixadb;
    return 0;
}


/*!
 * \brief   pixDisplayOutliers()
 *
 * \param[in]    pixas    unscaled labeled templates
 * \param[in]    nas      scores of templates (against class averages)
 * \return  pix    tiled pixa with text and scores, or NULL on failure
 *
 * <pre>
 * Notes:
 *      (1) This debug routine is called from recogRemoveOutliers2(),
 *          and takes the saved templates and their scores as input.
 * </pre>
 */
static PIX  *
pixDisplayOutliers(PIXA  *pixas,
                   NUMA  *nas)
{
char      *text;
char       buf[16];
l_int32    i, n;
l_float32  fval;
PIX       *pix1, *pix2;
PIXA      *pixa1;

    if (!pixas)
        return (PIX *)ERROR_PTR("pixas not defined", __func__, NULL);
    if (!nas)
        return (PIX *)ERROR_PTR("nas not defined", __func__, NULL);
    n = pixaGetCount(pixas);
    if (numaGetCount(nas) != n)
        return (PIX *)ERROR_PTR("pixas and nas sizes differ", __func__, NULL);

    pixa1 = pixaCreate(n);
    for (i = 0; i < n; i++) {
        pix1 = pixaGetPix(pixas, i, L_CLONE);
        pix2 = pixAddBlackOrWhiteBorder(pix1, 25, 25, 0, 0, L_GET_WHITE_VAL);
        text = pixGetText(pix1);
        numaGetFValue(nas, i, &fval);
        snprintf(buf, sizeof(buf), "'%s': %5.2f", text, fval);
        pixSetText(pix2, buf);
        pixaAddPix(pixa1, pix2, L_INSERT);
        pixDestroy(&pix1);
    }
    pix1 = pixaDisplayTiledWithText(pixa1, 1500, 1.0, 20, 2, 6, 0xff000000);
    pixaDestroy(&pixa1);
    return pix1;
}


/*!
 * \brief   recogDisplayOutlier()
 *
 * \param[in]    recog
 * \param[in]    iclass     sample is in this class
 * \param[in]    jsamp      index of sample is class i
 * \param[in]    maxclass   index of class with closest average to sample
 * \param[in]    maxscore   score of sample with average of class %maxclass
 * \return  pix  sample and template images, with score, or NULL on error
 *
 * <pre>
 * Notes:
 *      (1) This shows three templates, side-by-side:
 *          - The outlier sample
 *          - The average template from the same class
 *          - The average class template that best matched the outlier sample
 * </pre>
 */
static PIX  *
recogDisplayOutlier(L_RECOG   *recog,
                    l_int32    iclass,
                    l_int32    jsamp,
                    l_int32    maxclass,
                    l_float32  maxscore)
{
char   buf[64];
PIX   *pix1, *pix2, *pix3, *pix4, *pix5;
PIXA  *pixa;

    if (!recog)
        return (PIX *)ERROR_PTR("recog not defined", __func__, NULL);

    pix1 = pixaaGetPix(recog->pixaa, iclass, jsamp, L_CLONE);
    pix2 = pixaGetPix(recog->pixa, iclass, L_CLONE);
    pix3 = pixaGetPix(recog->pixa, maxclass, L_CLONE);
    pixa = pixaCreate(3);
    pixaAddPix(pixa, pix1, L_INSERT);
    pixaAddPix(pixa, pix2, L_INSERT);
    pixaAddPix(pixa, pix3, L_INSERT);
    pix4 = pixaDisplayTiledInRows(pixa, 32, 400, 2.0, 0, 12, 2);
    snprintf(buf, sizeof(buf), "C=%d, BAC=%d, S=%4.2f", iclass, maxclass,
             maxscore);
    pix5 = pixAddSingleTextblock(pix4, recog->bmf, buf, 0xff000000,
                                 L_ADD_BELOW, NULL);
    pixDestroy(&pix4);
    pixaDestroy(&pixa);
    return pix5;
}


/*!
 * \brief   recogShowMatchesInRange()
 *
 * \param[in]    recog
 * \param[in]    pixa        of 1 bpp images to match
 * \param[in]    minscore    min score to include output
 * \param[in]    maxscore    max score to include output
 * \param[in]    display     1 to display the result
 * \return  0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *      (1) This gives a visual output of the best matches for a given
 *          range of scores.  Each pair of images can optionally be
 *          labeled with the index of the best match and the correlation.
 *      (2) To use this, save a set of 1 bpp images (labeled or
 *          unlabeled) that can be given to a recognizer in a pixa.
 *          Then call this function with the pixa and parameters
 *          to filter a range of scores.
 * </pre>
 */
l_ok
recogShowMatchesInRange(L_RECOG   *recog,
                        PIXA      *pixa,
                        l_float32  minscore,
                        l_float32  maxscore,
                        l_int32    display)
{
l_int32    i, n, index, depth;
l_float32  score;
NUMA      *nascore, *naindex;
PIX       *pix1, *pix2;
PIXA      *pixa1, *pixa2;

    if (!recog)
        return ERROR_INT("recog not defined", __func__, 1);
    if (!pixa)
        return ERROR_INT("pixa not defined", __func__, 1);

        /* Run the recognizer on the set of images */
    n = pixaGetCount(pixa);
    nascore = numaCreate(n);
    naindex = numaCreate(n);
    pixa1 = pixaCreate(n);
    for (i = 0; i < n; i++) {
        pix1 = pixaGetPix(pixa, i, L_CLONE);
        recogIdentifyPix(recog, pix1, &pix2);
        rchExtract(recog->rch, &index, &score, NULL, NULL, NULL, NULL, NULL);
        numaAddNumber(nascore, score);
        numaAddNumber(naindex, index);
        pixaAddPix(pixa1, pix2, L_INSERT);
        pixDestroy(&pix1);
    }

        /* Filter the set and optionally add text to each */
    pixa2 = pixaCreate(n);
    depth = 1;
    for (i = 0; i < n; i++) {
        numaGetFValue(nascore, i, &score);
        if (score < minscore || score > maxscore) continue;
        pix1 = pixaGetPix(pixa1, i, L_CLONE);
        numaGetIValue(naindex, i, &index);
        pix2 = recogShowMatch(recog, pix1, NULL, NULL, index, score);
        if (i == 0) depth = pixGetDepth(pix2);
        pixaAddPix(pixa2, pix2, L_INSERT);
        pixDestroy(&pix1);
    }

        /* Package it up */
    pixDestroy(&recog->pixdb_range);
    if (pixaGetCount(pixa2) > 0) {
        recog->pixdb_range =
            pixaDisplayTiledInRows(pixa2, depth, 2500, 1.0, 0, 20, 1);
        if (display)
            pixDisplay(recog->pixdb_range, 300, 100);
    } else {
        L_INFO("no character matches in the range of scores\n", __func__);
    }

    pixaDestroy(&pixa1);
    pixaDestroy(&pixa2);
    numaDestroy(&nascore);
    numaDestroy(&naindex);
    return 0;
}


/*!
 * \brief   recogShowMatch()
 *
 * \param[in]    recog
 * \param[in]    pix1    input pix; several possibilities
 * \param[in]    pix2    [optional] matching template
 * \param[in]    box     [optional] region in pix1 for which pix2 matches
 * \param[in]    index   index of matching template; use -1 to disable printing
 * \param[in]    score   score of match
 * \return  pixd pair of images, showing input pix and best template,
 *                    optionally with matching information, or NULL on error.
 *
 * <pre>
 * Notes:
 *      (1) pix1 can be one of these:
 *          (a) The input pix alone, which can be either a single character
 *              (box == NULL) or several characters that need to be
 *              segmented.  If more than character is present, the box
 *              region is displayed with an outline.
 *          (b) Both the input pix and the matching template.  In this case,
 *              pix2 and box will both be null.
 *      (2) If the bmf has been made (by a call to recogMakeBmf())
 *          and the index >= 0, the text field, match score and index
 *          will be rendered; otherwise their values will be ignored.
 * </pre>
 */
PIX *
recogShowMatch(L_RECOG   *recog,
               PIX       *pix1,
               PIX       *pix2,
               BOX       *box,
               l_int32    index,
               l_float32  score)
{
char    buf[32];
char   *text;
L_BMF  *bmf;
PIX    *pix3, *pix4, *pix5, *pixd;
PIXA   *pixa;

    if (!recog)
        return (PIX *)ERROR_PTR("recog not defined", __func__, NULL);
    if (!pix1)
        return (PIX *)ERROR_PTR("pix1 not defined", __func__, NULL);

    bmf = (recog->bmf && index >= 0) ? recog->bmf : NULL;
    if (!pix2 && !box && !bmf)  /* nothing to do */
        return pixCopy(NULL, pix1);

    pix3 = pixConvertTo32(pix1);
    if (box)
        pixRenderBoxArb(pix3, box, 1, 255, 0, 0);

    if (pix2) {
        pixa = pixaCreate(2);
        pixaAddPix(pixa, pix3, L_CLONE);
        pixaAddPix(pixa, pix2, L_CLONE);
        pix4 = pixaDisplayTiledInRows(pixa, 1, 500, 1.0, 0, 15, 0);
        pixaDestroy(&pixa);
    } else {
        pix4 = pixCopy(NULL, pix3);
    }
    pixDestroy(&pix3);

    if (bmf) {
        pix5 = pixAddBorderGeneral(pix4, 55, 55, 0, 0, 0xffffff00);
        recogGetClassString(recog, index, &text);
        snprintf(buf, sizeof(buf), "C=%s, S=%4.3f, I=%d", text, score, index);
        pixd = pixAddSingleTextblock(pix5, bmf, buf, 0xff000000,
                                     L_ADD_BELOW, NULL);
        pixDestroy(&pix5);
        LEPT_FREE(text);
    } else {
        pixd = pixClone(pix4);
    }
    pixDestroy(&pix4);

    return pixd;
}
