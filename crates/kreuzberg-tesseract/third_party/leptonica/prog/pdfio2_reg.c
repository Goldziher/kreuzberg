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
 *  pdfio2_reg.c
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include <string.h>
#include "allheaders.h"

static void GetImageMask(PIX *pixs, l_int32 res, BOXA **pboxa,
                         L_REGPARAMS *rp, const char *debugfile);
static PIX * QuantizeNonImageRegion(PIX *pixs, PIX *pixm, l_int32 levels);


int main(int    argc,
         char **argv)
{
l_uint8      *data;
l_int32       w, h, same;
size_t        nbytes;
BOXA         *boxa1, *boxa2;
L_BYTEA      *ba;
PIX          *pix1, *pix2, *pix3, *pix4, *pix5, *pix6;
L_REGPARAMS  *rp;

#if !defined(HAVE_LIBPNG)
    L_ERROR("This test requires libpng to run.\n", "pdfio2_reg");
    exit(77);
#endif
#if !defined(HAVE_LIBJPEG)
    L_ERROR("This test requires libjpeg to run.\n", "pdfio2_reg");
    exit(77);
#endif
#if !defined(HAVE_LIBTIFF)
    L_ERROR("This test requires libtiff to run.\n", "pdfio2_reg");
    exit(77);
#endif

    if (regTestSetup(argc, argv, &rp))
        return 1;

    l_pdfSetDateAndVersion(0);
    lept_mkdir("lept/pdf2");

    /* ---------- pdf convert segmented with image regions ---------- */
    lept_stderr("\n*** Writing segmented images with image regions\n");
    startTimer();

        /* Get the image region(s) for rabi.png.  There are two
         * small bogus regions at the top, but we'll keep them for
         * the demonstration. */
    pix1 = pixRead("rabi.png");
    pix2 = pixScaleToGray2(pix1);
    pixWrite("/tmp/lept/pdf2/rabi8.jpg", pix2, IFF_JFIF_JPEG);
    pix3 = pixThresholdTo4bpp(pix2, 16, 1);
    pixWrite("/tmp/lept/pdf2/rabi4.png", pix3, IFF_PNG);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixSetResolution(pix1, 300, 300);
    pixGetDimensions(pix1, &w, &h, NULL);
    pix2 = pixGenerateHalftoneMask(pix1, NULL, NULL, NULL);
    pix3 = pixMorphSequence(pix2, "c20.1 + c1.20", 0);
    boxa1 = pixConnComp(pix3, NULL, 8);
    boxa2 = boxaTransform(boxa1, 0, 0, 0.5, 0.5);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);

        /* 1 bpp input */
    convertToPdfSegmented("rabi.png", 300, L_G4_ENCODE, 128, boxa1,
                          0, 0.25, NULL, "/tmp/lept/pdf2/file00.pdf");
    convertToPdfSegmented("rabi.png", 300, L_JPEG_ENCODE, 128, boxa1,
                          0, 0.25, NULL, "/tmp/lept/pdf2/file01.pdf");
    convertToPdfSegmented("rabi.png", 300, L_FLATE_ENCODE, 128, boxa1,
                          0, 0.25, NULL, "/tmp/lept/pdf2/file02.pdf");

        /* 8 bpp input, no cmap */
    convertToPdfSegmented("/tmp/lept/pdf2/rabi8.jpg", 150, L_G4_ENCODE, 128,
                          boxa2, 0, 0.5, NULL, "/tmp/lept/pdf2/file03.pdf");
    convertToPdfSegmented("/tmp/lept/pdf2/rabi8.jpg", 150, L_JPEG_ENCODE, 128,
                          boxa2, 0, 0.5, NULL, "/tmp/lept/pdf2/file04.pdf");
    convertToPdfSegmented("/tmp/lept/pdf2/rabi8.jpg", 150, L_FLATE_ENCODE, 128,
                          boxa2, 0, 0.5, NULL, "/tmp/lept/pdf2/file05.pdf");

        /* 4 bpp input, cmap */
    convertToPdfSegmented("/tmp/lept/pdf2/rabi4.png", 150, L_G4_ENCODE, 128,
                          boxa2, 0, 0.5, NULL, "/tmp/lept/pdf2/file06.pdf");
    convertToPdfSegmented("/tmp/lept/pdf2/rabi4.png", 150, L_JPEG_ENCODE, 128,
                          boxa2, 0, 0.5, NULL, "/tmp/lept/pdf2/file07.pdf");
    convertToPdfSegmented("/tmp/lept/pdf2/rabi4.png", 150, L_FLATE_ENCODE, 128,
                          boxa2, 0, 0.5, NULL, "/tmp/lept/pdf2/file08.pdf");

        /* 4 bpp input, cmap, data output */
    data = NULL;
    convertToPdfDataSegmented("/tmp/lept/pdf2/rabi4.png", 150, L_G4_ENCODE,
                              128, boxa2, 0, 0.5, NULL, &data, &nbytes);
    l_binaryWrite("/tmp/lept/pdf2/file09.pdf", "w", data, nbytes);
    lept_free(data);
    convertToPdfDataSegmented("/tmp/lept/pdf2/rabi4.png", 150, L_JPEG_ENCODE,
                              128, boxa2, 0, 0.5, NULL, &data, &nbytes);
    l_binaryWrite("/tmp/lept/pdf2/file10.pdf", "w", data, nbytes);
    lept_free(data);
    convertToPdfDataSegmented("/tmp/lept/pdf2/rabi4.png", 150, L_FLATE_ENCODE,
                              128, boxa2, 0, 0.5, NULL, &data, &nbytes);
    l_binaryWrite("/tmp/lept/pdf2/file11.pdf", "w", data, nbytes);
    lept_free(data);
    lept_stderr("Segmented images time: %7.3f\n", stopTimer());

    boxaDestroy(&boxa1);
    boxaDestroy(&boxa2);

#if 1
    /* -------- pdf convert segmented from color image -------- */
    lept_stderr("\n*** Writing color segmented images\n");
    startTimer();

    pix1 = pixRead("candelabrum.011.jpg");
    pix2 = pixScale(pix1, 3.0, 3.0);
    pixWrite("/tmp/lept/pdf2/candelabrum3.jpg", pix2, IFF_JFIF_JPEG);
    GetImageMask(pix2, 200, &boxa1, rp, "/tmp/lept/pdf2/seg1.jpg");
    convertToPdfSegmented("/tmp/lept/pdf2/candelabrum3.jpg", 200, L_G4_ENCODE,
                          100, boxa1, 0, 0.25, NULL,
                          "/tmp/lept/pdf2/file12.pdf");
    convertToPdfSegmented("/tmp/lept/pdf2/candelabrum3.jpg", 200, L_JPEG_ENCODE,
                          100, boxa1, 0, 0.25, NULL,
                          "/tmp/lept/pdf2/file13.pdf");
    convertToPdfSegmented("/tmp/lept/pdf2/candelabrum3.jpg", 200, L_FLATE_ENCODE,
                          100, boxa1, 0, 0.25, NULL,
                          "/tmp/lept/pdf2/file14.pdf");

    pixDestroy(&pix1);
    pixDestroy(&pix2);
    boxaDestroy(&boxa1);

    pix1 = pixRead("lion-page.00016.jpg");
    pix2 = pixScale(pix1, 3.0, 3.0);
    pixWrite("/tmp/lept/pdf2/lion16.jpg", pix2, IFF_JFIF_JPEG);
    pix3 = pixRead("lion-mask.00016.tif");
    boxa1 = pixConnComp(pix3, NULL, 8);
    boxa2 = boxaTransform(boxa1, 0, 0, 3.0, 3.0);
    convertToPdfSegmented("/tmp/lept/pdf2/lion16.jpg", 200, L_G4_ENCODE,
                          190, boxa2, 0, 0.5, NULL, "/tmp/lept/pdf2/file15.pdf");
    convertToPdfSegmented("/tmp/lept/pdf2/lion16.jpg", 200, L_JPEG_ENCODE,
                          190, boxa2, 0, 0.5, NULL, "/tmp/lept/pdf2/file16.pdf");
    convertToPdfSegmented("/tmp/lept/pdf2/lion16.jpg", 200, L_FLATE_ENCODE,
                          190, boxa2, 0, 0.5, NULL, "/tmp/lept/pdf2/file17.pdf");

        /* Quantize the non-image part and flate encode.
         * This is useful because it results in a smaller file than
         * when you flate-encode the un-quantized non-image regions. */
    pix4 = pixScale(pix3, 3.0, 3.0);  /* higher res mask, for combining */
    pix5 = QuantizeNonImageRegion(pix2, pix4, 12);
    pixWrite("/tmp/lept/pdf2/lion16-quant.png", pix5, IFF_PNG);
    convertToPdfSegmented("/tmp/lept/pdf2/lion16-quant.png", 200, L_FLATE_ENCODE,
                          190, boxa2, 0, 0.5, NULL, "/tmp/lept/pdf2/file18.pdf");
    lept_stderr("Color segmented images time: %7.3f\n", stopTimer());

    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);
    pixDestroy(&pix5);
    boxaDestroy(&boxa1);
    boxaDestroy(&boxa2);
#endif

#if 1
    /* -- Test simple interface for generating multi-page pdf from images -- */
    lept_stderr("\n*** Writing multipage pdfs from images");
    startTimer();

        /* Put four image files in a directory.  They will be encoded thus:
         *     file1.png:  flate (8 bpp, only 10 colors)
         *     file2.jpg:  dct (8 bpp, 256 colors because of the jpeg encoding)
         *     file3.tif:  g4 (1 bpp)
         *     file4.jpg:  dct (32 bpp)    */
    lept_mkdir("lept/image");
    pix1 = pixRead("feyn.tif");
    pix2 = pixRead("rabi.png");
    pix3 = pixScaleToGray3(pix1);
    pix4 = pixScaleToGray3(pix2);
    pix5 = pixScale(pix1, 0.33, 0.33);
    pix6 = pixRead("test24.jpg");
    pixWrite("/tmp/lept/image/file1.png", pix3, IFF_PNG);  /* 10 colors */
    pixWrite("/tmp/lept/image/file2.jpg", pix4, IFF_JFIF_JPEG); /* 256 colors */
    pixWrite("/tmp/lept/image/file3.tif", pix5, IFF_TIFF_G4);
    pixWrite("/tmp/lept/image/file4.jpg", pix6, IFF_JFIF_JPEG);

    startTimer();
    convertFilesToPdf("/tmp/lept/image", "file", 100, 0.8, 0, 75, "4 file test",
                      "/tmp/lept/pdf2/file19.pdf");
    lept_stderr("4-page pdf generated: /tmp/lept/pdf2/file19.pdf\n"
                "Multi-page gen time: %7.3f\n", stopTimer());
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);
    pixDestroy(&pix5);
    pixDestroy(&pix6);
#endif

    regTestCheckFile(rp, "/tmp/lept/pdf2/file00.pdf");
    regTestCheckFile(rp, "/tmp/lept/pdf2/file01.pdf");
    regTestCheckFile(rp, "/tmp/lept/pdf2/file02.pdf");
    regTestCheckFile(rp, "/tmp/lept/pdf2/file03.pdf");
    regTestCheckFile(rp, "/tmp/lept/pdf2/file04.pdf");
    regTestCheckFile(rp, "/tmp/lept/pdf2/file05.pdf");
    regTestCheckFile(rp, "/tmp/lept/pdf2/file06.pdf");
    regTestCheckFile(rp, "/tmp/lept/pdf2/file07.pdf");
    regTestCheckFile(rp, "/tmp/lept/pdf2/file08.pdf");
    regTestCheckFile(rp, "/tmp/lept/pdf2/file09.pdf");
    regTestCheckFile(rp, "/tmp/lept/pdf2/file10.pdf");
    regTestCheckFile(rp, "/tmp/lept/pdf2/file11.pdf");
    regTestCheckFile(rp, "/tmp/lept/pdf2/file12.pdf");
    regTestCheckFile(rp, "/tmp/lept/pdf2/file13.pdf");
    regTestCheckFile(rp, "/tmp/lept/pdf2/file14.pdf");
    regTestCheckFile(rp, "/tmp/lept/pdf2/file15.pdf");
    regTestCheckFile(rp, "/tmp/lept/pdf2/file16.pdf");
    regTestCheckFile(rp, "/tmp/lept/pdf2/file17.pdf");
    regTestCheckFile(rp, "/tmp/lept/pdf2/file18.pdf");
    regTestCheckFile(rp, "/tmp/lept/pdf2/file19.pdf");

#if 1
    /* ------------------ Test multipage pdf generation ----------------- */
    lept_stderr("\n*** Writing multipage pdfs from single page pdfs\n");

        /* Generate a multi-page pdf from all these files */
    startTimer();
    concatenatePdf("/tmp/lept/pdf2", "file", "/tmp/lept/pdf2/cat_lept.pdf");
    lept_stderr("All files are concatenated: /tmp/lept/pdf2/cat_lept.pdf\n"
                "Concatenation time: %7.3f\n", stopTimer());
#endif

#if 1
    /* ----------- Test corruption recovery by concatenation ------------ */
        /* Put two good pdf files in a directory */
    startTimer();
    lept_rmdir("lept/good");
    lept_mkdir("lept/good");
    lept_cp("testfile1.pdf", "lept/good", NULL, NULL);
    lept_cp("testfile2.pdf", "lept/good", NULL, NULL);
    concatenatePdf("/tmp/lept/good", "file", "/tmp/lept/pdf2/good.pdf");

        /* Make a bad version with the pdf id removed, so that it is not
         * recognized as a pdf */
    lept_rmdir("lept/bad");
    lept_mkdir("lept/bad");
    ba = l_byteaInitFromFile("testfile2.pdf");
    data = l_byteaGetData(ba, &nbytes);
    l_binaryWrite("/tmp/lept/bad/testfile0.notpdf.pdf", "w",
                  data + 10, nbytes - 10);

        /* Make a version with a corrupted trailer */
    if (data)
        data[2297] = '2';  /* munge trailer object 6: change 458 --> 428 */
    l_binaryWrite("/tmp/lept/bad/testfile2.bad.pdf", "w", data, nbytes);
    l_byteaDestroy(&ba);

        /* Copy testfile1.pdf to the /tmp/lept/bad directory.  Then
         * run concat on the bad files.  The "not pdf" file should be
         * ignored, and the corrupted pdf file should be properly parsed,
         * so the resulting concatenated pdf files should be identical.  */
    lept_stderr("\nWe attempt to build from a bad directory\n");
    lept_stderr("******************************************************\n");
    lept_stderr("* The next 3 error messages are intentional          *\n");
    lept_cp("testfile1.pdf", "lept/bad", NULL, NULL);
    concatenatePdf("/tmp/lept/bad", "file", "/tmp/lept/pdf2/bad.pdf");
    lept_stderr("******************************************************\n");
    filesAreIdentical("/tmp/lept/pdf2/good.pdf", "/tmp/lept/pdf2/bad.pdf",
                      &same);
    if (same)
        lept_stderr("Fixed: files are the same\nAttempt succeeded\n");
    else
        lept_stderr("Busted: files are different\n");
    lept_stderr("Corruption recovery time: %7.3f\n", stopTimer());
#endif

#if 0
{
    char     buffer[512];
    char    *tempfile1, *tempfile2;
    l_int32  ret;

    lept_stderr("\n*** pdftk writes multipage pdfs from images\n");
    tempfile1 = genPathname("/tmp/lept/pdf2", "file*.pdf");
    tempfile2 = genPathname("/tmp/lept/pdf2", "cat_pdftk.pdf");
    snprintf(buffer, sizeof(buffer), "pdftk %s output %s",
             tempfile1, tempfile2);
    ret = callSystemDebug(buffer);  /* pdftk */
    lept_free(tempfile1);
    lept_free(tempfile2);
}
#endif

    return regTestCleanup(rp);
}


static void
GetImageMask(PIX          *pixs,
             l_int32       res,
             BOXA        **pboxa,
             L_REGPARAMS  *rp,
             const char   *debugfile)
{
PIX   *pix1, *pix2, *pix3, *pix4;
PIXA  *pixa;

    pixSetResolution(pixs, 200, 200);
    pix1 = pixConvertTo1(pixs, 100);
    pix2 = pixGenerateHalftoneMask(pix1, NULL, NULL, NULL);
    pix3 = pixMorphSequence(pix2, "c20.1 + c1.20", 0);
    *pboxa = pixConnComp(pix3, NULL, 8);
    if (debugfile) {
        pixa = pixaCreate(0);
        pixaAddPix(pixa, pixs, L_COPY);
        pixaAddPix(pixa, pix1, L_INSERT);
        pixaAddPix(pixa, pix2, L_INSERT);
        pixaAddPix(pixa, pix3, L_INSERT);
        pix4 = pixaDisplayTiledInRows(pixa, 32, 1800, 0.25, 0, 25, 2);
        pixWrite(debugfile, pix4, IFF_JFIF_JPEG);
        pixDisplayWithTitle(pix4, 100, 100, NULL, rp->display);
        pixDestroy(&pix4);
        pixaDestroy(&pixa);
    } else {
        pixDestroy(&pix1);
        pixDestroy(&pix2);
        pixDestroy(&pix3);
    }

    return;
}

static PIX *
QuantizeNonImageRegion(PIX     *pixs,
                       PIX     *pixm,
                       l_int32  levels)
{
PIX  *pix1, *pix2, *pixd;

    pix1 = pixConvertTo8(pixs, 0);
    pix2 = pixThresholdOn8bpp(pix1, levels, 1);
    pixd = pixConvertTo32(pix2);  /* save in rgb */
    pixCombineMasked(pixd, pixs, pixm);  /* rgb result */
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    return pixd;
}
