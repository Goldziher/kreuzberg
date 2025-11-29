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
 * jbwords.c
 *
 *     jbwords dirin thresh weight rootname [firstpage npages]
 *
 *         dirin:  directory of input pages
 *         reduction: 1 (full res) or 2 (half-res)
 *         thresh: 0.80 is a reasonable compromise between accuracy
 *                 and number of classes, for characters
 *         weight: 0.6 seems to work reasonably with thresh = 0.8.
 *         rootname: used for naming the two output files (templates
 *                   and c.c. data)
 *         firstpage: <optional> 0-based; default is 0
 *         npages: <optional> use 0 for all pages; default is 0
 *
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

    /* Eliminate very large "words" */
static const l_int32  MAX_WORD_WIDTH = 500;
static const l_int32  MAX_WORD_HEIGHT = 200;

#define   BUF_SIZE                  512

    /* select additional debug output */
#define   RENDER_PAGES              1
#define   RENDER_DEBUG              1


int main(int    argc,
         char **argv)
{
char        filename[BUF_SIZE];
char       *dirin, *rootname;
l_int32     reduction, i, firstpage, npages;
l_float32   thresh, weight;
JBDATA     *data;
JBCLASSER  *classer;
NUMA       *natl;
PIX        *pix;
PIXA       *pixa, *pixadb;

    if (argc != 6 && argc != 8)
        return ERROR_INT(" Syntax: jbwords dirin reduction thresh "
                         "weight rootname [firstpage, npages]", __func__, 1);
    dirin = argv[1];
    reduction = atoi(argv[2]);
    thresh = atof(argv[3]);
    weight = atof(argv[4]);
    rootname = argv[5];
    if (argc == 6) {
        firstpage = 0;
        npages = 0;
    } else {
        firstpage = atoi(argv[6]);
        npages = atoi(argv[7]);
    }
    setLeptDebugOK(1);

    classer = jbWordsInTextlines(dirin, reduction, MAX_WORD_WIDTH,
                                 MAX_WORD_HEIGHT, thresh, weight,
                                 &natl, firstpage, npages);

        /* Save and write out the result */
    data = jbDataSave(classer);
    jbDataWrite(rootname, data);

#if  RENDER_PAGES
        /* Render the pages from the classifier data, and write to file.
         * Use debugflag == FALSE to omit outlines of each component. */
    pixa = jbDataRender(data, FALSE);
    npages = pixaGetCount(pixa);
    for (i = 0; i < npages; i++) {
        pix = pixaGetPix(pixa, i, L_CLONE);
        snprintf(filename, BUF_SIZE, "%s.%05d", rootname, i);
        lept_stderr("filename: %s\n", filename);
        pixWrite(filename, pix, IFF_PNG);
        pixDestroy(&pix);
    }
    pixaDestroy(&pixa);
#endif  /* RENDER_PAGES */

#if  RENDER_DEBUG
        /* Use debugflag == TRUE to see outlines of each component. */
    pixadb = jbDataRender(data, TRUE);
        /* Write the debug pages out */
    npages = pixaGetCount(pixadb);
    for (i = 0; i < npages; i++) {
        pix = pixaGetPix(pixadb, i, L_CLONE);
        snprintf(filename, BUF_SIZE, "%s.db.%05d", rootname, i);
        lept_stderr("filename: %s\n", filename);
        pixWrite(filename, pix, IFF_PNG);
        pixDestroy(&pix);
    }
    pixaDestroy(&pixadb);
#endif  /* RENDER_DEBUG */

    jbClasserDestroy(&classer);
    jbDataDestroy(&data);
    numaDestroy(&natl);
    return 0;
}

