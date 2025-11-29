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
 * \file recogbasic.c
 * <pre>
 *
 *      Recog creation, destruction and access
 *         L_RECOG            *recogCreateFromRecog()
 *         L_RECOG            *recogCreateFromPixa()
 *         L_RECOG            *recogCreateFromPixaNoFinish()
 *         L_RECOG            *recogCreate()
 *         void                recogDestroy()
 *
 *      Recog accessors
 *         l_int32             recogGetCount()
 *         l_int32             recogSetParams()
 *         static l_int32      recogGetCharsetSize()
 *
 *      Character/index lookup
 *         l_int32             recogGetClassIndex()
 *         l_int32             recogStringToIndex()
 *         l_int32             recogGetClassString()
 *         l_int32             l_convertCharstrToInt()
 *
 *      Serialization
 *         L_RECOG            *recogRead()
 *         L_RECOG            *recogReadStream()
 *         L_RECOG            *recogReadMem()
 *         l_int32             recogWrite()
 *         l_int32             recogWriteStream()
 *         l_int32             recogWriteMem()
 *         PIXA               *recogExtractPixa()
 *         static l_int32      recogAddCharstrLabels()
 *         static l_int32      recogAddAllSamples()
 *
 *  The recognizer functionality is split into four files:
 *    recogbasic.c: create, destroy, access, serialize
 *    recogtrain.c: training on labeled and unlabeled data
 *    recogident.c: running the recognizer(s) on input
 *    recogdid.c:   running the recognizer(s) on input using a
 *                  document image decoding (DID) hidden markov model
 *
 *  This is a content-adapted (or book-adapted) recognizer (BAR) application.
 *  The recognizers here are typically assembled from data that has
 *  been labeled by a generic recognition system, such as Tesseract.
 *  The general procedure to create a recognizer (recog) from labeled data is
 *  to add the labeled character bitmaps, either one at a time or
 *  all together from a pixa with labeled pix.
 *
 *  The suggested use for a BAR that consists of labeled templates drawn
 *  from a single source (e.g., a book) is to identify unlabeled samples
 *  by using unscaled character templates in the BAR, picking the
 *  template closest to the unlabeled sample.
 *
 *  Outliers can be removed from a pixa of labeled pix.  This is one of
 *  two methods that use averaged templates (the other is greedy splitting
 *  of characters).  See recogtrain.c for a discussion and the implementation.
 *
 *  A special bootstrap recognizer (BSR) can be used to make a BAR from
 *  unlabeled book data.  This is done by comparing character images
 *  from the book with labeled templates in the BSR, where all images
 *  are scaled to h = 40.  The templates can be either the scanned images
 *  or images consisting of width-normalized strokes derived from
 *  the skeleton of the character bitmaps.
 *
 *  Two BARs of labeled character data, that have been made by
 *  different recognizers, can be joined by extracting a pixa of the
 *  labeled templates from each, joining the two pixa, and then
 *  and regenerating a BAR from the joined set of templates.
 *  If all the labeled character data is from a single source (e.g, a book),
 *  identification can proceed using unscaled templates (either the input
 *  image or width-normalized lines).  But if the labeled data comes from
 *  more than one source, (a "hybrid" recognizer), the templates should
 *  be scaled, and we recommend scaling to a fixed height.
 *
 *  Suppose it is not possible to generate a BAR with a sufficient number
 *  of templates of each class taken from a single source.  In that case,
 *  templates from the BSR itself can be added.  This is the condition
 *  described above, where the labeled templates come from multiple
 *  sources, and it is necessary to do all character matches using
 *  templates that have been scaled to a fixed height (e.g., 40).
 *  Likewise, the samples to be identified using this hybrid recognizer
 *  must be modified in the same way.  See prog/recogtest3.c for an
 *  example of the steps that can be taken in the construction of a BAR
 *  using a BSR.
 *
 *  For training numeric input, an example set of calls that scales
 *  each training input to fixed h and will use the line templates of
 *  width linew for identifying unknown characters is:
 *         L_Recog  *rec = recogCreate(0, h, linew, 128, 1);
 *         for (i = 0; i < n; i++) {  // read in n training digits
 *             Pix *pix = ...
 *             recogTrainLabeled(rec, pix, NULL, text[i], 0);
 *         }
 *         recogTrainingFinished(&rec, 1, -1, -1.0);  // required
 *
 *  It is an error if any function that computes averages, removes
 *  outliers or requests identification of an unlabeled character,
 *  such as:
 *     (1) computing the sample averages: recogAverageSamples()
 *     (2) removing outliers: recogRemoveOutliers1() or recogRemoveOutliers2()
 *     (3) requesting identification of an unlabeled character:
 *         recogIdentifyPix()
 *  is called before an explicit call to finish training.  Note that
 *  to do further training on a "finished" recognizer, you can set
 *         recog->train_done = FALSE;
 *  add the new training samples, and again call
 *         recogTrainingFinished(&rec, 1, -1, -1.0);  // required
 *
 *  If not scaling, using the images directly for identification, and
 *  removing outliers, do something like this:
 *      L_Recog  *rec = recogCreate(0, 0, 0, 128, 1);
 *      for (i = 0; i < n; i++) {  // read in n training characters
 *          Pix *pix = ...
 *          recogTrainLabeled(rec, pix, NULL, text[i], 0);
 *      }
 *      recogTrainingFinished(&rec, 1, -1, -1.0);
 *      if (!rec) ... [return]
 *      // remove outliers
 *      recogRemoveOutliers1(&rec, 0.7, 2, NULL, NULL);
 *
 *  You can generate a recognizer from a pixa where the text field in
 *  each pix is the character string label for the pix.  For example,
 *  the following recognizer will store unscaled line images:
 *      L_Recog  *rec = recogCreateFromPixa(pixa, 0, 0, linew, 128, 1);
 *  and in use, it is fed unscaled line images to identify.
 *
 *  For the following, assume that you have a pixa of labeled templates.
 *  If it is likely that some of the input templates are mislabeled,
 *  there are several things that can be done to remove them.
 *  The first is to put a size and quantity filter on them; e.g.
 *       Pixa *pixa2 = recogFilterPixaBySize(pixa1, 10, 15, 2.6);
 *  Then you can remove outliers; e.g.,
 *       Pixa *pixa3 = pixaRemoveOutliers2(pixa2, -1.0, -1, NULL, NULL);
 *
 *  To this point, all templates are from a single source, so you
 *  can make a recognizer that uses the unscaled templates and optionally
 *  attempts to split touching characters:
 *       L_Recog *recog1 = recogCreateFromPixa(pixa3, ...);
 *  Alternatively, if you need more templates for some of the classes,
 *  you can pad with templates from a "bootstrap" recognizer (BSR).
 *  If you pad, it is necessary to scale the templates and input
 *  samples to a fixed height, and no attempt will be made to split
 *  the input sample connected components:
 *       L_Recog *recog1 = recogCreateFromPixa(pixa3, 0, 40, 0, 128, 0);
 *       recogPadDigitTrainingSet(&recog1, 40, 0);
 *
 *  A special case is a pure BSR, that contains images scaled to a fixed
 *  height (we use 40 in these examples).
 *  For this,use either the scanned bitmap:
 *      L_Recog  *recboot = recogCreateFromPixa(pixa, 0, 40, 0, 128, 1);
 *  or width-normalized lines (use width of 5 here):
 *      L_Recog  *recboot = recogCreateFromPixa(pixa, 0, 40, 5, 128, 1);
 *
 *  This can be used to train a new book adapted recognizer (BAC), on
 *  unlabeled data from, e.g., a book.  To do this, the following is required:
 *   (1) the input images from the book must be scaled in the same
 *       way as those in the BSR, and
 *   (2) both the BSR and the input images must be set up to be either
 *       input scanned images or width-normalized lines.
 *
 * </pre>
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include <string.h>
#include "allheaders.h"

static const l_int32    MaxExamplesInClass = 256;

    /* Default recog parameters that can be changed */
static const l_int32    DefaultCharsetType = L_ARABIC_NUMERALS;
static const l_int32    DefaultMinNopad = 1;
static const l_float32  DefaultMaxWHRatio = 3.0f;  /* max allowed w/h
                                    ratio for a component to be split  */
static const l_float32  DefaultMaxHTRatio = 2.6f;  /* max allowed ratio of
                               max/min unscaled averaged template heights  */
static const l_int32    DefaultThreshold = 150;  /* for binarization */
static const l_int32    DefaultMaxYShift = 1;  /* for identification */

    /* Static functions */
static l_int32 recogGetCharsetSize(l_int32 type);
static l_int32 recogAddCharstrLabels(L_RECOG *recog);
static l_int32 recogAddAllSamples(L_RECOG **precog, PIXAA *paa, l_int32 debug);


/*------------------------------------------------------------------------*
 *                Recog: initialization and destruction                   *
 *------------------------------------------------------------------------*/
/*!
 * \brief   recogCreateFromRecog()
 *
 * \param[in]    recs        source recog with arbitrary input parameters
 * \param[in]    scalew      scale all widths to this; use 0 otherwise
 * \param[in]    scaleh      scale all heights to this; use 0 otherwise
 * \param[in]    linew       width of normalized strokes; use 0 to skip
 * \param[in]    threshold   for binarization; typically ~128
 * \param[in]    maxyshift   from nominal centroid alignment; default is 1
 * \return  recd, or NULL on error
 *
 * <pre>
 * Notes:
 *      (1) This is a convenience function that generates a recog using
 *          the unscaled training data in an existing recog.
 *      (2) It is recommended to use %maxyshift = 1 (the default value)
 *      (3) See recogCreate() for use of %scalew, %scaleh and %linew.
 * </pre>
 */
L_RECOG *
recogCreateFromRecog(L_RECOG  *recs,
                     l_int32   scalew,
                     l_int32   scaleh,
                     l_int32   linew,
                     l_int32   threshold,
                     l_int32   maxyshift)
{
L_RECOG  *recd;
PIXA     *pixa;

    if (!recs)
        return (L_RECOG *)ERROR_PTR("recs not defined", __func__, NULL);

    pixa = recogExtractPixa(recs);
    recd = recogCreateFromPixa(pixa, scalew, scaleh, linew, threshold,
                               maxyshift);
    pixaDestroy(&pixa);
    return recd;
}


/*!
 * \brief   recogCreateFromPixa()
 *
 * \param[in]    pixa         of labeled, 1 bpp images
 * \param[in]    scalew       scale all widths to this; use 0 otherwise
 * \param[in]    scaleh       scale all heights to this; use 0 otherwise
 * \param[in]    linew        width of normalized strokes; use 0 to skip
 * \param[in]    threshold    for binarization; typically ~150
 * \param[in]    maxyshift    from nominal centroid alignment; default is 1
 * \return  recog, or NULL on error
 *
 * <pre>
 * Notes:
 *      (1) This is a convenience function for training from labeled data.
 *          The pixa can be read from file.
 *      (2) The pixa should contain the unscaled bitmaps used for training.
 *      (3) See recogCreate() for use of %scalew, %scaleh and %linew.
 *      (4) It is recommended to use %maxyshift = 1 (the default value)
 *      (5) All examples in the same class (i.e., with the same character
 *          label) should be similar.  They can be made similar by invoking
 *          recogRemoveOutliers[1,2]() on %pixa before calling this function.
 * </pre>
 */
L_RECOG *
recogCreateFromPixa(PIXA    *pixa,
                    l_int32  scalew,
                    l_int32  scaleh,
                    l_int32  linew,
                    l_int32  threshold,
                    l_int32  maxyshift)
{
L_RECOG  *recog;

    if (!pixa)
        return (L_RECOG *)ERROR_PTR("pixa not defined", __func__, NULL);

    recog = recogCreateFromPixaNoFinish(pixa, scalew, scaleh, linew,
                                        threshold, maxyshift);
    if (!recog)
        return (L_RECOG *)ERROR_PTR("recog not made", __func__, NULL);

    recogTrainingFinished(&recog, 1, -1, -1.0);
    if (!recog)
        return (L_RECOG *)ERROR_PTR("bad templates", __func__, NULL);
    return recog;
}


/*!
 * \brief   recogCreateFromPixaNoFinish()
 *
 * \param[in]    pixa         of labeled, 1 bpp images
 * \param[in]    scalew       scale all widths to this; use 0 otherwise
 * \param[in]    scaleh       scale all heights to this; use 0 otherwise
 * \param[in]    linew        width of normalized strokes; use 0 to skip
 * \param[in]    threshold    for binarization; typically ~150
 * \param[in]    maxyshift    from nominal centroid alignment; default is 1
 * \return  recog, or NULL on error
 *
 * <pre>
 * Notes:
 *      (1) See recogCreateFromPixa() for details.
 *      (2) This is also used to generate a pixaa with templates
 *          in each class within a pixa.  For that, all args except for
 *          %pixa are ignored.
 * </pre>
 */
L_RECOG *
recogCreateFromPixaNoFinish(PIXA    *pixa,
                            l_int32  scalew,
                            l_int32  scaleh,
                            l_int32  linew,
                            l_int32  threshold,
                            l_int32  maxyshift)
{
char     *text;
l_int32   full, n, i, ntext, same, maxd;
PIX      *pix;
L_RECOG  *recog;

    if (!pixa)
        return (L_RECOG *)ERROR_PTR("pixa not defined", __func__, NULL);
    pixaVerifyDepth(pixa, &same, &maxd);
    if (maxd > 1)
        return (L_RECOG *)ERROR_PTR("not all pix are 1 bpp", __func__, NULL);

    pixaIsFull(pixa, &full, NULL);
    if (!full)
        return (L_RECOG *)ERROR_PTR("not all pix are present", __func__, NULL);

    n = pixaGetCount(pixa);
    pixaCountText(pixa, &ntext);
    if (ntext == 0)
        return (L_RECOG *)ERROR_PTR("no pix have text strings", __func__, NULL);
    if (ntext < n)
        L_ERROR("%d text strings < %d pix\n", __func__, ntext, n);

    recog = recogCreate(scalew, scaleh, linew, threshold, maxyshift);
    if (!recog)
        return (L_RECOG *)ERROR_PTR("recog not made", __func__, NULL);
    for (i = 0; i < n; i++) {
        pix = pixaGetPix(pixa, i, L_CLONE);
        text = pixGetText(pix);
        if (!text || strlen(text) == 0) {
            L_ERROR("pix[%d] has no text\n", __func__, i);
            pixDestroy(&pix);
            continue;
        }
        recogTrainLabeled(recog, pix, NULL, text, 0);
        pixDestroy(&pix);
    }

    return recog;
}


/*!
 * \brief   recogCreate()
 *
 * \param[in]    scalew       scale all widths to this; use 0 otherwise
 * \param[in]    scaleh       scale all heights to this; use 0 otherwise
 * \param[in]    linew        width of normalized strokes; use 0 to skip
 * \param[in]    threshold    for binarization; typically ~128; 0 for default
 * \param[in]    maxyshift    from nominal centroid alignment; default is 1
 * \return  recog, or NULL on error
 *
 * <pre>
 * Notes:
 *      (1) If %scalew == 0 and %scaleh == 0, no scaling is done.
 *          If one of these is 0 and the other is > 0, scaling is isotropic
 *          to the requested size.  We typically do not set both > 0.
 *      (2) Use linew > 0 to convert the templates to images with fixed
 *          width strokes.  linew == 0 skips the conversion.
 *      (3) The only valid values for %maxyshift are 0, 1 and 2.
 *          It is recommended to use %maxyshift == 1 (default value).
 *          Using %maxyshift == 0 is much faster than %maxyshift == 1, but
 *          it is much less likely to find the template with the best
 *          correlation.  Use of anything but 1 results in a warning.
 *      (4) Scaling is used for finding outliers and for training a
 *          book-adapted recognizer (BAR) from a bootstrap recognizer (BSR).
 *          Scaling the height to a fixed value and scaling the width
 *          accordingly (e.g., %scaleh = 40, %scalew = 0) is recommended.
 *      (5) The storage for most of the arrays is allocated when training
 *          is finished.
 * </pre>
 */
L_RECOG *
recogCreate(l_int32  scalew,
            l_int32  scaleh,
            l_int32  linew,
            l_int32  threshold,
            l_int32  maxyshift)
{
L_RECOG  *recog;

    if (scalew < 0 || scaleh < 0)
        return (L_RECOG *)ERROR_PTR("invalid scalew or scaleh", __func__, NULL);
    if (linew > 10)
        return (L_RECOG *)ERROR_PTR("invalid linew > 10", __func__, NULL);
    if (threshold == 0) threshold = DefaultThreshold;
    if (threshold < 0 || threshold > 255) {
        L_WARNING("invalid threshold; using default\n", __func__);
        threshold = DefaultThreshold;
    }
    if (maxyshift < 0 || maxyshift > 2) {
         L_WARNING("invalid maxyshift; using default value\n", __func__);
         maxyshift = DefaultMaxYShift;
    } else if (maxyshift == 0) {
         L_WARNING("Using maxyshift = 0; faster, worse correlation results\n",
                   __func__);
    } else if (maxyshift == 2) {
         L_WARNING("Using maxyshift = 2; slower\n", __func__);
    }

    recog = (L_RECOG *)LEPT_CALLOC(1, sizeof(L_RECOG));
    recog->templ_use = L_USE_ALL_TEMPLATES;  /* default */
    recog->threshold = threshold;
    recog->scalew = scalew;
    recog->scaleh = scaleh;
    recog->linew = linew;
    recog->maxyshift = maxyshift;
    recogSetParams(recog, 1, -1, -1.0, -1.0);
    recog->bmf = bmfCreate(NULL, 6);
    recog->bmf_size = 6;
    recog->maxarraysize = MaxExamplesInClass;

        /* Generate the LUTs */
    recog->centtab = makePixelCentroidTab8();
    recog->sumtab = makePixelSumTab8();
    recog->sa_text = sarrayCreate(0);
    recog->dna_tochar = l_dnaCreate(0);

        /* Input default values for min component size for splitting.
         * These are overwritten when pixTrainingFinished() is called. */
    recog->min_splitw = 6;
    recog->max_splith = 60;

        /* Allocate the paa for the unscaled training bitmaps */
    recog->pixaa_u = pixaaCreate(recog->maxarraysize);

        /* Generate the storage for debugging */
    recog->pixadb_boot = pixaCreate(2);
    recog->pixadb_split = pixaCreate(2);
    return recog;
}


/*!
 * \brief   recogDestroy()
 *
 * \param[in,out]   precog    will be set to null before returning
 * \return  void
 */
void
recogDestroy(L_RECOG  **precog)
{
L_RECOG  *recog;

    if (!precog) {
        L_WARNING("ptr address is null\n", __func__);
        return;
    }

    if ((recog = *precog) == NULL) return;

    LEPT_FREE(recog->centtab);
    LEPT_FREE(recog->sumtab);
    sarrayDestroy(&recog->sa_text);
    l_dnaDestroy(&recog->dna_tochar);
    pixaaDestroy(&recog->pixaa_u);
    pixaDestroy(&recog->pixa_u);
    ptaaDestroy(&recog->ptaa_u);
    ptaDestroy(&recog->pta_u);
    numaDestroy(&recog->nasum_u);
    numaaDestroy(&recog->naasum_u);
    pixaaDestroy(&recog->pixaa);
    pixaDestroy(&recog->pixa);
    ptaaDestroy(&recog->ptaa);
    ptaDestroy(&recog->pta);
    numaDestroy(&recog->nasum);
    numaaDestroy(&recog->naasum);
    pixaDestroy(&recog->pixa_tr);
    pixaDestroy(&recog->pixadb_ave);
    pixaDestroy(&recog->pixa_id);
    pixDestroy(&recog->pixdb_ave);
    pixDestroy(&recog->pixdb_range);
    pixaDestroy(&recog->pixadb_boot);
    pixaDestroy(&recog->pixadb_split);
    bmfDestroy(&recog->bmf);
    rchDestroy(&recog->rch);
    rchaDestroy(&recog->rcha);
    recogDestroyDid(recog);
    LEPT_FREE(recog);
    *precog = NULL;
}


/*------------------------------------------------------------------------*
 *                              Recog accessors                           *
 *------------------------------------------------------------------------*/
/*!
 * \brief   recogGetCount()
 *
 * \param[in]    recog
 * \return  count of classes in recog; 0 if no recog or on error
 */
l_int32
recogGetCount(L_RECOG  *recog)
{
    if (!recog)
        return ERROR_INT("recog not defined", __func__, 0);
    return recog->setsize;
}


/*!
 * \brief   recogSetParams()
 *
 * \param[in]    recog          to be padded, if necessary
 * \param[in]    type           type of char set; -1 for default;
 *                              see enum in recog.h
 * \param[in]    min_nopad      min number in a class without padding;
 *                              use -1 for default
 * \param[in]    max_wh_ratio   max width/height ratio allowed for splitting;
 *                              use -1.0 for default
 * \param[in]    max_ht_ratio   max of max/min averaged template height ratio;
 *                              use -1.0 for default
 * \return       0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *      (1) This is called when a recog is created.
 *      (2) Default %min_nopad value allows for some padding.
 *          To disable padding, set %min_nopad = 0.  To pad only when
 *          no samples are available for the class, set %min_nopad = 1.
 *      (3) The %max_wh_ratio limits the width/height ratio for components
 *          that we attempt to split.  Splitting long components is expensive.
 *      (4) The %max_ht_ratio is a quality requirement on the training data.
 *          The recognizer will not run if the averages are computed and
 *          the templates do not satisfy it.
 * </pre>
 */
l_ok
recogSetParams(L_RECOG   *recog,
               l_int32    type,
               l_int32    min_nopad,
               l_float32  max_wh_ratio,
               l_float32  max_ht_ratio)
{
    if (!recog)
        return ERROR_INT("recog not defined", __func__, 1);

    recog->charset_type = (type >= 0) ? type : DefaultCharsetType;
    recog->charset_size = recogGetCharsetSize(recog->charset_type);
    recog->min_nopad = (min_nopad >= 0) ? min_nopad : DefaultMinNopad;
    recog->max_wh_ratio = (max_wh_ratio > 0.0) ? max_wh_ratio :
                          DefaultMaxWHRatio;
    recog->max_ht_ratio = (max_ht_ratio > 1.0) ? max_ht_ratio :
                          DefaultMaxHTRatio;
    return 0;
}


/*!
 * \brief   recogGetCharsetSize()
 *
 * \param[in]    type     of charset
 * \return  size of charset, or 0 if unknown or on error
 */
static l_int32
recogGetCharsetSize(l_int32  type)
{
    switch (type) {
    case L_UNKNOWN:
        return 0;
    case L_ARABIC_NUMERALS:
        return 10;
    case L_LC_ROMAN_NUMERALS:
        return 7;
    case L_UC_ROMAN_NUMERALS:
        return 7;
    case L_LC_ALPHA:
        return 26;
    case L_UC_ALPHA:
        return 26;
    default:
        L_ERROR("invalid charset_type %d\n", __func__, type);
    }
    return 0;
}


/*------------------------------------------------------------------------*
 *                         Character/index lookup                         *
 *------------------------------------------------------------------------*/
/*!
 * \brief   recogGetClassIndex()
 *
 * \param[in]    recog     with LUT's pre-computed
 * \param[in]    val       integer value; can be up to 4 bytes for UTF-8
 * \param[in]    text      text from which %val was derived; used if not found
 * \param[out]   pindex    index into dna_tochar
 * \return  0 if found; 1 if not found and added; 2 on error.
 *
 * <pre>
 * Notes:
 *      (1) This is used during training.  There is one entry in
 *          recog->dna_tochar (integer value, e.g., ascii) and
 *          one in recog->sa_text (e.g, ascii letter in a string)
 *          for each character class.
 *      (2) This searches the dna character array for %val.  If it is
 *          not found, the template represents a character class not
 *          already seen: it increments setsize (the number of character
 *          classes) by 1, and augments both the index (dna_tochar)
 *          and text (sa_text) arrays.
 *      (3) Returns the index in &index, except on error.
 *      (4) Caller must check the function return value.
 * </pre>
 */
l_int32
recogGetClassIndex(L_RECOG  *recog,
                   l_int32   val,
                   char     *text,
                   l_int32  *pindex)
{
l_int32  i, n, ival;

    if (!pindex)
        return ERROR_INT("&index not defined", __func__, 2);
    *pindex = -1;
    if (!recog)
        return ERROR_INT("recog not defined", __func__, 2);
    if (!text)
        return ERROR_INT("text not defined", __func__, 2);

        /* Search existing characters */
    n = l_dnaGetCount(recog->dna_tochar);
    for (i = 0; i < n; i++) {
        l_dnaGetIValue(recog->dna_tochar, i, &ival);
        if (val == ival) {  /* found */
            *pindex = i;
            return 0;
        }
    }

       /* If not found... */
    l_dnaAddNumber(recog->dna_tochar, val);
    sarrayAddString(recog->sa_text, text, L_COPY);
    recog->setsize++;
    *pindex = n;
    return 1;
}


/*!
 * \brief   recogStringToIndex()
 *
 * \param[in]    recog
 * \param[in]    text     text string for some class
 * \param[out]   pindex   index for that class; -1 if not found
 * \return  0 if OK, 1 on error not finding the string is an error
 */
l_ok
recogStringToIndex(L_RECOG  *recog,
                   char     *text,
                   l_int32  *pindex)
{
char    *charstr;
l_int32  i, n, diff;

    if (!pindex)
        return ERROR_INT("&index not defined", __func__, 1);
    *pindex = -1;
    if (!recog)
        return ERROR_INT("recog not defined", __func__, 1);
    if (!text)
        return ERROR_INT("text not defined", __func__, 1);

        /* Search existing characters */
    n = recog->setsize;
    for (i = 0; i < n; i++) {
        recogGetClassString(recog, i, &charstr);
        if (!charstr) {
            L_ERROR("string not found for index %d\n", __func__, i);
            continue;
        }
        diff = strcmp(text, charstr);
        LEPT_FREE(charstr);
        if (diff) continue;
        *pindex = i;
        return 0;
    }

    return 1;  /* not found */
}


/*!
 * \brief   recogGetClassString()
 *
 * \param[in]    recog
 * \param[in]    index       into array of char types
 * \param[out]   pcharstr    string representation;
 *                           returns an empty string on error
 * \return  0 if found, 1 on error
 *
 * <pre>
 * Notes:
 *      (1) Extracts a copy of the string from sa_text, which
 *          the caller must free.
 *      (2) Caller must check the function return value.
 * </pre>
 */
l_int32
recogGetClassString(L_RECOG  *recog,
                    l_int32   index,
                    char    **pcharstr)
{
    if (!pcharstr)
        return ERROR_INT("&charstr not defined", __func__, 1);
    *pcharstr = stringNew("");
    if (!recog)
        return ERROR_INT("recog not defined", __func__, 2);

    if (index < 0 || index >= recog->setsize)
        return ERROR_INT("invalid index", __func__, 1);
    LEPT_FREE(*pcharstr);
    *pcharstr = sarrayGetString(recog->sa_text, index, L_COPY);
    return 0;
}


/*!
 * \brief   l_convertCharstrToInt()
 *
 * \param[in]    str     input string representing one UTF-8 character;
 *                       not more than 4 bytes
 * \param[out]   pval    integer value for the input.  Think of it
 *                       as a 1-to-1 hash code.
 * \return  0 if OK, 1 on error
 */
l_ok
l_convertCharstrToInt(const char  *str,
                      l_int32     *pval)
{
l_int32   size;
l_uint32  val;

    if (!pval)
        return ERROR_INT("&val not defined", __func__, 1);
    *pval = 0;
    if (!str)
        return ERROR_INT("str not defined", __func__, 1);
    size = strlen(str);
    if (size == 0)
        return ERROR_INT("empty string", __func__, 1);
    if (size > 4)
        return ERROR_INT("invalid string: > 4 bytes", __func__, 1);

    val = (l_uint8)str[0];
    if (size > 1)
        val = (val << 8) + (l_uint8)str[1];
    if (size > 2)
        val = (val << 8) + (l_uint8)str[2];
    if (size > 3)
        val = (val << 8) + (l_uint8)str[3];
    *pval = (l_int32)(val & 0x7fffffff);
    return 0;
}


/*------------------------------------------------------------------------*
 *                             Serialization                              *
 *------------------------------------------------------------------------*/
/*!
 * \brief   recogRead()
 *
 * \param[in]    filename
 * \return  recog, or NULL on error
 *
 * <pre>
 * Notes:
 *      (1) When a recog is serialized, a pixaa of the templates that are
 *          actually used for correlation is saved in the pixaa_u array
 *          of the recog.  These can be different from the templates that
 *          were used to generate the recog, because those original templates
 *          can be scaled and turned into normalized lines.  When recog1
 *          is deserialized to recog2, these templates are put in both the
 *          unscaled array (pixaa_u) and the modified array (pixaa) in recog2.
 *          Why not put it in only the unscaled array and let
 *          recogTrainingFinalized() regenerate the modified templates?
 *          The reason is that with normalized lines, the operation of
 *          thinning to a skeleton and dilating back to a fixed width
 *          is not idempotent.  Thinning to a skeleton saves pixels at
 *          the end of a line segment, and thickening the skeleton puts
 *          additional pixels at the end of the lines.  This tends to
 *          close gaps.
 * </pre>
 */
L_RECOG *
recogRead(const char  *filename)
{
FILE     *fp;
L_RECOG  *recog;

    if (!filename)
        return (L_RECOG *)ERROR_PTR("filename not defined", __func__, NULL);
    if ((fp = fopenReadStream(filename)) == NULL)
        return (L_RECOG *)ERROR_PTR_1("stream not opened",
                                      filename, __func__, NULL);

    if ((recog = recogReadStream(fp)) == NULL) {
        fclose(fp);
        return (L_RECOG *)ERROR_PTR_1("recog not read",
                                      filename, __func__, NULL);
    }

    fclose(fp);
    return recog;
}


/*!
 * \brief   recogReadStream()
 *
 * \param[in]    fp     file stream
 * \return  recog, or NULL on error
 */
L_RECOG *
recogReadStream(FILE  *fp)
{
l_int32   version, setsize, threshold, scalew, scaleh, linew;
l_int32   maxyshift, nc;
L_DNA    *dna_tochar;
PIXAA    *paa;
L_RECOG  *recog;
SARRAY   *sa_text;

    if (!fp)
        return (L_RECOG *)ERROR_PTR("stream not defined", __func__, NULL);

    if (fscanf(fp, "\nRecog Version %d\n", &version) != 1)
        return (L_RECOG *)ERROR_PTR("not a recog file", __func__, NULL);
    if (version != RECOG_VERSION_NUMBER)
        return (L_RECOG *)ERROR_PTR("invalid recog version", __func__, NULL);
    if (fscanf(fp, "Size of character set = %d\n", &setsize) != 1)
        return (L_RECOG *)ERROR_PTR("setsize not read", __func__, NULL);
    if (fscanf(fp, "Binarization threshold = %d\n", &threshold) != 1)
        return (L_RECOG *)ERROR_PTR("binary thresh not read", __func__, NULL);
    if (fscanf(fp, "Maxyshift = %d\n", &maxyshift) != 1)
        return (L_RECOG *)ERROR_PTR("maxyshift not read", __func__, NULL);
    if (fscanf(fp, "Scale to width = %d\n", &scalew) != 1)
        return (L_RECOG *)ERROR_PTR("width not read", __func__, NULL);
    if (fscanf(fp, "Scale to height = %d\n", &scaleh) != 1)
        return (L_RECOG *)ERROR_PTR("height not read", __func__, NULL);
    if (fscanf(fp, "Normalized line width = %d\n", &linew) != 1)
        return (L_RECOG *)ERROR_PTR("line width not read", __func__, NULL);
    if ((recog = recogCreate(scalew, scaleh, linew, threshold,
                             maxyshift)) == NULL)
        return (L_RECOG *)ERROR_PTR("recog not made", __func__, NULL);

    if (fscanf(fp, "\nLabels for character set:\n") == -1) {
        recogDestroy(&recog);
        return (L_RECOG *)ERROR_PTR("label intro not read", __func__, NULL);
    }
    l_dnaDestroy(&recog->dna_tochar);
    if ((dna_tochar = l_dnaReadStream(fp)) == NULL) {
        recogDestroy(&recog);
        return (L_RECOG *)ERROR_PTR("dna_tochar not read", __func__, NULL);
    }
    recog->dna_tochar = dna_tochar;
    sarrayDestroy(&recog->sa_text);
    if ((sa_text = sarrayReadStream(fp)) == NULL) {
        recogDestroy(&recog);
        return (L_RECOG *)ERROR_PTR("sa_text not read", __func__, NULL);
    }
    recog->sa_text = sa_text;

    if (fscanf(fp, "\nPixaa of all samples in the training set:\n") == -1) {
        recogDestroy(&recog);
        return (L_RECOG *)ERROR_PTR("pixaa intro not read", __func__, NULL);
    }
    if ((paa = pixaaReadStream(fp)) == NULL) {
        recogDestroy(&recog);
        return (L_RECOG *)ERROR_PTR("pixaa not read", __func__, NULL);
    }
    recog->setsize = setsize;
    nc = pixaaGetCount(paa, NULL);
    if (nc != setsize) {
        recogDestroy(&recog);
        pixaaDestroy(&paa);
        L_ERROR("(setsize = %d) != (paa count = %d)\n", __func__,
                     setsize, nc);
        return NULL;
    }

    recogAddAllSamples(&recog, paa, 0);  /* this finishes */
    pixaaDestroy(&paa);
    if (!recog)
        return (L_RECOG *)ERROR_PTR("bad templates", __func__, NULL);
    return recog;
}


/*!
 * \brief   recogReadMem()
 *
 * \param[in]    data    serialization of recog (not ascii)
 * \param[in]    size    of data in bytes
 * \return  recog, or NULL on error
 */
L_RECOG *
recogReadMem(const l_uint8  *data,
             size_t          size)
{
FILE     *fp;
L_RECOG  *recog;

    if (!data)
        return (L_RECOG *)ERROR_PTR("data not defined", __func__, NULL);
    if ((fp = fopenReadFromMemory(data, size)) == NULL)
        return (L_RECOG *)ERROR_PTR("stream not opened", __func__, NULL);

    recog = recogReadStream(fp);
    fclose(fp);
    if (!recog) L_ERROR("recog not read\n", __func__);
    return recog;
}


/*!
 * \brief   recogWrite()
 *
 * \param[in]    filename
 * \param[in]    recog
 * \return  0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *      (1) The pixaa of templates that is written is the modified one
 *          in the pixaa field. It is the pixaa that is actually used
 *          for correlation. This is not the unscaled array of labeled
 *          bitmaps, in pixaa_u, that was used to generate the recog in the
 *          first place.  See the notes in recogRead() for the rationale.
 * </pre>
 */
l_ok
recogWrite(const char  *filename,
           L_RECOG     *recog)
{
l_int32  ret;
FILE    *fp;

    if (!filename)
        return ERROR_INT("filename not defined", __func__, 1);
    if (!recog)
        return ERROR_INT("recog not defined", __func__, 1);

    if ((fp = fopenWriteStream(filename, "wb")) == NULL)
        return ERROR_INT_1("stream not opened", filename, __func__, 1);
    ret = recogWriteStream(fp, recog);
    fclose(fp);
    if (ret)
        return ERROR_INT_1("recog not written to stream",
                           filename, __func__, 1);
    return 0;
}


/*!
 * \brief   recogWriteStream()
 *
 * \param[in]    fp      file stream opened for "wb"
 * \param[in]    recog
 * \return  0 if OK, 1 on error
 */
l_ok
recogWriteStream(FILE     *fp,
                 L_RECOG  *recog)
{
    if (!fp)
        return ERROR_INT("stream not defined", __func__, 1);
    if (!recog)
        return ERROR_INT("recog not defined", __func__, 1);

    fprintf(fp, "\nRecog Version %d\n", RECOG_VERSION_NUMBER);
    fprintf(fp, "Size of character set = %d\n", recog->setsize);
    fprintf(fp, "Binarization threshold = %d\n", recog->threshold);
    fprintf(fp, "Maxyshift = %d\n", recog->maxyshift);
    fprintf(fp, "Scale to width = %d\n", recog->scalew);
    fprintf(fp, "Scale to height = %d\n", recog->scaleh);
    fprintf(fp, "Normalized line width = %d\n", recog->linew);
    fprintf(fp, "\nLabels for character set:\n");
    l_dnaWriteStream(fp, recog->dna_tochar);
    sarrayWriteStream(fp, recog->sa_text);
    fprintf(fp, "\nPixaa of all samples in the training set:\n");
    pixaaWriteStream(fp, recog->pixaa);

    return 0;
}


/*!
 * \brief   recogWriteMem()
 *
 * \param[out]   pdata    data of serialized recog (not ascii)
 * \param[out]   psize    size of returned data
 * \param[in]    recog
 * \return  0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *      (1) Serializes a recog in memory and puts the result in a buffer.
 * </pre>
 */
l_ok
recogWriteMem(l_uint8  **pdata,
              size_t    *psize,
              L_RECOG   *recog)
{
l_int32  ret;
FILE    *fp;

    if (pdata) *pdata = NULL;
    if (psize) *psize = 0;
    if (!pdata)
        return ERROR_INT("&data not defined", __func__, 1);
    if (!psize)
        return ERROR_INT("&size not defined", __func__, 1);
    if (!recog)
        return ERROR_INT("recog not defined", __func__, 1);

#if HAVE_FMEMOPEN
    if ((fp = open_memstream((char **)pdata, psize)) == NULL)
        return ERROR_INT("stream not opened", __func__, 1);
    ret = recogWriteStream(fp, recog);
    fputc('\0', fp);
    fclose(fp);
    if (*psize > 0) *psize = *psize - 1;
#else
    L_INFO("no fmemopen API --> work-around: write to temp file\n", __func__);
  #ifdef _WIN32
    if ((fp = fopenWriteWinTempfile()) == NULL)
        return ERROR_INT("tmpfile stream not opened", __func__, 1);
  #else
    if ((fp = tmpfile()) == NULL)
        return ERROR_INT("tmpfile stream not opened", __func__, 1);
  #endif  /* _WIN32 */
    ret = recogWriteStream(fp, recog);
    rewind(fp);
    *pdata = l_binaryReadStream(fp, psize);
    fclose(fp);
#endif  /* HAVE_FMEMOPEN */
    return ret;
}


/*!
 * \brief   recogExtractPixa()
 *
 * \param[in]   recog
 * \return  pixa if OK, NULL on error
 *
 * <pre>
 * Notes:
 *      (1) This generates a pixa of all the unscaled images in the
 *          recognizer, where each one has its character class label in
 *          the pix text field, by flattening pixaa_u to a pixa.
 * </pre>
 */
PIXA *
recogExtractPixa(L_RECOG  *recog)
{
    if (!recog)
        return (PIXA *)ERROR_PTR("recog not defined", __func__, NULL);

    recogAddCharstrLabels(recog);
    return pixaaFlattenToPixa(recog->pixaa_u, NULL, L_CLONE);
}


/*!
 * \brief   recogAddCharstrLabels()
 *
 * \param[in]    recog
 * \return  0 if OK, 1 on error
 */
static l_int32
recogAddCharstrLabels(L_RECOG  *recog)
{
char    *text;
l_int32  i, j, n1, n2;
PIX     *pix;
PIXA    *pixa;
PIXAA   *paa;

    if (!recog)
        return ERROR_INT("recog not defined", __func__, 1);

        /* Add the labels to each unscaled pix */
    paa = recog->pixaa_u;
    n1 = pixaaGetCount(paa, NULL);
    for (i = 0; i < n1; i++) {
        pixa = pixaaGetPixa(paa, i, L_CLONE);
        text = sarrayGetString(recog->sa_text, i, L_NOCOPY);
        n2 = pixaGetCount(pixa);
        for (j = 0; j < n2; j++) {
             pix = pixaGetPix(pixa, j, L_CLONE);
             pixSetText(pix, text);
             pixDestroy(&pix);
        }
        pixaDestroy(&pixa);
    }

    return 0;
}


/*!
 * \brief   recogAddAllSamples()
 *
 * \param[in]    precog    addr of recog
 * \param[in]    paa       pixaa from previously trained recog
 * \param[in]    debug
 * \return  0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *      (1) On error, the input recog is destroyed.
 *      (2) This is used with the serialization routine recogRead(),
 *          where each pixa in the pixaa represents a set of characters
 *          in a different class.  Before calling this function, we have
 *          verified that the number of character classes, given by the
 *          setsize field in %recog, equals the number of pixa in the paa.
 *          The character labels for each set are in the sa_text field.
 * </pre>
 */
static l_int32
recogAddAllSamples(L_RECOG  **precog,
                   PIXAA     *paa,
                   l_int32    debug)
{
char     *text;
l_int32   i, j, nc, ns;
PIX      *pix;
PIXA     *pixa, *pixa1;
L_RECOG  *recog;

    if (!precog)
        return ERROR_INT("&recog not defined", __func__, 1);
    if ((recog = *precog) == NULL)
        return ERROR_INT("recog not defined", __func__, 1);
    if (!paa) {
        recogDestroy(&recog);
        *precog = NULL;
        return ERROR_INT("paa not defined", __func__, 1);
    }

    nc = pixaaGetCount(paa, NULL);
    for (i = 0; i < nc; i++) {
        pixa = pixaaGetPixa(paa, i, L_CLONE);
        ns = pixaGetCount(pixa);
        text = sarrayGetString(recog->sa_text, i, L_NOCOPY);
        pixa1 = pixaCreate(ns);
        pixaaAddPixa(recog->pixaa_u, pixa1, L_INSERT);
        for (j = 0; j < ns; j++) {
            pix = pixaGetPix(pixa, j, L_CLONE);
            if (debug) lept_stderr("pix[%d,%d]: text = %s\n", i, j, text);
            pixaaAddPix(recog->pixaa_u, i, pix, NULL, L_INSERT);
        }
        pixaDestroy(&pixa);
    }

    recogTrainingFinished(&recog, 0, -1, -1.0);  /* For second parameter,
                                             see comment in recogRead() */
    if (!recog)
        return ERROR_INT("bad templates; recog destroyed", __func__, 1);
    return 0;
}
