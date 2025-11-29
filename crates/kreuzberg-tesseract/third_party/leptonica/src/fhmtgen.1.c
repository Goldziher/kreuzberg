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
 *      Top-level fast hit-miss transform with auto-generated sels
 *
 *             PIX     *pixHMTDwa_1()
 *             PIX     *pixFHMTGen_1()
 */

#include <string.h>
#include "allheaders.h"

PIX *pixHMTDwa_1(PIX *pixd, PIX *pixs, const char *selname);
PIX *pixFHMTGen_1(PIX *pixd, PIX *pixs, const char *selname);
l_int32 fhmtgen_low_1(l_uint32 *datad, l_int32 w,
                      l_int32 h, l_int32 wpld,
                      l_uint32 *datas, l_int32 wpls,
                      l_int32 index);

static l_int32   NUM_SELS_GENERATED = 10;
static char  SEL_NAMES[][80] = {
                             "sel_3hm",
                             "sel_3de",
                             "sel_3ue",
                             "sel_3re",
                             "sel_3le",
                             "sel_sl1",
                             "sel_ulc",
                             "sel_urc",
                             "sel_llc",
                             "sel_lrc"};

/*!
 * \brief  pixHMTDwa_1()
 *
 * \param[in]    pixd      usual 3 choices: null, == pixs, != pixs
 * \param[in]    pixs      1 bpp
 * \param[in]    sel       name
 * \return   pixd
 *
 * <pre>
 * Notes:
 *      (1) This simply adds a 32 pixel border, calls the appropriate
 *          pixFHMTGen_*(), and removes the border.
 *          See notes below for that function.
 * </pre>
 */
PIX *
pixHMTDwa_1(PIX         *pixd,
            PIX         *pixs,
            const char  *selname)
{
PIX  *pixt1, *pixt2, *pixt3;

    if (!pixs)
        return (PIX *)ERROR_PTR("pixs not defined", __func__, pixd);
    if (pixGetDepth(pixs) != 1)
        return (PIX *)ERROR_PTR("pixs must be 1 bpp", __func__, pixd);

    pixt1 = pixAddBorder(pixs, 32, 0);
    pixt2 = pixFHMTGen_1(NULL, pixt1, selname);
    pixt3 = pixRemoveBorder(pixt2, 32);
    pixDestroy(&pixt1);
    pixDestroy(&pixt2);

    if (!pixd)
        return pixt3;

    pixCopy(pixd, pixt3);
    pixDestroy(&pixt3);
    return pixd;
}


/*!
 * \brief  pixFHMTGen_1()
 *
 * \param[in]    pixd     usual 3 choices: null, == pixs, != pixs
 * \param[in]    pixs     1 bpp
 * \param[in]    sel      name
 * \return   pixd
 *
 * <pre>
 * Notes:
 *      (1) This is a dwa implementation of the hit-miss transform
 *          on pixs by the sel.
 *      (2) The sel must be limited in size to not more than 31 pixels
 *          about the origin.  It must have at least one hit, and it
 *          can have any number of misses.
 *      (3) This handles all required setting of the border pixels
 *          before erosion and dilation.
 * </pre>
 */
PIX *
pixFHMTGen_1(PIX         *pixd,
             PIX         *pixs,
             const char  *selname)
{
l_int32    i, index, found, w, h, wpls, wpld;
l_uint32  *datad, *datas, *datat;
PIX       *pixt;

    if (!pixs)
        return (PIX *)ERROR_PTR("pixs not defined", __func__, pixd);
    if (pixGetDepth(pixs) != 1)
        return (PIX *)ERROR_PTR("pixs must be 1 bpp", __func__, pixd);

    found = FALSE;
    for (i = 0; i < NUM_SELS_GENERATED; i++) {
        if (strcmp(selname, SEL_NAMES[i]) == 0) {
            found = TRUE;
            index = i;
            break;
        }
    }
    if (found == FALSE)
        return (PIX *)ERROR_PTR("sel index not found", __func__, pixd);

    if (!pixd) {
        if ((pixd = pixCreateTemplate(pixs)) == NULL)
            return (PIX *)ERROR_PTR("pixd not made", __func__, NULL);
    }
    else  /* for in-place or pre-allocated */
        pixResizeImageData(pixd, pixs);
    wpls = pixGetWpl(pixs);
    wpld = pixGetWpl(pixd);

        /*  The images must be surrounded with 32 additional border
         *  pixels, that we'll read from.  We fabricate a "proper"
         *  image as the subimage within the border, having the
         *  following parameters:  */
    w = pixGetWidth(pixs) - 64;
    h = pixGetHeight(pixs) - 64;
    datas = pixGetData(pixs) + 32 * wpls + 1;
    datad = pixGetData(pixd) + 32 * wpld + 1;

    if (pixd == pixs) {  /* need temp image if in-place */
        if ((pixt = pixCopy(NULL, pixs)) == NULL)
            return (PIX *)ERROR_PTR("pixt not made", __func__, pixd);
        datat = pixGetData(pixt) + 32 * wpls + 1;
        fhmtgen_low_1(datad, w, h, wpld, datat, wpls, index);
        pixDestroy(&pixt);
    }
    else {  /* not in-place */
        fhmtgen_low_1(datad, w, h, wpld, datas, wpls, index);
    }

    return pixd;
}
