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
 * \file ccthin.c
 * <pre>
 *
 *     PIXA   *pixaThinConnected()
 *     PIX    *pixThinConnected()
 *     PIX    *pixThinConnectedBySet()
 *     SELA   *selaMakeThinSets()
 * </pre>
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

    /* ------------------------------------------------------------
     * The sels used here (and their rotated counterparts) are the
     * useful 3x3 Sels for thinning.   They are defined in sel2.c,
     * and the sets are constructed in selaMakeThinSets().
     * The notation is based on "Connectivity-preserving morphological
     * image transformations", a version of which can be found at
     *           http://www.leptonica.com/papers/conn.pdf
     * ------------------------------------------------------------ */

/*----------------------------------------------------------------*
 *                      CC-preserving thinning                    *
 *----------------------------------------------------------------*/
/*!
 * \brief   pixaThinConnected()
 *
 * \param[in]   pixas          of 1 bpp pix
 * \param[in]   type           L_THIN_FG, L_THIN_BG
 * \param[in]   connectivity   4 or 8
 * \param[in]   maxiters       max number of iters allowed;
 *                             use 0 to iterate until completion
 * \return  pixds, or NULL on error
 *
 * <pre>
 * Notes:
 *      (1) See notes in pixThinConnected().
 * </pre>
 */
PIXA *
pixaThinConnected(PIXA    *pixas,
                  l_int32  type,
                  l_int32  connectivity,
                  l_int32  maxiters)
{
l_int32  i, n, d, same;
PIX     *pix1, *pix2;
PIXA    *pixad;
SELA    *sela;

    if (!pixas)
        return (PIXA *)ERROR_PTR("pixas not defined", __func__, NULL);
    if (type != L_THIN_FG && type != L_THIN_BG)
        return (PIXA *)ERROR_PTR("invalid fg/bg type", __func__, NULL);
    if (connectivity != 4 && connectivity != 8)
        return (PIXA *)ERROR_PTR("connectivity not 4 or 8", __func__, NULL);
    if (maxiters == 0) maxiters = 10000;

    pixaVerifyDepth(pixas, &same, &d);
    if (d != 1)
        return (PIXA *)ERROR_PTR("pix are not all 1 bpp", __func__, NULL);

    if (connectivity == 4)
        sela = selaMakeThinSets(1, 0);
    else  /* connectivity == 8 */
        sela = selaMakeThinSets(5, 0);

    n = pixaGetCount(pixas);
    pixad = pixaCreate(n);
    for (i = 0; i < n; i++) {
        pix1 = pixaGetPix(pixas, i, L_CLONE);
        pix2 = pixThinConnectedBySet(pix1, type, sela, maxiters);
        pixaAddPix(pixad, pix2, L_INSERT);
        pixDestroy(&pix1);
    }

    selaDestroy(&sela);
    return pixad;
}


/*!
 * \brief   pixThinConnected()
 *
 * \param[in]   pixs           1 bpp
 * \param[in]   type           L_THIN_FG, L_THIN_BG
 * \param[in]   connectivity   4 or 8
 * \param[in]   maxiters       max number of iters allowed;
 *                             use 0 to iterate until completion
 * \return  pixd, or NULL on error
 *
 * <pre>
 * Notes:
 *      (1) See "Connectivity-preserving morphological image transformations,"
 *          Dan S. Bloomberg, in SPIE Visual Communications and Image
 *          Processing, Conference 1606, pp. 320-334, November 1991,
 *          Boston, MA.   A web version is available at
 *              http://www.leptonica.com/papers/conn.pdf
 *      (2) This is a simple interface for two of the best iterative
 *          morphological thinning algorithms, for 4-c.c and 8-c.c.
 *          Each iteration uses a mixture of parallel operations
 *          (using several different 3x3 Sels) and serial operations.
 *          Specifically, each thinning iteration consists of
 *          four sequential thinnings from each of four directions.
 *          Each of these thinnings is a parallel composite
 *          operation, where the union of a set of HMTs are set
 *          subtracted from the input.  For 4-cc thinning, we
 *          use 3 HMTs in parallel, and for 8-cc thinning we use 4 HMTs.
 *      (3) A "good" thinning algorithm is one that generates a skeleton
 *          that is near the medial axis and has neither pruned
 *          real branches nor left extra dendritic branches.
 *      (4) Duality between operations on fg and bg require switching
 *          the connectivity.  To thin the foreground, which is the usual
 *          situation, use type == L_THIN_FG.  Thickening the foreground
 *          is equivalent to thinning the background (type == L_THIN_BG),
 *          where the alternate connectivity gets preserved.
 *          For example, to thicken the fg with 2 rounds of iterations
 *          using 4-c.c., thin the bg using Sels that preserve 8-connectivity:
 *             Pix *pix = pixThinConnected(pixs, L_THIN_BG, 8, 2);
 *      (5) This makes and destroys the sela set each time. It's not a large
 *          overhead, but if you are calling this thousands of times on
 *          very small images, you can avoid the overhead; e.g.
 *             Sela *sela = selaMakeThinSets(1, 0);  // for 4-c.c.
 *             Pix *pix = pixThinConnectedBySet(pixs, L_THIN_FG, sela, 0);
 *          using set 1 for 4-c.c. and set 5 for 8-c.c operations.
 * </pre>
 */
PIX *
pixThinConnected(PIX     *pixs,
                 l_int32  type,
                 l_int32  connectivity,
                 l_int32  maxiters)
{
PIX   *pixd;
SELA  *sela;

    if (!pixs)
        return (PIX *)ERROR_PTR("pixs not defined", __func__, NULL);
    if (pixGetDepth(pixs) != 1)
        return (PIX *)ERROR_PTR("pixs not 1 bpp", __func__, NULL);
    if (type != L_THIN_FG && type != L_THIN_BG)
        return (PIX *)ERROR_PTR("invalid fg/bg type", __func__, NULL);
    if (connectivity != 4 && connectivity != 8)
        return (PIX *)ERROR_PTR("connectivity not 4 or 8", __func__, NULL);
    if (maxiters == 0) maxiters = 10000;

    if (connectivity == 4)
        sela = selaMakeThinSets(1, 0);
    else  /* connectivity == 8 */
        sela = selaMakeThinSets(5, 0);

    pixd = pixThinConnectedBySet(pixs, type, sela, maxiters);

    selaDestroy(&sela);
    return pixd;
}


/*!
 * \brief   pixThinConnectedBySet()
 *
 * \param[in]   pixs       1 bpp
 * \param[in]   type       L_THIN_FG, L_THIN_BG
 * \param[in]   sela       of Sels for parallel composite HMTs
 * \param[in]   maxiters   max number of iters allowed;
 *                         use 0 to iterate until completion
 * \return  pixd, or NULL on error
 *
 * <pre>
 * Notes:
 *      (1) See notes in pixThinConnected().
 *      (2) This takes a sela representing one of 11 sets of HMT Sels.
 *          The HMTs from this set are run in parallel and the result
 *          is OR'd before being subtracted from the source.  For each
 *          iteration, this "parallel" thin is performed four times
 *          sequentially, for sels rotated by 90 degrees in all four
 *          directions.
 *      (3) The "parallel" and "sequential" nomenclature is standard
 *          in digital filtering.  Here, "parallel" operations work on the
 *          same source (pixd), and accumulate the results in a temp
 *          image before actually applying them to the source (in this
 *          case, using an in-place subtraction).  "Sequential" operations
 *          operate directly on the source (pixd) to produce the result
 *          (in this case, with four sequential thinning operations, one
 *          from each of four directions).
 * </pre>
 */
PIX *
pixThinConnectedBySet(PIX     *pixs,
                      l_int32  type,
                      SELA    *sela,
                      l_int32  maxiters)
{
l_int32  i, j, r, nsels, same;
PIXA    *pixahmt;
PIX    **pixhmt;  /* array owned by pixahmt; do not destroy! */
PIX     *pix1, *pix2, *pixd;
SEL     *sel, *selr;

    if (!pixs)
        return (PIX *)ERROR_PTR("pixs not defined", __func__, NULL);
    if (pixGetDepth(pixs) != 1)
        return (PIX *)ERROR_PTR("pixs not 1 bpp", __func__, NULL);
    if (type != L_THIN_FG && type != L_THIN_BG)
        return (PIX *)ERROR_PTR("invalid fg/bg type", __func__, NULL);
    if (!sela)
        return (PIX *)ERROR_PTR("sela not defined", __func__, NULL);
    if (maxiters == 0) maxiters = 10000;

        /* Set up array of temp pix to hold hmts */
    nsels = selaGetCount(sela);
    pixahmt = pixaCreate(nsels);
    for (i = 0; i < nsels; i++) {
        pix1 = pixCreateTemplate(pixs);
        pixaAddPix(pixahmt, pix1, L_INSERT);
    }
    pixhmt = pixaGetPixArray(pixahmt);
    if (!pixhmt) {
        pixaDestroy(&pixahmt);
        return (PIX *)ERROR_PTR("pixhmt array not made", __func__, NULL);
    }

        /* Set up initial image for fg thinning */
    if (type == L_THIN_FG)
        pixd = pixCopy(NULL, pixs);
    else  /* bg thinning */
        pixd = pixInvert(NULL, pixs);

        /* Thin the fg, with up to maxiters iterations */
    for (i = 0; i < maxiters; i++) {
        pix1 = pixCopy(NULL, pixd);  /* test for completion */
        for (r = 0; r < 4; r++) {  /* over 90 degree rotations of Sels */
            for (j = 0; j < nsels; j++) {  /* over individual sels in sela */
                sel = selaGetSel(sela, j);  /* not a copy */
                selr = selRotateOrth(sel, r);
                pixHMT(pixhmt[j], pixd, selr);
                selDestroy(&selr);
                if (j > 0)
                    pixOr(pixhmt[0], pixhmt[0], pixhmt[j]);  /* accum result */
            }
            pixSubtract(pixd, pixd, pixhmt[0]);  /* remove result */
        }
        pixEqual(pixd, pix1, &same);
        pixDestroy(&pix1);
        if (same) {
/*            L_INFO("%d iterations to completion\n", __func__, i); */
            break;
        }
    }

        /* This is a bit tricky. If we're thickening the foreground, then
         * we get a fg border of thickness equal to the number of
         * iterations.  This border is connected to all components that
         * were initially touching the border, but as it grows, it does
         * not touch other growing components -- it leaves a 1 pixel wide
         * background between it and the growing components, and that
         * thin background prevents the components from growing further.
         * This border can be entirely removed as follows:
         * (1) Subtract the original (unthickened) image pixs from the
         *     thickened image. This removes the pixels that were originally
         *     touching the border.
         * (2) Get all remaining pixels that are connected to the border.
         * (3) Remove those pixels from the thickened image.  */
    if (type == L_THIN_BG) {
        pixInvert(pixd, pixd);  /* finish with duality */
        pix1 = pixSubtract(NULL, pixd, pixs);
        pix2 = pixExtractBorderConnComps(pix1, 4);
        pixSubtract(pixd, pixd, pix2);
        pixDestroy(&pix1);
        pixDestroy(&pix2);
    }

    pixaDestroy(&pixahmt);
    return pixd;
}


/*!
 * \brief   selaMakeThinSets()
 *
 * \param[in]   index   into specific sets
 * \param[in]   debug   1 to output display of sela
 * \return  sela, or NULL on error
 *
 * <pre>
 * Notes:
 *      (1) These are specific sets of HMTs to be used in parallel for
 *          for thinning from each of four directions.
 *      (2) The sets are indexed as follows:
 *          For thinning (e.g., run to completion):
 *              index = 1     sel_4_1, sel_4_2, sel_4_3
 *              index = 2     sel_4_1, sel_4_5, sel_4_6
 *              index = 3     sel_4_1, sel_4_7, sel_4_7_rot
 *              index = 4     sel_48_1, sel_48_1_rot, sel_48_2
 *              index = 5     sel_8_2, sel_8_3, sel_8_5, sel_8_6
 *              index = 6     sel_8_2, sel_8_3, sel_48_2
 *              index = 7     sel_8_1, sel_8_5, sel_8_6
 *              index = 8     sel_8_2, sel_8_3, sel_8_8, sel_8_9
 *              index = 9     sel_8_5, sel_8_6, sel_8_7, sel_8_7_rot
 *          For thickening (e.g., just a few iterations):
 *              index = 10    sel_4_2, sel_4_3
 *              index = 11    sel_8_4
 *      (3) For a very smooth skeleton, use set 1 for 4 connected and
 *          set 5 for 8 connected thins.
 * </pre>
 */
SELA *
selaMakeThinSets(l_int32  index,
                 l_int32  debug)
{
SEL   *sel;
SELA  *sela1, *sela2, *sela3;

    if (index < 1 || index > 11)
        return (SELA *)ERROR_PTR("invalid index", __func__, NULL);

    sela2 = selaCreate(4);
    switch(index)
    {
    case 1:
        sela1 = sela4ccThin(NULL);
        selaFindSelByName(sela1, "sel_4_1", NULL, &sel);
        selaAddSel(sela2, sel, NULL, L_COPY);
        selaFindSelByName(sela1, "sel_4_2", NULL, &sel);
        selaAddSel(sela2, sel, NULL, L_COPY);
        selaFindSelByName(sela1, "sel_4_3", NULL, &sel);
        selaAddSel(sela2, sel, NULL, L_COPY);
        break;
    case 2:
        sela1 = sela4ccThin(NULL);
        selaFindSelByName(sela1, "sel_4_1", NULL, &sel);
        selaAddSel(sela2, sel, NULL, L_COPY);
        selaFindSelByName(sela1, "sel_4_5", NULL, &sel);
        selaAddSel(sela2, sel, NULL, L_COPY);
        selaFindSelByName(sela1, "sel_4_6", NULL, &sel);
        selaAddSel(sela2, sel, NULL, L_COPY);
        break;
    case 3:
        sela1 = sela4ccThin(NULL);
        selaFindSelByName(sela1, "sel_4_1", NULL, &sel);
        selaAddSel(sela2, sel, NULL, L_COPY);
        selaFindSelByName(sela1, "sel_4_7", NULL, &sel);
        selaAddSel(sela2, sel, NULL, L_COPY);
        sel = selRotateOrth(sel, 1);
        selaAddSel(sela2, sel, "sel_4_7_rot", L_INSERT);
        break;
    case 4:
        sela1 = sela4and8ccThin(NULL);
        selaFindSelByName(sela1, "sel_48_1", NULL, &sel);
        selaAddSel(sela2, sel, NULL, L_COPY);
        sel = selRotateOrth(sel, 1);
        selaAddSel(sela2, sel, "sel_48_1_rot", L_INSERT);
        selaFindSelByName(sela1, "sel_48_2", NULL, &sel);
        selaAddSel(sela2, sel, NULL, L_COPY);
        break;
    case 5:
        sela1 = sela8ccThin(NULL);
        selaFindSelByName(sela1, "sel_8_2", NULL, &sel);
        selaAddSel(sela2, sel, NULL, L_COPY);
        selaFindSelByName(sela1, "sel_8_3", NULL, &sel);
        selaAddSel(sela2, sel, NULL, L_COPY);
        selaFindSelByName(sela1, "sel_8_5", NULL, &sel);
        selaAddSel(sela2, sel, NULL, L_COPY);
        selaFindSelByName(sela1, "sel_8_6", NULL, &sel);
        selaAddSel(sela2, sel, NULL, L_COPY);
        break;
    case 6:
        sela1 = sela8ccThin(NULL);
        sela3 = sela4and8ccThin(NULL);
        selaFindSelByName(sela1, "sel_8_2", NULL, &sel);
        selaAddSel(sela2, sel, NULL, L_COPY);
        selaFindSelByName(sela1, "sel_8_3", NULL, &sel);
        selaAddSel(sela2, sel, NULL, L_COPY);
        selaFindSelByName(sela3, "sel_48_2", NULL, &sel);
        selaAddSel(sela2, sel, NULL, L_COPY);
        selaDestroy(&sela3);
        break;
    case 7:
        sela1 = sela8ccThin(NULL);
        selaFindSelByName(sela1, "sel_8_1", NULL, &sel);
        selaAddSel(sela2, sel, NULL, L_COPY);
        selaFindSelByName(sela1, "sel_8_5", NULL, &sel);
        selaAddSel(sela2, sel, NULL, L_COPY);
        selaFindSelByName(sela1, "sel_8_6", NULL, &sel);
        selaAddSel(sela2, sel, NULL, L_COPY);
        break;
    case 8:
        sela1 = sela8ccThin(NULL);
        selaFindSelByName(sela1, "sel_8_2", NULL, &sel);
        selaAddSel(sela2, sel, NULL, L_COPY);
        selaFindSelByName(sela1, "sel_8_3", NULL, &sel);
        selaAddSel(sela2, sel, NULL, L_COPY);
        selaFindSelByName(sela1, "sel_8_8", NULL, &sel);
        selaAddSel(sela2, sel, NULL, L_COPY);
        selaFindSelByName(sela1, "sel_8_9", NULL, &sel);
        selaAddSel(sela2, sel, NULL, L_COPY);
        break;
    case 9:
        sela1 = sela8ccThin(NULL);
        selaFindSelByName(sela1, "sel_8_5", NULL, &sel);
        selaAddSel(sela2, sel, NULL, L_COPY);
        selaFindSelByName(sela1, "sel_8_6", NULL, &sel);
        selaAddSel(sela2, sel, NULL, L_COPY);
        selaFindSelByName(sela1, "sel_8_7", NULL, &sel);
        selaAddSel(sela2, sel, NULL, L_COPY);
        sel = selRotateOrth(sel, 1);
        selaAddSel(sela2, sel, "sel_8_7_rot", L_INSERT);
        break;
    case 10:  /* thicken for this one; use just a few iterations */
        sela1 = sela4ccThin(NULL);
        selaFindSelByName(sela1, "sel_4_2", NULL, &sel);
        selaAddSel(sela2, sel, NULL, L_COPY);
        selaFindSelByName(sela1, "sel_4_3", NULL, &sel);
        selaAddSel(sela2, sel, NULL, L_COPY);
        break;
    case 11:  /* thicken for this one; use just a few iterations */
        sela1 = sela8ccThin(NULL);
        selaFindSelByName(sela1, "sel_8_4", NULL, &sel);
        selaAddSel(sela2, sel, NULL, L_COPY);
        break;
    }

        /* Optionally display the sel set */
    if (debug) {
        PIX  *pix1;
        char  buf[32];
        lept_mkdir("/lept/sels");
        pix1 = selaDisplayInPix(sela2, 35, 3, 15, 4);
        snprintf(buf, sizeof(buf), "/tmp/lept/sels/set%d.png", index);
        pixWrite(buf, pix1, IFF_PNG);
        pixDisplay(pix1, 100, 100);
        pixDestroy(&pix1);
    }

    selaDestroy(&sela1);
    return sela2;
}
