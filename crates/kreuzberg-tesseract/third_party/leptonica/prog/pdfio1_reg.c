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
 *  pdfio1_reg.c
 *
 *    Basic high-level interface tests
 *       Single images
 *       Multiple images
 *       Segmented images, with and without colormaps
 *       1 bpp images
 *
 *    Low-level interface tests for 1 bpp images
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include <string.h>
#include "allheaders.h"

int main(int    argc,
         char **argv)
{
l_uint8      *data8;
l_int32       i, j, seq;
size_t        nbytes;
const char   *title;
BOX          *box;
L_COMP_DATA  *cid;
L_PDF_DATA   *lpd;
PIX          *pix1, *pix2, *pix3;
PIX          *pixs, *pixt, *pixg, *pixgc, *pixc;
PIXCMAP      *cmap;
L_REGPARAMS  *rp;

#if !defined(HAVE_LIBPNG)
    L_ERROR("This test requires libpng to run.\n", "pdfio1_reg");
    exit(77);
#endif
#if !defined(HAVE_LIBJPEG)
    L_ERROR("This test requires libjpeg to run.\n", "pdfio1_reg");
    exit(77);
#endif
#if !defined(HAVE_LIBTIFF)
    L_ERROR("This test requires libtiff to run.\n", "pdfio1_reg");
    exit(77);
#endif

    if (regTestSetup(argc, argv, &rp))
        return 1;

    l_pdfSetDateAndVersion(0);
    lept_mkdir("lept/pdf1");

#if 1
    /* ---------------  Single image tests  ------------------- */
    lept_stderr("\n*** Writing single images as pdf files\n");
    convertToPdf("weasel2.4c.png", L_FLATE_ENCODE, 0,
                 "/tmp/lept/pdf1/file00.pdf",
                 0, 0, 72, "weasel2.4c.png", NULL, 0);
    convertToPdf("test24.jpg", L_JPEG_ENCODE, 0, "/tmp/lept/pdf1/file01.pdf",
                 0, 0, 72, "test24.jpg", NULL, 0);
    convertToPdf("feyn.tif", L_G4_ENCODE, 0, "/tmp/lept/pdf1/file02.pdf",
                 0, 0, 300, "feyn.tif", NULL, 0);

    pixs = pixRead("feyn.tif");
    pixConvertToPdf(pixs, L_G4_ENCODE, 0, "/tmp/lept/pdf1/file03.pdf", 0, 0, 300,
                    "feyn.tif", NULL, 0);
    pixDestroy(&pixs);

    pixs = pixRead("test24.jpg");
    pixConvertToPdf(pixs, L_JPEG_ENCODE, 5, "/tmp/lept/pdf1/file04.pdf",
                    0, 0, 72, "test24.jpg", NULL, 0);
    pixDestroy(&pixs);

    pixs = pixRead("feyn.tif");
    pixt = pixScaleToGray2(pixs);
    pixWrite("/tmp/lept/pdf1/feyn8.png", pixt, IFF_PNG);
    convertToPdf("/tmp/lept/pdf1/feyn8.png", L_JPEG_ENCODE, 0,
                 "/tmp/lept/pdf1/file05.pdf", 0, 0, 150, "feyn8.png", NULL, 0);
    pixDestroy(&pixs);
    pixDestroy(&pixt);

    convertToPdf("weasel4.16g.png", L_FLATE_ENCODE, 0,
                 "/tmp/lept/pdf1/file06.pdf", 0, 0, 30,
                 "weasel4.16g.png", NULL, 0);

    pixs = pixRead("test24.jpg");
    pixg = pixConvertTo8(pixs, 0);
    box = boxCreate(100, 100, 100, 100);
    pixc = pixClipRectangle(pixs, box, NULL);
    pixgc = pixClipRectangle(pixg, box, NULL);
    pixWrite("/tmp/lept/pdf1/pix32.jpg", pixc, IFF_JFIF_JPEG);
    pixWrite("/tmp/lept/pdf1/pix8.jpg", pixgc, IFF_JFIF_JPEG);
    convertToPdf("/tmp/lept/pdf1/pix32.jpg", L_FLATE_ENCODE, 0,
                 "/tmp/lept/pdf1/file07.pdf", 0, 0, 72, "pix32.jpg", NULL, 0);
    convertToPdf("/tmp/lept/pdf1/pix8.jpg", L_FLATE_ENCODE, 0,
                 "/tmp/lept/pdf1/file08.pdf", 0, 0, 72, "pix8.jpg", NULL, 0);
    pixDestroy(&pixs);
    pixDestroy(&pixg);
    pixDestroy(&pixc);
    pixDestroy(&pixgc);
    boxDestroy(&box);
#endif


#if 1
    /* ---------------  Multiple image tests  ------------------- */
    lept_stderr("\n*** Writing multiple images as single page pdf files\n");
    pix1 = pixRead("feyn-fract.tif");
    pix2 = pixRead("weasel8.240c.png");

        /* First, write the 1 bpp image through the mask onto the weasels */
    for (i = 0; i < 5; i++) {
        for (j = 0; j < 10; j++) {
            seq = (i == 0 && j == 0) ? L_FIRST_IMAGE : L_NEXT_IMAGE;
            title = (i == 0 && j == 0) ? "feyn-fract.tif" : NULL;
            pixConvertToPdf(pix2, L_FLATE_ENCODE, 0, NULL, 100 * j,
                            100 * i, 70, title, &lpd, seq);
        }
    }
    pixConvertToPdf(pix1, L_G4_ENCODE, 0, "/tmp/lept/pdf1/file09.pdf", 0, 0, 80,
                    NULL, &lpd, L_LAST_IMAGE);

        /* Now, write the 1 bpp image over the weasels */
    l_pdfSetG4ImageMask(0);
    for (i = 0; i < 5; i++) {
        for (j = 0; j < 10; j++) {
            seq = (i == 0 && j == 0) ? L_FIRST_IMAGE : L_NEXT_IMAGE;
            title = (i == 0 && j == 0) ? "feyn-fract.tif" : NULL;
            pixConvertToPdf(pix2, L_FLATE_ENCODE, 0, NULL, 100 * j,
                            100 * i, 70, title, &lpd, seq);
        }
    }
    pixConvertToPdf(pix1, L_G4_ENCODE, 0, "/tmp/lept/pdf1/file10.pdf", 0, 0, 80,
                    NULL, &lpd, L_LAST_IMAGE);
    l_pdfSetG4ImageMask(1);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
#endif

#if 1
    /* -------- pdf convert segmented with no image regions -------- */
    lept_stderr("\n*** Writing segmented images without image regions\n");
    pix1 = pixRead("rabi.png");
    pix2 = pixScaleToGray2(pix1);
    pixWrite("/tmp/lept/pdf1/rabi8.jpg", pix2, IFF_JFIF_JPEG);
    pix3 = pixThresholdTo4bpp(pix2, 16, 1);
    pixWrite("/tmp/lept/pdf1/rabi4.png", pix3, IFF_PNG);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);

        /* 1 bpp input */
    convertToPdfSegmented("rabi.png", 300, L_G4_ENCODE, 128, NULL, 0, 0,
                          NULL, "/tmp/lept/pdf1/file11.pdf");
    convertToPdfSegmented("rabi.png", 300, L_JPEG_ENCODE, 128, NULL, 0, 0,
                          NULL, "/tmp/lept/pdf1/file12.pdf");
    convertToPdfSegmented("rabi.png", 300, L_FLATE_ENCODE, 128, NULL, 0, 0,
                          NULL, "/tmp/lept/pdf1/file13.pdf");

        /* 8 bpp input, no cmap */
    convertToPdfSegmented("/tmp/lept/pdf1/rabi8.jpg", 150, L_G4_ENCODE, 128,
                          NULL, 0, 0, NULL, "/tmp/lept/pdf1/file14.pdf");
    convertToPdfSegmented("/tmp/lept/pdf1/rabi8.jpg", 150, L_JPEG_ENCODE, 128,
                          NULL, 0, 0, NULL, "/tmp/lept/pdf1/file15.pdf");
    convertToPdfSegmented("/tmp/lept/pdf1/rabi8.jpg", 150, L_FLATE_ENCODE, 128,
                          NULL, 0, 0, NULL, "/tmp/lept/pdf1/file16.pdf");

        /* 4 bpp input, cmap */
    convertToPdfSegmented("/tmp/lept/pdf1/rabi4.png", 150, L_G4_ENCODE, 128,
                          NULL, 0, 0, NULL, "/tmp/lept/pdf1/file17.pdf");
    convertToPdfSegmented("/tmp/lept/pdf1/rabi4.png", 150, L_JPEG_ENCODE, 128,
                          NULL, 0, 0, NULL, "/tmp/lept/pdf1/file18.pdf");
    convertToPdfSegmented("/tmp/lept/pdf1/rabi4.png", 150, L_FLATE_ENCODE, 128,
                          NULL, 0, 0, NULL, "/tmp/lept/pdf1/file19.pdf");

#endif

#if 1
    /* ----------  Generating from 1 bpp images (high-level) -------------- */
    lept_stderr("\n*** Writing 1 bpp images as pdf files (high-level)\n");
    pix1 = pixRead("feyn-fract.tif");
    pixWrite("/tmp/lept/pdf1/feyn-nocmap.png", pix1, IFF_PNG);
    pix2 = pixCopy(NULL, pix1);
    cmap = pixcmapCreate(1);
    pixcmapAddColor(cmap, 0, 0, 0);  /* with cmap: black bg, white letters */
    pixcmapAddColor(cmap, 255, 255, 255);
    pixSetColormap(pix2, cmap);
    pixWrite("/tmp/lept/pdf1/feyn-cmap1.png", pix2, IFF_PNG);
    cmap = pixcmapCreate(1);
    pixcmapAddColor(cmap, 200, 0, 0);  /* with cmap: red bg, white letters */
    pixcmapAddColor(cmap, 255, 255, 255);
    pixSetColormap(pix1, cmap);
    pixWrite("/tmp/lept/pdf1/feyn-cmap2.png", pix1, IFF_PNG);

    convertToPdf("/tmp/lept/pdf1/feyn-nocmap.png", L_FLATE_ENCODE, 0,
                 "/tmp/lept/pdf1/file20.pdf",
                 0, 0, 0, NULL, NULL, 0);
    convertToPdf("/tmp/lept/pdf1/feyn-cmap1.png", L_FLATE_ENCODE, 0,
                 "/tmp/lept/pdf1/file21.pdf",
                 0, 0, 0, NULL, NULL, 0);
    convertToPdf("/tmp/lept/pdf1/feyn-cmap2.png", L_FLATE_ENCODE, 0,
                 "/tmp/lept/pdf1/file22.pdf",
                 0, 0, 0, NULL, NULL, 0);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
#endif

#if 1
    /* ----------  Generating from 1 bpp images (low-level) -------------- */
    lept_stderr("\n*** Writing 1 bpp images as pdf files (low-level)\n");
    pix1 = pixRead("cat-and-mouse.png");
    pix2 = pixConvertRGBToCmapLossless(pix1);  /* restore the cmap */

        /* Add a black/white colormap */
    cmap = pixcmapCreate(1);
    pixcmapAddColor(cmap, 255, 255, 255);  /* white = 0 */
    pixcmapAddColor(cmap, 0, 0, 0);  /* black = 1 */
    pixSetColormap(pix2, cmap);  /* replace with a b/w colormap */
    pixWrite("/tmp/lept/pdf1/cat-and-mouse-cmap1.png", pix2, IFF_PNG);

        /* Generate a pdf from this pix. The pdf has the colormap */
    pixGenerateCIData(pix2, L_FLATE_ENCODE, 0, 0, &cid);
    lept_stderr("  Should have 2 colors: %d\n", cid->ncolors);
    cidConvertToPdfData(cid, "with colormap", &data8, &nbytes);
    l_binaryWrite("/tmp/lept/pdf1/file23.pdf", "w", data8, nbytes);
    lept_free(data8);

        /* Generate a pdf from the colormap file:
         *   l_generateCIDataForPdf() calls l_generateFlateDataPdf()
         *   which calls pixRead(), removing the cmap  */
    l_generateCIDataForPdf("/tmp/lept/pdf1/cat-and-mouse-cmap1.png",
                           NULL, 75, &cid);
    lept_stderr("  Should have 0 colors: %d\n", cid->ncolors);
    cidConvertToPdfData(cid, "no colormap", &data8, &nbytes);
    l_binaryWrite("/tmp/lept/pdf1/file24.pdf", "w", data8, nbytes);
    lept_free(data8);

        /* Use an arbitrary colormap */
    cmap = pixcmapCreate(1);
    pixcmapAddColor(cmap, 254, 240, 185);  // yellow
    pixcmapAddColor(cmap, 50, 50, 130);   // blue
    pixSetColormap(pix2, cmap);
    pixWrite("/tmp/lept/pdf1/cat-and-mouse-cmap2.png", pix2, IFF_PNG);

       /* Generate a pdf from this pix. The pdf has the colormap. */
    pixGenerateCIData(pix2, L_FLATE_ENCODE, 0, 0, &cid);
    lept_stderr("  Should have 2 colors: %d\n", cid->ncolors);
    cidConvertToPdfData(cid, "with colormap", &data8, &nbytes);
    l_binaryWrite("/tmp/lept/pdf1/file25.pdf", "w", data8, nbytes);
    lept_free(data8);

        /* Generate a pdf from the cmap file.  No cmap in the pdf. */
    l_generateCIDataForPdf("/tmp/lept/pdf1/cat-and-mouse-cmap2.png",
                           NULL, 75, &cid);
    lept_stderr("  Should have 0 colors: %d\n", cid->ncolors);
    cidConvertToPdfData(cid, "no colormap", &data8, &nbytes);
    l_binaryWrite("/tmp/lept/pdf1/file26.pdf", "w", data8, nbytes);
    lept_free(data8);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
#endif

    regTestCheckFile(rp, "/tmp/lept/pdf1/file00.pdf");
    regTestCheckFile(rp, "/tmp/lept/pdf1/file01.pdf");
    regTestCheckFile(rp, "/tmp/lept/pdf1/file02.pdf");
    regTestCheckFile(rp, "/tmp/lept/pdf1/file03.pdf");
    regTestCheckFile(rp, "/tmp/lept/pdf1/file04.pdf");
    regTestCheckFile(rp, "/tmp/lept/pdf1/file05.pdf");
    regTestCheckFile(rp, "/tmp/lept/pdf1/file06.pdf");
    regTestCheckFile(rp, "/tmp/lept/pdf1/file07.pdf");
    regTestCheckFile(rp, "/tmp/lept/pdf1/file08.pdf");
    regTestCheckFile(rp, "/tmp/lept/pdf1/file09.pdf");
    regTestCheckFile(rp, "/tmp/lept/pdf1/file10.pdf");
    regTestCheckFile(rp, "/tmp/lept/pdf1/file11.pdf");
    regTestCheckFile(rp, "/tmp/lept/pdf1/file12.pdf");
    regTestCheckFile(rp, "/tmp/lept/pdf1/file13.pdf");
    regTestCheckFile(rp, "/tmp/lept/pdf1/file14.pdf");
    regTestCheckFile(rp, "/tmp/lept/pdf1/file15.pdf");
    regTestCheckFile(rp, "/tmp/lept/pdf1/file16.pdf");
    regTestCheckFile(rp, "/tmp/lept/pdf1/file17.pdf");
    regTestCheckFile(rp, "/tmp/lept/pdf1/file18.pdf");
    regTestCheckFile(rp, "/tmp/lept/pdf1/file19.pdf");
    regTestCheckFile(rp, "/tmp/lept/pdf1/file20.pdf");
    regTestCheckFile(rp, "/tmp/lept/pdf1/file21.pdf");
    regTestCheckFile(rp, "/tmp/lept/pdf1/file22.pdf");
    regTestCheckFile(rp, "/tmp/lept/pdf1/file23.pdf");
    regTestCheckFile(rp, "/tmp/lept/pdf1/file24.pdf");
    regTestCheckFile(rp, "/tmp/lept/pdf1/file25.pdf");
    regTestCheckFile(rp, "/tmp/lept/pdf1/file26.pdf");
    return regTestCleanup(rp);
}
