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
 * jbcorrelation.c
 *
 *     jbcorrelation dirin thresh weight [firstpage npages]
 *
 *         dirin:  directory of input pages
 *         thresh: 0.80 - 0.85 is a reasonable compromise between accuracy
 *                 and number of classes, for characters
 *         weight: 0.6 seems to work reasonably with thresh = 0.8.
 *
 *     Notes:
 *         (1) All components larger than a default size are not saved.
 *             The default size is given in jbclass.c.
 *         (2) The two output files (for templates and c.c. data)
 *             are written with the rootname
 *               /tmp/lept/jb_correl/result
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

    /* Choose one of these */
#define  COMPONENTS  JB_CONN_COMPS
/* #define  COMPONENTS  JB_CHARACTERS */
/* #define  COMPONENTS  JB_WORDS */

#define   BUF_SIZE         512

    /* Select additional debug output */
#define   DEBUG_TEST_DATA_IO        0
#define   RENDER_DEBUG              1
#define   DISPLAY_DIFFERENCE        1
#define   DISPLAY_ALL_INSTANCES     0

static const char  rootname[] = "/tmp/lept/jb_correl/result";

int main(int    argc,
         char **argv)
{
char        filename[BUF_SIZE];
char       *dirin;
l_int32     i, firstpage, npages, nfiles;
l_float32   thresh, weight;
JBDATA     *data;
JBCLASSER  *classer;
SARRAY     *safiles;
PIX        *pix;
PIXA       *pixa, *pixadb;

    if (argc != 4 && argc != 6)
        return ERROR_INT(
             " Syntax: jbcorrelation dirin thresh weight [firstpage, npages]",
             __func__, 1);
    dirin = argv[1];
    thresh = atof(argv[2]);
    weight = atof(argv[3]);

    if (argc == 4) {
        firstpage = 0;
        npages = 0;
    }
    else {
        firstpage = atoi(argv[4]);
        npages = atoi(argv[5]);
    }

    setLeptDebugOK(1);
    lept_mkdir("lept/jb_correl");

#if 0  /* Choose library function or detailed steps */

    /*--------------------------------------------------------------*/

    jbCorrelation(dirin, thresh, weight, COMPONENTS, rootname,
                  firstpage, npages, 1);

    /*--------------------------------------------------------------*/

#else

    /*--------------------------------------------------------------*/

    safiles = getSortedPathnamesInDirectory(dirin, NULL, firstpage, npages);
    nfiles = sarrayGetCount(safiles);
/*    sarrayWriteStderr(safiles); */

        /* Classify components on requested pages */
    startTimer();
    classer = jbCorrelationInit(COMPONENTS, 0, 0, thresh, weight);
    jbAddPages(classer, safiles);
    lept_stderr("Time to generate classes: %6.3f sec\n", stopTimer());

        /* Save and write out the result */
    data = jbDataSave(classer);
    jbDataWrite(rootname, data);
    if (classer)
        lept_stderr("Number of classes: %d\n", classer->nclass);

        /* Render the pages from the classifier data.
         * Use debugflag == FALSE to omit outlines of each component. */
    pixa = jbDataRender(data, FALSE);

        /* Write the pages out */
    npages = pixaGetCount(pixa);
    if (npages != nfiles)
        lept_stderr("npages = %d, nfiles = %d, not equal!\n", npages, nfiles);
    for (i = 0; i < npages; i++) {
        pix = pixaGetPix(pixa, i, L_CLONE);
        snprintf(filename, BUF_SIZE, "%s.%03d", rootname, i);
        lept_stderr("filename: %s\n", filename);
        pixWrite(filename, pix, IFF_PNG);
        pixDestroy(&pix);
    }

  #if  DISPLAY_DIFFERENCE
    {
    char *fname;
    PIX  *pix1, *pix2;
    fname = sarrayGetString(safiles, 0, L_NOCOPY);
    pix1 = pixRead(fname);
    pix2 = pixaGetPix(pixa, 0, L_CLONE);
    pixXor(pix1, pix1, pix2);
    pixWrite("/tmp/lept/jb/output_diff.png", pix1, IFF_PNG);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    }
  #endif  /* DISPLAY_DIFFERENCE */

  #if  DEBUG_TEST_DATA_IO
    {
    JBDATA  *newdata;
    PIX     *newpix;
    PIXA    *newpixa;
    l_int32  same, iofail;

        /* Read the data back in and render the pages */
    newdata = jbDataRead(rootname);
    newpixa = jbDataRender(newdata, FALSE);
    iofail = FALSE;
    for (i = 0; i < npages; i++) {
        pix = pixaGetPix(pixa, i, L_CLONE);
        newpix = pixaGetPix(newpixa, i, L_CLONE);
        pixEqual(pix, newpix, &same);
        if (!same) {
            iofail = TRUE;
            lept_stderr("pix on page %d are unequal!\n", i);
        }
        pixDestroy(&pix);
        pixDestroy(&newpix);

    }
    if (iofail)
        lept_stderr("read/write for jbdata fails\n");
    else
        lept_stderr("read/write for jbdata succeeds\n");
    jbDataDestroy(&newdata);
    pixaDestroy(&newpixa);
    }
  #endif  /* DEBUG_TEST_DATA_IO */

  #if  RENDER_DEBUG
        /* Use debugflag == TRUE to see outlines of each component. */
    pixadb = jbDataRender(data, TRUE);
        /* Write the debug pages out */
    npages = pixaGetCount(pixadb);
    for (i = 0; i < npages; i++) {
        pix = pixaGetPix(pixadb, i, L_CLONE);
        snprintf(filename, BUF_SIZE, "%s.db.%04d", rootname, i);
        lept_stderr("filename: %s\n", filename);
        pixWrite(filename, pix, IFF_PNG);
        pixDestroy(&pix);
    }
    pixaDestroy(&pixadb);
  #endif  /* RENDER_DEBUG */

  #if  DISPLAY_ALL_INSTANCES
        /* Display all instances, organized by template.
         * The display programs have a lot of trouble with these. */
    pix = pixaaDisplayByPixa(classer->pixaa, 5, 1.0, 10, 0, 0);
    pixWrite("/tmp/lept/jb/output_instances", pix, IFF_PNG);
    pixDestroy(&pix);
  #endif  /* DISPLAY_ALL_INSTANCES */

    pixaDestroy(&pixa);
    sarrayDestroy(&safiles);
    jbClasserDestroy(&classer);
    jbDataDestroy(&data);

    /*--------------------------------------------------------------*/

#endif  /* Choose library function or detailed steps */

    return 0;
}
