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
 * find_colorregions.c
 *
 *   This shows the output of pixHasColorRegions(), which attempts to locate
 *   colored regions on scanned images.  The difficulty arises when the
 *   scanned images are oxidized, dark and reddish.
 *
 *   It also shows output from pixFindColorRegionsLight(), which is an
 *   inferior implementation that does not work on images with a dark
 *   background.
 *
 *   The input image should be RGB at 75 ppi resolution.
 *
 *   Use, e.g. these 75 ppi images:
 *     map.057.jpg
 *     colorpage.030.jpg
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

    /* Implementation is in this file */
static l_int32
pixFindColorRegionsLight(PIX *pixs, PIX *pixm, l_int32 factor,
                    l_int32 darkthresh, l_int32 lightthresh, l_int32 mindiff,
                    l_int32 colordiff, l_float32 *pcolorfract,
                    PIX **pcolormask1, PIX **pcolormask2, PIXA *pixadb);

int main(int    argc,
         char **argv)
{
l_float32  fcolor;
PIX       *pix1, *pix2, *pix3, *pix4;
PIXA      *pixadb;

    setLeptDebugOK(1);
    lept_mkdir("lept/color");
    pix1 = pixRead("colorpage.030.jpg");
/*    pix1 = pixRead("map.057.jpg"); */

        /* More general method */
    pixadb = pixaCreate(0);
    pixFindColorRegions(pix1, NULL, 4, 200, 60, 10, 90, 0.05,
                        &fcolor, &pix3, &pix4, pixadb);
    lept_stderr("ncolor = %f\n", fcolor);
    if (pix3) pixDisplay(pix3, 0, 800);
    if (pix4) pixDisplay(pix4, 600, 800);

    pix2 = pixaDisplayTiledInColumns(pixadb, 5, 0.3, 20, 2);
    pixDisplay(pix2, 0, 0);
    pixWrite("/tmp/lept/color/result1.png", pix2, IFF_PNG);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);
    pixaDestroy(&pixadb);

        /* Method for pages with very light background */
    pixadb = pixaCreate(0);
    pixFindColorRegionsLight(pix1, NULL, 4, 60, 230, 40, 20,
                             &fcolor, &pix3, &pix4, pixadb);
    lept_stderr("ncolor = %f\n", fcolor);
    if (pix3) pixDisplay(pix3, 1100, 800);
    if (pix4) pixDisplay(pix4, 1700, 800);

    pix2 = pixaDisplayTiledInColumns(pixadb, 5, 0.3, 20, 2);
    pixDisplay(pix2, 1100, 0);
    pixWrite("/tmp/lept/color/result2.png", pix2, IFF_PNG);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);
    pixaDestroy(&pixadb);

    pixDestroy(&pix1);
    return 0;
}


/*!
 * Note: this method is generally inferior to pixHasColorRegions(); it
 *       is retained as a reference only
 *
 * \brief   pixFindColorRegionsLight()
 *
 * \param[in]    pixs        32 bpp rgb
 * \param[in]    pixm        [optional] 1 bpp mask image
 * \param[in]    factor      subsample factor; integer >= 1
 * \param[in]    darkthresh  threshold to eliminate dark pixels (e.g., text)
 *                           from consideration; typ. 70; -1 for default.
 * \param[in]    lightthresh threshold for minimum gray value at 95% rank
 *                           near white; typ. 220; -1 for default
 * \param[in]    mindiff     minimum difference from 95% rank value, used
 *                           to count darker pixels; typ. 50; -1 for default
 * \param[in]    colordiff   minimum difference in (max - min) component to
 *                           qualify as a color pixel; typ. 40; -1 for default
 * \param[out]   pcolorfract fraction of 'color' pixels found
 * \param[out]   pcolormask1 [optional] mask over background color, if any
 * \param[out]   pcolormask2 [optional] filtered mask over background color
 * \param[out]   pixadb      [optional] debug intermediate results
 * \return  0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *      (1) This function tries to determine if there is a significant
 *          color or darker region on a scanned page image where part
 *          of the image is very close to "white".  It will also allow
 *          extraction of small regions of lightly colored pixels.
 *          If the background is darker (and reddish), use instead
 *          pixHasColorRegions2().
 *      (2) If %pixm exists, only pixels under fg are considered. Typically,
 *          the inverse of %pixm would have fg pixels over a photograph.
 *      (3) There are four thresholds.
 *          * %darkthresh: ignore pixels darker than this (typ. fg text).
 *            We make a 1 bpp mask of these pixels, and then dilate it to
 *            remove all vestiges of fg from their vicinity.
 *          * %lightthresh: let val95 be the pixel value for which 95%
 *            of the non-masked pixels have a lower value (darker) of
 *            their min component.  Then if val95 is darker than
 *            %lightthresh, the image is not considered to have a
 *            light bg, and this returns 0.0 for %colorfract.
 *          * %mindiff: we are interested in the fraction of pixels that
 *            have two conditions.  The first is that their min component
 *            is at least %mindiff darker than val95.
 *          * %colordiff: the second condition is that the max-min diff
 *            of the pixel components exceeds %colordiff.
 *      (4) This returns in %pcolorfract the fraction of pixels that have
 *          both a min component that is at least %mindiff below that at the
 *          95% rank value (where 100% rank is the lightest value), and
 *          a max-min diff that is at least %colordiff.  Without the
 *          %colordiff constraint, gray pixels of intermediate value
 *          could get flagged by this function.
 *      (5) No masks are returned unless light color pixels are found.
 *          If colorfract > 0.0 and %pcolormask1 is defined, this returns
 *          a 1 bpp mask with fg pixels over the color background.
 *          This mask may have some holes in it.
 *      (6) If colorfract > 0.0 and %pcolormask2 is defined, this returns
 *          a filtered version of colormask1.  The two changes are
 *            (a) small holes have been filled
 *            (b) components near the border have been removed.
 *          The latter insures that dark pixels near the edge of the
 *          image are not included.
 *      (7) To generate a boxa of rectangular regions from the overlap
 *          of components in the filtered mask:
 *                boxa1 = pixConnCompBB(colormask2, 8);
 *                boxa2 = boxaCombineOverlaps(boxa1);
 *          This is done here in debug mode.
 * </pre>
 */
static l_int32
pixFindColorRegionsLight(PIX        *pixs,
                         PIX        *pixm,
                         l_int32     factor,
                         l_int32     darkthresh,
                         l_int32     lightthresh,
                         l_int32     mindiff,
                         l_int32     colordiff,
                         l_float32  *pcolorfract,
                         PIX       **pcolormask1,
                         PIX       **pcolormask2,
                         PIXA       *pixadb)
{
l_int32    lightbg, w, h, count;
l_float32  ratio, val95;
BOXA      *boxa1, *boxa2;
NUMA      *nah;
PIX       *pix1, *pix2, *pix3, *pix4, *pix5, *pixm1, *pixm2, *pixm3;

    if (pcolormask1) *pcolormask1 = NULL;
    if (pcolormask2) *pcolormask2 = NULL;
    if (!pcolorfract)
        return ERROR_INT("&colorfract not defined", __func__, 1);
    *pcolorfract = 0.0;
    if (!pixs || pixGetDepth(pixs) != 32)
        return ERROR_INT("pixs not defined or not 32 bpp", __func__, 1);
    if (factor < 1) factor = 1;
    if (darkthresh < 0) darkthresh = 70;  /* defaults */
    if (lightthresh < 0) lightthresh = 220;
    if (mindiff < 0) mindiff = 50;
    if (colordiff < 0) colordiff = 40;

        /* Check if pixm covers most of the image.  If so, just return. */
    pixGetDimensions(pixs, &w, &h, NULL);
    if (pixm) {
        pixCountPixels(pixm, &count, NULL);
        ratio = (l_float32)count / ((l_float32)(w) * h);
        if (ratio > 0.7) {
            if (pixadb) L_INFO("pixm has big fg: %f5.2\n", __func__, ratio);
            return 0;
        }
    }

        /* Make a mask pixm1 over the dark pixels in the image:
         * convert to gray using the average of the components;
         * threshold using %darkthresh; do a small dilation;
         * combine with pixm. */
    pix1 = pixConvertRGBToGray(pixs, 0.33, 0.34, 0.33);
    if (pixadb) pixaAddPix(pixadb, pixs, L_COPY);
    if (pixadb) pixaAddPix(pixadb, pix1, L_COPY);
    pixm1 = pixThresholdToBinary(pix1, darkthresh);
    pixDilateBrick(pixm1, pixm1, 7, 7);
    if (pixadb) pixaAddPix(pixadb, pixm1, L_COPY);
    if (pixm) {
        pixOr(pixm1, pixm1, pixm);
        if (pixadb) pixaAddPix(pixadb, pixm1, L_COPY);
    }
    pixDestroy(&pix1);

        /* Convert to gray using the minimum component value and
         * find the gray value at rank 0.95, that represents the light
         * pixels in the image.  If it is too dark, quit. */
    pix1 = pixConvertRGBToGrayMinMax(pixs, L_SELECT_MIN);
    pix2 = pixInvert(NULL, pixm1);  /* pixels that are not dark */
    pixGetRankValueMasked(pix1, pix2, 0, 0, factor, 0.95, &val95, &nah);
    pixDestroy(&pix2);
    if (pixadb) {
        L_INFO("val at 0.95 rank = %5.1f\n", __func__, val95);
        gplotSimple1(nah, GPLOT_PNG, "/tmp/lept/histo1", "gray histo");
        pix3 = pixRead("/tmp/lept/histo1.png");
        pix4 = pixExpandReplicate(pix3, 2);
        pixaAddPix(pixadb, pix4, L_INSERT);
        pixDestroy(&pix3);
    }
    lightbg = (l_int32)val95 >= lightthresh;
    numaDestroy(&nah);
    if (!lightbg) {
        pixDestroy(&pix1);
        pixDestroy(&pixm1);
        return 0;
    }

        /* Make mask pixm2 over pixels that are darker than val95 - mindiff. */
    pixm2 = pixThresholdToBinary(pix1, val95 - mindiff);
    if (pixadb) pixaAddPix(pixadb, pixm2, L_COPY);
    pixDestroy(&pix1);

        /* Make a mask pixm3 over pixels that have some color saturation,
         * with a (max - min) component difference >= %colordiff,
         * and combine using AND with pixm2. */
    pix2 = pixConvertRGBToGrayMinMax(pixs, L_CHOOSE_MAXDIFF);
    pixm3 = pixThresholdToBinary(pix2, colordiff);
    pixDestroy(&pix2);
    pixInvert(pixm3, pixm3);  /* need pixels above threshold */
    if (pixadb) pixaAddPix(pixadb, pixm3, L_COPY);
    pixAnd(pixm2, pixm2, pixm3);
    if (pixadb) pixaAddPix(pixadb, pixm2, L_COPY);
    pixDestroy(&pixm3);

        /* Subtract the dark pixels represented by pixm1.
         * pixm2 now holds all the color pixels of interest  */
    pixSubtract(pixm2, pixm2, pixm1);
    pixDestroy(&pixm1);
    if (pixadb) pixaAddPix(pixadb, pixm2, L_COPY);

        /* But we're not quite finished.  Remove pixels from any component
         * that is touching the image border.  False color pixels can
         * sometimes be found there if the image is much darker near
         * the border, due to oxidation or reduced illumination. */
    pixm3 = pixRemoveBorderConnComps(pixm2, 8);
    pixDestroy(&pixm2);
    if (pixadb) pixaAddPix(pixadb, pixm3, L_COPY);

        /* Get the fraction of light color pixels */
    pixCountPixels(pixm3, &count, NULL);
    *pcolorfract = (l_float32)count / (w * h);
    if (pixadb) {
        if (count == 0)
            L_INFO("no light color pixels found\n", __func__);
        else
            L_INFO("fraction of light color pixels = %5.3f\n", __func__,
                   *pcolorfract);
    }

        /* Debug: extract the color pixels from pixs */
    if (pixadb && count > 0) {
            /* Use pixm3 to extract the color pixels */
        pix3 = pixCreateTemplate(pixs);
        pixSetAll(pix3);
        pixCombineMasked(pix3, pixs, pixm3);
        pixaAddPix(pixadb, pix3, L_INSERT);

            /* Use additional filtering to extract the color pixels */
        pix3 = pixCloseSafeBrick(NULL, pixm3, 15, 15);
        pixaAddPix(pixadb, pix3, L_INSERT);
        pix5 = pixCreateTemplate(pixs);
        pixSetAll(pix5);
        pixCombineMasked(pix5, pixs, pix3);
        pixaAddPix(pixadb, pix5, L_INSERT);

            /* Get the combined bounding boxes of the mask components
             * in pix3, and extract those pixels from pixs. */
        boxa1 = pixConnCompBB(pix3, 8);
        boxa2 = boxaCombineOverlaps(boxa1, NULL);
        pix4 = pixCreateTemplate(pix3);
        pixMaskBoxa(pix4, pix4, boxa2, L_SET_PIXELS);
        pixaAddPix(pixadb, pix4, L_INSERT);
        pix5 = pixCreateTemplate(pixs);
        pixSetAll(pix5);
        pixCombineMasked(pix5, pixs, pix4);
        pixaAddPix(pixadb, pix5, L_INSERT);
        boxaDestroy(&boxa1);
        boxaDestroy(&boxa2);
        pixaAddPix(pixadb, pixs, L_COPY);
    }

        /* Optional colormask returns */
    if (pcolormask2 && count > 0)
        *pcolormask2 = pixCloseSafeBrick(NULL, pixm3, 15, 15);
    if (pcolormask1 && count > 0)
        *pcolormask1 = pixm3;
    else
        pixDestroy(&pixm3);
    return 0;
}
