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
 * mtiff_reg.c
 *
 *   Tests tiff I/O for:
 *
 *       - multipage tiff read/write
 *       - writing special tiff tags to file [not tested here]
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"
#include <string.h>

static const char *weasel_rev = "/tmp/lept/tiff/weasel_rev.tif";
static const char *weasel_rev_rev = "/tmp/lept/tiff/weasel_rev_rev.tif";
static const char *weasel_orig = "/tmp/lept/tiff/weasel_orig.tif";

int main(int    argc,
         char **argv)
{
l_uint8      *data;
char         *fname, *filename;
const char   *str;
char          buf[512];
l_int32       i, n, npages, equal, success;
size_t        offset, size;
FILE         *fp;
PIX          *pix1, *pix2;
PIXA         *pixa, *pixa1, *pixa2, *pixa3;
SARRAY       *sa;
L_REGPARAMS  *rp;

#if !defined(HAVE_LIBPNG)
    L_ERROR("This test requires libpng to run.\n", "mtiff_reg");
    exit(77);
#endif

   if (regTestSetup(argc, argv, &rp))
        return 1;

    lept_mkdir("lept/tiff");

    /* ----------------------  Test multipage I/O  -----------------------*/
        /* This puts every image file in the directory with a string
         * match to "weasel8" into a multipage tiff file.
         * Images with 1 bpp are coded as g4; the others as zip.
         * It then reads back into a pix and displays.  */
    writeMultipageTiff(".", "weasel8.", "/tmp/lept/tiff/weasel8.tif");
    regTestCheckFile(rp, "/tmp/lept/tiff/weasel8.tif");  /* 0 */
    pixa = pixaReadMultipageTiff("/tmp/lept/tiff/weasel8.tif");
    pix1 = pixaDisplayTiledInRows(pixa, 1, 1200, 0.5, 0, 15, 4);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 1 */
    pixDisplayWithTitle(pix1, 0, 0, NULL, rp->display);
    pixDestroy(&pix1);
    pix1 = pixaDisplayTiledInRows(pixa, 8, 1200, 0.8, 0, 15, 4);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 2 */
    pixDisplayWithTitle(pix1, 0, 200, NULL, rp->display);
    pixDestroy(&pix1);
    pix1 = pixaDisplayTiledInRows(pixa, 32, 1200, 1.2, 0, 15, 4);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 3 */
    pixDisplayWithTitle(pix1, 0, 400, NULL, rp->display);
    pixDestroy(&pix1);
    pixaDestroy(&pixa);

        /* This uses the offset method for linearizing overhead of
         * reading from a multi-image tiff file. */
    offset = 0;
    n = 0;
    pixa = pixaCreate(8);
    do {
        pix1 = pixReadFromMultipageTiff("/tmp/lept/tiff/weasel8.tif", &offset);
        if (!pix1) continue;
        pixaAddPix(pixa, pix1, L_INSERT);
        if (rp->display)
             lept_stderr("offset = %ld\n", (unsigned long)offset);
        n++;
    } while (offset != 0);
    if (rp->display) lept_stderr("Num images = %d\n", n);
    pix1 = pixaDisplayTiledInRows(pixa, 32, 1200, 1.2, 0, 15, 4);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 4 */
    pixDisplayWithTitle(pix1, 0, 600, NULL, rp->display);
    pixDestroy(&pix1);
    pixaDestroy(&pixa);

        /* This uses the offset method for linearizing overhead of
         * reading from a multi-image tiff file in memory. */
    offset = 0;
    n = 0;
    pixa = pixaCreate(8);
    data = l_binaryRead("/tmp/lept/tiff/weasel8.tif", &size);
    do {
        pix1 = pixReadMemFromMultipageTiff(data, size, &offset);
        if (!pix1) continue;
        pixaAddPix(pixa, pix1, L_INSERT);
        if (rp->display)
            lept_stderr("offset = %ld\n", (unsigned long)offset);
        n++;
    } while (offset != 0);
    if (rp->display) lept_stderr("Num images = %d\n", n);
    pix1 = pixaDisplayTiledInRows(pixa, 32, 1200, 1.2, 0, 15, 4);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 5 */
    pixDisplayWithTitle(pix1, 0, 800, NULL, rp->display);
    pixDestroy(&pix1);
    pixaDestroy(&pixa);
    lept_free(data);
    regTestCompareFiles(rp, 3, 4);  /* 6 */
    regTestCompareFiles(rp, 3, 5);  /* 7 */

        /* This makes a 1000 image tiff file and gives timing
         * for writing and reading.  Reading uses both the offset method
         * for returning individual pix and atomic pixaReadMultipageTiff()
         * method for returning a pixa of all the images.  Reading time
         * is linear in the number of images, but the writing time is
         *  quadratic, and the actual wall clock time is significantly
         *  more than the printed value. */
    pix1 = pixRead("char.tif");
    startTimer();
    pixWriteTiff("/tmp/lept/tiff/junkm.tif", pix1, IFF_TIFF_G4, "w");
    for (i = 1; i < 1000; i++) {
        pixWriteTiff("/tmp/lept/tiff/junkm.tif", pix1, IFF_TIFF_G4, "a");
    }
    regTestCheckFile(rp, "/tmp/lept/tiff/junkm.tif");  /* 8 */
    pixDestroy(&pix1);
    if (rp->display) {
        lept_stderr("\n1000 image file: /tmp/lept/tiff/junkm.tif\n");
        lept_stderr("Time to write 1000 images: %7.3f sec\n", stopTimer());
    }

    startTimer();
    offset = 0;
    n = 0;
    do {
        pix1 = pixReadFromMultipageTiff("/tmp/lept/tiff/junkm.tif", &offset);
        if (!pix1) continue;
        if (rp->display && (n % 100 == 0))
            lept_stderr("offset = %ld\n", (unsigned long)offset);
        pixDestroy(&pix1);
        n++;
    } while (offset != 0);
    regTestCompareValues(rp, 1000, n, 0);  /* 9 */
    if (rp->display)
        lept_stderr("Time to read %d images: %6.3f sec\n", n, stopTimer());

    startTimer();
    pixa = pixaReadMultipageTiff("/tmp/lept/tiff/junkm.tif");
    lept_stderr("Time to read %d images and return a pixa: %6.3f sec\n",
            pixaGetCount(pixa), stopTimer());
    pix1 = pixaDisplayTiledInRows(pixa, 8, 1500, 0.8, 0, 15, 4);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 10 */
    pixDestroy(&pix1);
    pixaDestroy(&pixa);

        /* This does the following sequence of operations:
         * (1) makes pixa1 and writes a multipage tiff file from it
         * (2) reads that file into memory
         * (3) generates pixa2 from the data in memory
         * (4) tiff compresses pixa2 back to memory
         * (5) generates pixa3 by uncompressing the memory data
         * (6) compares pixa3 with pixa1   */
    pix1 = pixRead("weasel8.240c.png");  /* (1) */
    pixa1 = pixaCreate(10);
    for (i = 0; i < 10; i++)
        pixaAddPix(pixa1, pix1, L_COPY);
    pixDestroy(&pix1);
    pixaWriteMultipageTiff("/tmp/lept/tiff/junkm2.tif", pixa1);
    regTestCheckFile(rp, "/tmp/lept/tiff/junkm2.tif");  /* 11 */
    data = l_binaryRead("/tmp/lept/tiff/junkm2.tif", &size);  /* (2) */
    pixa2 = pixaCreate(10);  /* (3) */
    offset = 0;
    n = 0;
    do {
        pix1 = pixReadMemFromMultipageTiff(data, size, &offset);
        pixaAddPix(pixa2, pix1, L_INSERT);
        n++;
    } while (offset != 0);
    regTestCompareValues(rp, 10, n, 0);  /* 12 */
    if (rp->display) lept_stderr("\nRead %d images\n", n);
    lept_free(data);
    pixaWriteMemMultipageTiff(&data, &size, pixa2);  /* (4) */
    pixa3 = pixaReadMemMultipageTiff(data, size);  /* (5) */
    pix1 = pixaDisplayTiledInRows(pixa3, 8, 1500, 0.8, 0, 15, 4);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 13 */
    pixDestroy(&pix1);
    n = pixaGetCount(pixa3);
    if (rp->display) lept_stderr("Write/read %d images\n", n);
    success = TRUE;
    for (i = 0; i < n; i++) {
        pix1 = pixaGetPix(pixa1, i, L_CLONE);
        pix2 = pixaGetPix(pixa3, i, L_CLONE);
        pixEqual(pix1, pix2, &equal);
        if (!equal) success = FALSE;
        pixDestroy(&pix1);
        pixDestroy(&pix2);
    }
    regTestCompareValues(rp, TRUE, success, 0);  /* 14 */
    pixaDestroy(&pixa1);
    pixaDestroy(&pixa2);
    pixaDestroy(&pixa3);
    lept_free(data);

    /* ------------------ Test single-to-multipage I/O  -------------------*/
        /* Read the files and generate a multipage tiff file of G4 images.
         * Then convert that to a G4 compressed and ascii85 encoded PS file. */
    sa = getSortedPathnamesInDirectory(".", "weasel4.", 0, 4);
    if (rp->display) sarrayWriteStderr(sa);
    sarraySort(sa, sa, L_SORT_INCREASING);
    if (rp->display) sarrayWriteStderr(sa);
    npages = sarrayGetCount(sa);
    for (i = 0; i < npages; i++) {
        fname = sarrayGetString(sa, i, L_NOCOPY);
        filename = genPathname(".", fname);
        pix1 = pixRead(filename);
        if (!pix1) continue;
        pix2 = pixConvertTo1(pix1, 128);
        if (i == 0)
            pixWriteTiff("/tmp/lept/tiff/weasel4", pix2, IFF_TIFF_G4, "w+");
        else
            pixWriteTiff("/tmp/lept/tiff/weasel4", pix2, IFF_TIFF_G4, "a");
        pixDestroy(&pix1);
        pixDestroy(&pix2);
        lept_free(filename);
    }
    regTestCheckFile(rp, "/tmp/lept/tiff/junkm2.tif");  /* 15 */

        /* Write it out as a PS file */
    lept_stderr("Writing to: /tmp/lept/tiff/weasel4.ps\n");
    convertTiffMultipageToPS("/tmp/lept/tiff/weasel4",
                             "/tmp/lept/tiff/weasel4.ps", 0.95);
    regTestCheckFile(rp, "/tmp/lept/tiff/weasel4.ps");  /* 16 */

        /* Write it out as a pdf file */
    lept_stderr("Writing to: /tmp/lept/tiff/weasel4.pdf\n");
    l_pdfSetDateAndVersion(FALSE);
    convertTiffMultipageToPdf("/tmp/lept/tiff/weasel4",
                              "/tmp/lept/tiff/weasel4.pdf");
    regTestCheckFile(rp, "/tmp/lept/tiff/weasel4.pdf");  /* 17 */
    sarrayDestroy(&sa);

    /* ------------------  Test multipage I/O  -------------------*/
        /* Read count of pages in tiff multipage  file */
    writeMultipageTiff(".", "weasel2", weasel_orig);
    regTestCheckFile(rp, weasel_orig);  /* 18 */
    fp = lept_fopen(weasel_orig, "rb");
    success = fileFormatIsTiff(fp);
    regTestCompareValues(rp, TRUE, success, 0);  /* 19 */
    if (success) {
        tiffGetCount(fp, &npages);
        regTestCompareValues(rp, 5, npages, 0);  /* 20 */
        lept_stderr(" Tiff: %d page\n", npages);
    }
    lept_fclose(fp);

        /* Split into separate page files */
    for (i = 0; i < npages + 1; i++) {   /* read one beyond to catch error */
        pix1 = pixReadTiff(weasel_orig, i);
        if (!pix1) continue;
        snprintf(buf, sizeof(buf), "/tmp/lept/tiff/%03d.tif", i);
        pixWrite(buf, pix1, IFF_TIFF_ZIP);
        pixDestroy(&pix1);
    }

        /* Read separate page files and write reversed file */
    for (i = npages - 1; i >= 0; i--) {
        snprintf(buf, sizeof(buf), "/tmp/lept/tiff/%03d.tif", i);
        pix1 = pixRead(buf);
        if (!pix1) continue;
        if (i == npages - 1)
            pixWriteTiff(weasel_rev, pix1, IFF_TIFF_ZIP, "w+");
        else
            pixWriteTiff(weasel_rev, pix1, IFF_TIFF_ZIP, "a");
        pixDestroy(&pix1);
    }
    regTestCheckFile(rp, weasel_rev);  /* 21 */

        /* Read reversed file and reverse again */
    pixa = pixaCreate(npages);
    for (i = 0; i < npages; i++) {
        pix1 = pixReadTiff(weasel_rev, i);
        pixaAddPix(pixa, pix1, L_INSERT);
    }
    for (i = npages - 1; i >= 0; i--) {
        pix1 = pixaGetPix(pixa, i, L_CLONE);
        if (i == npages - 1)
            pixWriteTiff(weasel_rev_rev, pix1, IFF_TIFF_ZIP, "w+");
        else
            pixWriteTiff(weasel_rev_rev, pix1, IFF_TIFF_ZIP, "a");
        pixDestroy(&pix1);
    }
    regTestCheckFile(rp, weasel_rev_rev);  /* 22 */
    regTestCompareFiles(rp, 18, 22);  /* 23 */
    pixaDestroy(&pixa);


#if 1    /* -----   test adding custom public tags to a tiff header ----- */
{
    size_t        length;
    NUMA         *naflags, *nasizes;
    SARRAY       *savals, *satypes;

    pix1 = pixRead("feyn.tif");
    naflags = numaCreate(10);
    savals = sarrayCreate(10);
    satypes = sarrayCreate(10);
    nasizes = numaCreate(10);

/*    numaAddNumber(naflags, TIFFTAG_XMLPACKET);  */ /* XMP:  700 */
    numaAddNumber(naflags, 700);
    str = "<xmp>This is a Fake XMP packet</xmp>\n<text>Guess what ...?</text>";
    length = strlen(str);
    sarrayAddString(savals, str, L_COPY);
    sarrayAddString(satypes, "char*", L_COPY);
    numaAddNumber(nasizes, length);  /* get it all */

    numaAddNumber(naflags, 269);  /* DOCUMENTNAME */
    sarrayAddString(savals, "One silly title", L_COPY);
    sarrayAddString(satypes, "const char*", L_COPY);
    numaAddNumber(naflags, 270);  /* IMAGEDESCRIPTION */
    sarrayAddString(savals, "One page of text", L_COPY);
    sarrayAddString(satypes, "const char*", L_COPY);
        /* the max sample is used by rendering programs
         * to scale the dynamic range */
    numaAddNumber(naflags, 281);  /* MAXSAMPLEVALUE */
    sarrayAddString(savals, "4", L_COPY);
    sarrayAddString(satypes, "l_uint16", L_COPY);
        /* note that date is required to be a 20 byte string */
    numaAddNumber(naflags, 306);  /* DATETIME */
    sarrayAddString(savals, "2004:10:11 09:35:15", L_COPY);
    sarrayAddString(satypes, "const char*", L_COPY);
        /* note that page number requires 2 l_uint16 input */
    numaAddNumber(naflags, 297);  /* PAGENUMBER */
    sarrayAddString(savals, "1-412", L_COPY);
    sarrayAddString(satypes, "l_uint16-l_uint16", L_COPY);
    pixWriteTiffCustom("/tmp/lept/tiff/tags.tif", pix1, IFF_TIFF_G4, "w", naflags,
                       savals, satypes, nasizes);
    fprintTiffInfo(stderr, "/tmp/lept/tiff/tags.tif");
    lept_stderr("num flags = %d\n", numaGetCount(naflags));
    lept_stderr("num sizes = %d\n", numaGetCount(nasizes));
    lept_stderr("num vals = %d\n", sarrayGetCount(savals));
    lept_stderr("num types = %d\n", sarrayGetCount(satypes));
    numaDestroy(&naflags);
    numaDestroy(&nasizes);
    sarrayDestroy(&savals);
    sarrayDestroy(&satypes);
    pixDestroy(&pix1);
}
#endif

    return regTestCleanup(rp);
}
