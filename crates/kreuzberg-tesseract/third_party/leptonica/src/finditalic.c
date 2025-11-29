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
 * \file  finditalic.c
 * <pre>
 *
 *      l_int32   pixItalicWords()
 *
 *    Locate italic words.  This is an example of the use of
 *    hit-miss binary morphology with binary reconstruction
 *    (filling from a seed into a mask).
 *
 *    To see how this works, run with prog/italic.png.
 * </pre>
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

    /* ---------------------------------------------------------------  *
     * These hit-miss sels match the slanted edge of italic characters  *
     * ---------------------------------------------------------------  */
static const char *str_ital1 = "   o x"
                               "      "
                               "      "
                               "      "
                               "  o x "
                               "      "
                               "  C   "
                               "      "
                               " o x  "
                               "      "
                               "      "
                               "      "
                               "o x   ";

static const char *str_ital2 = "   o x"
                               "      "
                               "      "
                               "  o x "
                               "  C   "
                               "      "
                               " o x  "
                               "      "
                               "      "
                               "o x   ";

    /* ------------------------------------------------------------- *
     * This sel removes noise that is not oriented as a slanted edge *
     * ------------------------------------------------------------- */
static const char *str_ital3 = " x"
                               "Cx"
                               "x "
                               "x ";

/*!
 * \brief   pixItalicWords()
 *
 * \param[in]    pixs       1 bpp
 * \param[in]    boxaw      [optional] word bounding boxes; can be NULL
 * \param[in]    pixw       [optional] word box mask; can be NULL
 * \param[out]   pboxa      boxa of italic words
 * \param[in]    debugflag  1 for debug output; 0 otherwise
 * \return  0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *      (1) You can input the bounding boxes for the words in one of
 *          two forms: as bounding boxes (%boxaw) or as a word mask with
 *          the word bounding boxes filled (%pixw).  For example,
 *          to compute %pixw, you can use pixWordMaskByDilation().
 *      (2) Alternatively, you can set both of these inputs to NULL,
 *          in which case the word mask is generated here.  This is
 *          done by dilating and closing the input image to connect
 *          letters within a word, while leaving the words separated.
 *          The parameters are chosen under the assumption that the
 *          input is 10 to 12 pt text, scanned at about 300 ppi.
 *      (3) sel_ital1 and sel_ital2 detect the right edges that are
 *          nearly vertical, at approximately the angle of italic
 *          strokes.  We use the right edge to avoid getting seeds
 *          from lower-case 'y'.  The typical italic slant has a smaller
 *          angle with the vertical than the 'W', so in most cases we
 *          will not trigger on the slanted lines in the 'W'.
 *      (4) Note that sel_ital2 is shorter than sel_ital1.  It is
 *          more appropriate for a typical font scanned at 200 ppi.
 * </pre>
 */
l_ok
pixItalicWords(PIX     *pixs,
               BOXA    *boxaw,
               PIX     *pixw,
               BOXA   **pboxa,
               l_int32  debugflag)
{
char     opstring[32], buf[32];
l_int32  size, type;
BOXA    *boxa;
PIX     *pixsd, *pixm, *pixd;
SEL     *sel_ital1, *sel_ital2, *sel_ital3;

    if (!pboxa)
        return ERROR_INT("&boxa not defined", __func__, 1);
    *pboxa = NULL;
    if (!pixs)
        return ERROR_INT("pixs not defined", __func__, 1);
    if (boxaw && pixw)
        return ERROR_INT("both boxaw and pixw are defined", __func__, 1);

    sel_ital1 = selCreateFromString(str_ital1, 13, 6, NULL);
    sel_ital2 = selCreateFromString(str_ital2, 10, 6, NULL);
    sel_ital3 = selCreateFromString(str_ital3, 4, 2, NULL);

        /* Make the italic seed: extract with HMT; remove noise.
         * The noise removal close/open is important to exclude
         * situations where a small slanted line accidentally
         * matches sel_ital1. */
    pixsd = pixHMT(NULL, pixs, sel_ital1);
    pixClose(pixsd, pixsd, sel_ital3);
    pixOpen(pixsd, pixsd, sel_ital3);

        /* Make the word mask.  Use input boxes or mask if given. */
    size = 0;  /* init */
    if (boxaw) {
        pixm = pixCreateTemplate(pixs);
        pixMaskBoxa(pixm, pixm, boxaw, L_SET_PIXELS);
        type = 1;
    } else if (pixw) {
        pixm = pixClone(pixw);
        type = 2;
    } else {
        pixWordMaskByDilation(pixs, NULL, &size, NULL);
        L_INFO("dilation size = %d\n", __func__, size);
        snprintf(opstring, sizeof(opstring), "d1.5 + c%d.1", size);
        pixm = pixMorphSequence(pixs, opstring, 0);
        type = 3;
    }

        /* Binary reconstruction to fill in those word mask
         * components for which there is at least one seed pixel. */
    pixd = pixSeedfillBinary(NULL, pixsd, pixm, 8);
    boxa = pixConnComp(pixd, NULL, 8);
    *pboxa = boxa;

    if (debugflag) {
            /* Save results at at 2x reduction */
        l_int32  res, upper;
        lept_mkdir("lept/ital");
        BOXA  *boxat;
        GPLOT *gplot;
        NUMA  *na;
        PIXA  *pixa1;
        PIX   *pix1, *pix2, *pix3;
        pixa1 = pixaCreate(0);
        boxat = pixConnComp(pixm, NULL, 8);
        boxaWriteDebug("/tmp/lept/ital/ital.ba", boxat);
        pixaAddPix(pixa1, pixs, L_COPY);  /* orig */
        pixaAddPix(pixa1, pixsd, L_COPY);  /* seed */
        pix1 = pixConvertTo32(pixm);
        pixRenderBoxaArb(pix1, boxat, 3, 255, 0, 0);
        pixaAddPix(pixa1, pix1, L_INSERT);  /* mask + outline */
        pixaAddPix(pixa1, pixd, L_COPY);  /* ital mask */
        pix1 = pixConvertTo32(pixs);
        pixRenderBoxaArb(pix1, boxa, 3, 255, 0, 0);
        pixaAddPix(pixa1, pix1, L_INSERT);  /* orig + outline */
        pix1 = pixCreateTemplate(pixs);
        pix2 = pixSetBlackOrWhiteBoxa(pix1, boxa, L_SET_BLACK);
        pixCopy(pix1, pixs);
        pix3 = pixDilateBrick(NULL, pixs, 3, 3);
        pixCombineMasked(pix1, pix3, pix2);
        pixaAddPix(pixa1, pix1, L_INSERT);  /* ital bolded */
        pixDestroy(&pix2);
        pixDestroy(&pix3);
        pix2 = pixaDisplayTiledInColumns(pixa1, 1, 0.5, 20, 2);
        snprintf(buf, sizeof(buf), "/tmp/lept/ital/ital.%d.png", type);
        pixWriteDebug(buf, pix2, IFF_PNG);
        pixDestroy(&pix2);

            /* Assuming the image represents 6 inches of actual page width,
             * the pixs resolution is approximately
             *    (width of pixs in pixels) / 6
             * and the images have been saved at half this resolution.   */
        res = pixGetWidth(pixs) / 12;
        L_INFO("resolution = %d\n", __func__, res);
        l_pdfSetDateAndVersion(0);
        snprintf(buf, sizeof(buf), "/tmp/lept/ital/ital.%d.pdf", type);
        pixaConvertToPdf(pixa1, res, 1.0, L_FLATE_ENCODE, 75, "Italic Finder",
                         buf);
        l_pdfSetDateAndVersion(1);
        pixaDestroy(&pixa1);
        boxaDestroy(&boxat);

            /* Plot histogram of horizontal white run sizes.  A small
             * initial vertical dilation removes most runs that are neither
             * inter-character nor inter-word.  The larger first peak is
             * from inter-character runs, and the smaller second peak is
             * from inter-word runs. */
        pix1 = pixDilateBrick(NULL, pixs, 1, 15);
        upper = L_MAX(30, 3 * size);
        na = pixRunHistogramMorph(pix1, L_RUN_OFF, L_HORIZ, upper);
        pixDestroy(&pix1);
        gplot = gplotCreate("/tmp/lept/ital/runhisto", GPLOT_PNG,
                "Histogram of horizontal runs of white pixels, vs length",
                "run length", "number of runs");
        gplotAddPlot(gplot, NULL, na, GPLOT_LINES, "plot1");
        gplotMakeOutput(gplot);
        gplotDestroy(&gplot);
        numaDestroy(&na);
    }

    selDestroy(&sel_ital1);
    selDestroy(&sel_ital2);
    selDestroy(&sel_ital3);
    pixDestroy(&pixsd);
    pixDestroy(&pixm);
    pixDestroy(&pixd);
    return 0;
}
