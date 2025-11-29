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
 * ioformats_reg.c
 *
 *    !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
 *    This is the primary Leptonica regression test for lossless
 *    read/write I/O to standard image files (png, tiff, bmp, etc.)
 *    !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
 *
 *    This tests reading and writing of images in different formats
 *    It should work properly on input images of any depth, with
 *    and without colormaps.  There are 7 sections.
 *
 *    Section 1. Test write/read with lossless and lossy compression, with
 *    and without colormaps.  The lossless results are tested for equality.
 *
 *    Section 2. Test read/write to file with different tiff compressions.
 *
 *    Section 3. Test read/write to memory with different tiff compressions.
 *
 *    Section 4. Test read/write to memory with other compression formats.
 *
 *    Section 5. Test multippage tiff read/write to file and memory.
 *
 *    Section 6. Test writing 24 bpp (not 32 bpp) pix
 *
 *    Section 7. Miscellaneous additional tests
 *
 *    Section 8. Test header reading
 *
 *    This test requires the following external I/O libraries
 *        libjpeg, libtiff, libpng, libz
 *    and optionally tests these:
 *        libwebp, libopenjp2, libgif
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif /* HAVE_CONFIG_H */

#include "allheaders.h"

#define   BMP_FILE             "test1.bmp"
#define   FILE_1BPP            "feyn.tif"
#define   FILE_2BPP            "speckle2.png"
#define   FILE_2BPP_C          "weasel2.4g.png"
#define   FILE_4BPP            "speckle4.png"
#define   FILE_4BPP_C          "weasel4.16c.png"
#define   FILE_8BPP_1          "dreyfus8.png"
#define   FILE_8BPP_2          "weasel8.240c.png"
#define   FILE_8BPP_3          "test8.jpg"
#define   FILE_16BPP           "test16.tif"
#define   FILE_32BPP           "marge.jpg"
#define   FILE_32BPP_ALPHA     "test32-alpha.png"
#define   FILE_1BIT_ALPHA      "test-1bit-alpha.png"
#define   FILE_CMAP_ALPHA      "test-cmap-alpha.png"
#define   FILE_TRANS_ALPHA     "test-fulltrans-alpha.png"
#define   FILE_GRAY_ALPHA      "test-gray-alpha.png"
#define   FILE_GRAY_ALPHA_TIF  "gray-alpha.tif"
#define   FILE_RGB16_TIF       "rgb16.tif"

static l_int32 testcomp(const char *filename, PIX *pix, l_int32 comptype);
static l_int32 testcomp_mem(PIX *pixs, PIX **ppixt, l_int32 index,
                            l_int32 format);
static l_int32 test_writemem(PIX *pixs, l_int32 format, char *psfile);
static PIX *make_24_bpp_pix(PIX *pixs);
static l_int32 get_header_data(const char *filename, l_int32 true_format);
static const char *get_tiff_compression_name(l_int32 format);

LEPT_DLL extern const char *ImageFileFormatExtensions[];

int main(int    argc,
         char **argv)
{
char          psname[256];
char         *tempname;
l_uint8      *data;
l_int32       i, d, n, success, failure, same;
l_int32       w, h, bps, spp, iscmap, res;
size_t        size, nbytes;
PIX          *pix1, *pix2, *pix3, *pix4, *pix5, *pix6, *pix8, *pix16, *pix32;
PIX          *pix, *pixt, *pixd;
PIXA         *pixa;
PIXCMAP      *cmap;
L_REGPARAMS  *rp;

#if  !HAVE_LIBJPEG
    lept_stderr("Omitting libjpeg tests in ioformats_reg\n");
#endif  /* !HAVE_LIBJPEG */

#if  !HAVE_LIBTIFF
    lept_stderr("Omitting libtiff tests in ioformats_reg\n");
#endif  /* !HAVE_LIBTIFF */

#if  !HAVE_LIBPNG || !HAVE_LIBZ
    lept_stderr("Omitting libpng tests in ioformats_reg\n");
#endif  /* !HAVE_LIBPNG || !HAVE_LIBZ */

#if  !HAVE_LIBWEBP
    lept_stderr("Omitting libwebp tests in ioformats_reg\n");
#endif  /* !HAVE_LIBWEBP */

#if  !HAVE_LIBJP2K
    lept_stderr("Omitting libopenjp2 tests in ioformats_reg\n");
#endif  /* !HAVE_LIBJP2K */

#if  !HAVE_LIBGIF
    lept_stderr("Omitting libgif tests in ioformats_reg\n");
#endif  /* !HAVE_LIBGIF */

    if (regTestSetup(argc, argv, &rp))
        return 1;

    /* --------- Part 1: Test all formats for r/w to file ---------*/

    failure = FALSE;
    success = TRUE;
    lept_stderr("Test bmp 1 bpp file:\n");
    if (ioFormatTest(BMP_FILE)) success = FALSE;

#if  HAVE_LIBTIFF
    lept_stderr("\nTest other 1 bpp file:\n");
    if (ioFormatTest(FILE_1BPP)) success = FALSE;
#endif  /* HAVE_LIBTIFF */

#if  HAVE_LIBPNG
    lept_stderr("\nTest 2 bpp file:\n");
    if (ioFormatTest(FILE_2BPP)) success = FALSE;
    lept_stderr("\nTest 2 bpp file with cmap:\n");
    if (ioFormatTest(FILE_2BPP_C)) success = FALSE;
    lept_stderr("\nTest 4 bpp file:\n");
    if (ioFormatTest(FILE_4BPP)) success = FALSE;
    lept_stderr("\nTest 4 bpp file with cmap:\n");
    if (ioFormatTest(FILE_4BPP_C)) success = FALSE;
    lept_stderr("\nTest 8 bpp grayscale file with cmap:\n");
    if (ioFormatTest(FILE_8BPP_1)) success = FALSE;
    lept_stderr("\nTest 8 bpp color file with cmap:\n");
    if (ioFormatTest(FILE_8BPP_2)) success = FALSE;
#endif  /* HAVE_LIBPNG */

#if  HAVE_LIBJPEG
    lept_stderr("\nTest 8 bpp file without cmap:\n");
    if (ioFormatTest(FILE_8BPP_3)) success = FALSE;
#endif  /* HAVE_LIBJPEG */

#if  HAVE_LIBTIFF
    lept_stderr("\nTest 16 bpp file:\n");
    if (ioFormatTest(FILE_16BPP)) success = FALSE;
#endif  /* HAVE_LIBTIFF */

#if  HAVE_LIBJPEG
    lept_stderr("\nTest 32 bpp files:\n");
    if (ioFormatTest(FILE_32BPP)) success = FALSE;
    if (ioFormatTest(FILE_32BPP_ALPHA)) success = FALSE;
#endif  /* HAVE_LIBJPEG */

#if  HAVE_LIBPNG && HAVE_LIBJPEG
    lept_stderr("\nTest spp = 1, bpp = 1, cmap with alpha file:\n");
    if (ioFormatTest(FILE_1BIT_ALPHA)) success = FALSE;
    lept_stderr("\nTest spp = 1, bpp = 8, cmap with alpha file:\n");
    if (ioFormatTest(FILE_CMAP_ALPHA)) success = FALSE;
    lept_stderr("\nTest spp = 1, fully transparent with alpha file:\n");
    if (ioFormatTest(FILE_TRANS_ALPHA)) success = FALSE;
    lept_stderr("\nTest spp = 2, gray with alpha file:\n");
    if (ioFormatTest(FILE_GRAY_ALPHA)) success = FALSE;
#endif  /* HAVE_LIBJPEG */

    if (success)
        lept_stderr(
            "\n  ********** Success on all i/o format tests *********\n");
    else
        lept_stderr(
            "\n  ******* Failure on at least one i/o format test ******\n");
    if (!success) failure = TRUE;


    /* ------------------ Part 2: Test tiff r/w to file ------------------- */
#if  !HAVE_LIBTIFF
    lept_stderr("\nNo libtiff.  Skipping:\n"
                    "  part 2 (tiff r/w)\n"
                    "  part 3 (tiff r/w to memory)\n"
                    "  part 4 (non-tiff r/w to memory)\n"
                    "  part 5 (multipage tiff r/w to memory)\n\n");
    goto part6;
#endif  /* !HAVE_LIBTIFF */

    lept_stderr("\nTest tiff r/w and format extraction\n");
    pixa = pixaCreate(6);
    pix1 = pixRead(BMP_FILE);
    pix2 = pixConvert1To2(NULL, pix1, 3, 0);
    pix4 = pixConvert1To4(NULL, pix1, 15, 0);
    pix16 = pixRead(FILE_16BPP);
    lept_stderr("Input format: %d\n", pixGetInputFormat(pix16));
    pix8 = pixConvert16To8(pix16, 1);
    pix32 = pixRead(FILE_32BPP);
    pixaAddPix(pixa, pix1, L_INSERT);
    pixaAddPix(pixa, pix2, L_INSERT);
    pixaAddPix(pixa, pix4, L_INSERT);
    pixaAddPix(pixa, pix8, L_INSERT);
    pixaAddPix(pixa, pix16, L_INSERT);
    pixaAddPix(pixa, pix32, L_INSERT);
    n = pixaGetCount(pixa);

    success = (n < 6) ? FALSE : TRUE;
    if (!success)
        lept_stderr("Error: only %d / 6 images loaded\n", n);
    for (i = 0; i < n; i++) {
        if ((pix = pixaGetPix(pixa, i, L_CLONE)) == NULL) {
            success = FALSE;
            continue;
        }
        d = pixGetDepth(pix);
        lept_stderr("%d bpp\n", d);
        if (i == 0) {   /* 1 bpp */
            pixWrite("/tmp/lept/regout/junkg3.tif", pix, IFF_TIFF_G3);
            pixWrite("/tmp/lept/regout/junkg4.tif", pix, IFF_TIFF_G4);
            pixWrite("/tmp/lept/regout/junkrle.tif", pix, IFF_TIFF_RLE);
            pixWrite("/tmp/lept/regout/junkpb.tif", pix, IFF_TIFF_PACKBITS);
            if (testcomp("/tmp/lept/regout/junkg3.tif", pix, IFF_TIFF_G3))
                success = FALSE;
            if (testcomp("/tmp/lept/regout/junkg4.tif", pix, IFF_TIFF_G4))
                success = FALSE;
            if (testcomp("/tmp/lept/regout/junkrle.tif", pix, IFF_TIFF_RLE))
                success = FALSE;
            if (testcomp("/tmp/lept/regout/junkpb.tif", pix, IFF_TIFF_PACKBITS))
                success = FALSE;
        }
        pixWrite("/tmp/lept/regout/junklzw.tif", pix, IFF_TIFF_LZW);
        pixWrite("/tmp/lept/regout/junkzip.tif", pix, IFF_TIFF_ZIP);
        pixWrite("/tmp/lept/regout/junknon.tif", pix, IFF_TIFF);
        if (testcomp("/tmp/lept/regout/junklzw.tif", pix, IFF_TIFF_LZW))
            success = FALSE;
        if (testcomp("/tmp/lept/regout/junkzip.tif", pix, IFF_TIFF_ZIP))
            success = FALSE;
        if (testcomp("/tmp/lept/regout/junknon.tif", pix, IFF_TIFF))
            success = FALSE;
        pixDestroy(&pix);
    }

        /* Test writing and reading tiff colormaps */
    lept_stderr("Tiff read/write 8 bpp with cmap\n");
    pix1 = pixRead(FILE_8BPP_2);
    pixWrite("/tmp/lept/regout/weas8.tif", pix1, IFF_TIFF);
    readHeaderTiff("/tmp/lept/regout/weas8.tif", 0, &w, &h, &bps, &spp,
                   NULL, &iscmap, NULL);
    if (w != 82 || h != 73 || bps != 8 || spp != 1 || iscmap != 1) {
        lept_stderr("Header error testing tiff cmaps\n");
        success = FALSE;
    }
    pix2 = pixRead("/tmp/lept/regout/weas8.tif");
    pixWrite("/tmp/lept/regout/weas8a.tif", pix2, IFF_TIFF);
    pix3 = pixRead("/tmp/lept/regout/weas8a.tif");
    pixEqual(pix1, pix3, &same);
    if (!same) {
        lept_stderr("Tiff read/write failed for cmaps\n");
        success = FALSE;
    }
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);

        /* Test writing and reading 1 bpp tiff with colormap */
    lept_stderr("Tiff read/write 1 bpp with cmap\n");
    pix1 = pixRead("feyn-fract2.tif");
    cmap = pixcmapCreate(1);
    pixcmapAddColor(cmap, 0, 0, 0);  /* inverted b/w */
    pixcmapAddColor(cmap, 255, 255, 255);
    pixSetColormap(pix1, cmap);
    pixWrite("/tmp/lept/regout/fract1.tif", pix1, IFF_TIFF_ZIP);
    pix2 = pixRead("/tmp/lept/regout/fract1.tif");
    pixEqual(pix1, pix2, &same);
    if (!same) {
        lept_stderr("Tiff read/write failed for 1 bpp cmap\n");
        success = FALSE;
    }
    cmap = pixcmapCreate(1);
    pixcmapAddColor(cmap, 255, 255, 255);
    pixcmapAddColor(cmap, 100, 200, 50);  /* with color */
    pixSetColormap(pix1, cmap);  /* replace the colormap */
    pixWrite("/tmp/lept/regout/fract2.tif", pix1, IFF_TIFF_ZIP);
    pix3 = pixRead("/tmp/lept/regout/fract2.tif");
    pixEqual(pix1, pix3, &same);
    if (!same) {
        lept_stderr("Tiff read/write failed for 1 bpp color cmap\n");
        success = FALSE;
    }
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);

        /* Test writing and reading tiff with alpha */
    lept_stderr("Tiff read/write gray plus alpha\n");
    pix1 = pixRead(FILE_GRAY_ALPHA_TIF);  /* converts to RGBA */
    pixWrite("/tmp/lept/regout/graya.tif", pix1, IFF_TIFF);
    readHeaderTiff("/tmp/lept/regout/graya.tif", 0, &w, &h, &bps, &spp,
                   NULL, &iscmap, NULL);
    if (w != 100 || h != 100 || bps != 8 || spp != 4 || iscmap != 0) {
        lept_stderr("Header error testing tiff with alpha\n");
        success = FALSE;
    }
    pix2 = pixRead("/tmp/lept/regout/graya.tif");
    pixEqual(pix1, pix2, &same);
    if (!same) {
        lept_stderr("Tiff read/write failed for graya.tif\n");
        success = FALSE;
    }
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pix1 = pixRead(FILE_GRAY_ALPHA);  /* converts to RGBA */
    pixWriteTiff("/tmp/lept/regout/graya2.tif", pix1, IFF_TIFF_ZIP, "w");
    pix2 = pixRead("/tmp/lept/regout/graya2.tif");
    pixEqual(pix1, pix2, &same);
    if (!same) {
        lept_stderr("Tiff read/write failed for graya2.tif\n");
        success = FALSE;
    }
    pixDestroy(&pix1);
    pixDestroy(&pix2);

        /* Test reading 16 bit sampled rgb tiff */
    lept_stderr("Tiff read/write 16 bit sampled rgb\n");
    pix1 = pixRead(FILE_RGB16_TIF);  /* converts 16 to 8 bits RGB */
    pixWrite("/tmp/lept/regout/rgb16.tif", pix1, IFF_TIFF_ZIP);
    readHeaderTiff("/tmp/lept/regout/rgb16.tif", 0, &w, &h, &bps, &spp,
                   NULL, &iscmap, NULL);
    if (w != 129 || h != 90 || bps != 8 || spp != 3 || iscmap != 0) {
        lept_stderr("Header error testing tiff with alpha\n");
        success = FALSE;
    }
    pix2 = pixRead("/tmp/lept/regout/rgb16.tif");
    pixEqual(pix1, pix2, &same);
    if (!same) {
        lept_stderr("Tiff read/write failed for rgb16.tif\n");
        success = FALSE;
    }
    pixDestroy(&pix1);
    pixDestroy(&pix2);

        /* Test reading 32 bit rgb with approx half-sized tiff buffer */
    lept_stderr("Tiff read/write rgb with half-sized tiff buffer\n");
    pix1 = pixRead("testbuffer.tif");
    pixWrite("/tmp/lept/regout/testbuffer.tif", pix1, IFF_TIFF_ZIP);
    readHeaderTiff("/tmp/lept/regout/testbuffer.tif", 0, &w, &h, &bps, &spp,
                   &res, &iscmap, NULL);
    if (w != 659 || h != 799 || bps != 8 || spp != 3 || res != 96) {
        lept_stderr("Header error testing rgb tiff with small tif buffer\n");
        success = FALSE;
    }
    pix2 = pixRead("/tmp/lept/regout/testbuffer.tif");
    pixEqual(pix1, pix2, &same);
    if (!same) {
        lept_stderr("Tiff read/write failed for testbuffer.tif\n");
        success = FALSE;
    }
    pixDestroy(&pix1);
    pixDestroy(&pix2);

    if (success)
        lept_stderr(
            "\n  ********** Success on tiff r/w to file *********\n\n");
    else
        lept_stderr(
            "\n  ******* Failure on at least one tiff r/w to file ******\n\n");
    if (!success) failure = TRUE;

    /* ------------------ Part 3: Test tiff r/w to memory ----------------- */

    success = (n < 6) ? FALSE : TRUE;
    for (i = 0; i < n; i++) {
        if ((pix = pixaGetPix(pixa, i, L_CLONE)) == NULL) {
            success = FALSE;
            continue;
        }
        d = pixGetDepth(pix);
        lept_stderr("%d bpp\n", d);
        if (i == 0) {   /* 1 bpp */
            pixWriteMemTiff(&data, &size, pix, IFF_TIFF_G3);
            nbytes = nbytesInFile("/tmp/lept/regout/junkg3.tif");
            lept_stderr("nbytes = %lu, size = %lu\n",
                        (unsigned long)nbytes, (unsigned long)size);
            pixt = pixReadMemTiff(data, size, 0);
            if (testcomp_mem(pix, &pixt, i, IFF_TIFF_G3)) success = FALSE;
            lept_free(data);
            pixWriteMemTiff(&data, &size, pix, IFF_TIFF_G4);
            nbytes = nbytesInFile("/tmp/lept/regout/junkg4.tif");
            lept_stderr("nbytes = %lu, size = %lu\n",
                        (unsigned long)nbytes, (unsigned long)size);
            pixt = pixReadMemTiff(data, size, 0);
            if (testcomp_mem(pix, &pixt, i, IFF_TIFF_G4)) success = FALSE;
            readHeaderMemTiff(data, size, 0, &w, &h, &bps, &spp,
                              NULL, NULL, NULL);
            lept_stderr("(w,h,bps,spp) = (%d,%d,%d,%d)\n", w, h, bps, spp);
            lept_free(data);
            pixWriteMemTiff(&data, &size, pix, IFF_TIFF_RLE);
            nbytes = nbytesInFile("/tmp/lept/regout/junkrle.tif");
            lept_stderr("nbytes = %lu, size = %lu\n",
                        (unsigned long)nbytes, (unsigned long)size);
            pixt = pixReadMemTiff(data, size, 0);
            if (testcomp_mem(pix, &pixt, i, IFF_TIFF_RLE)) success = FALSE;
            lept_free(data);
            pixWriteMemTiff(&data, &size, pix, IFF_TIFF_PACKBITS);
            nbytes = nbytesInFile("/tmp/lept/regout/junkpb.tif");
            lept_stderr("nbytes = %lu, size = %lu\n",
                        (unsigned long)nbytes, (unsigned long)size);
            pixt = pixReadMemTiff(data, size, 0);
            if (testcomp_mem(pix, &pixt, i, IFF_TIFF_PACKBITS)) success = FALSE;
            lept_free(data);
        }
        pixWriteMemTiff(&data, &size, pix, IFF_TIFF_LZW);
        pixt = pixReadMemTiff(data, size, 0);
        if (testcomp_mem(pix, &pixt, i, IFF_TIFF_LZW)) success = FALSE;
        lept_free(data);
        pixWriteMemTiff(&data, &size, pix, IFF_TIFF_ZIP);
        pixt = pixReadMemTiff(data, size, 0);
        if (testcomp_mem(pix, &pixt, i, IFF_TIFF_ZIP)) success = FALSE;
        readHeaderMemTiff(data, size, 0, &w, &h, &bps, &spp, NULL, NULL, NULL);
        lept_stderr("(w,h,bps,spp) = (%d,%d,%d,%d)\n", w, h, bps, spp);
        lept_free(data);
        pixWriteMemTiff(&data, &size, pix, IFF_TIFF);
        pixt = pixReadMemTiff(data, size, 0);
        if (testcomp_mem(pix, &pixt, i, IFF_TIFF)) success = FALSE;
        lept_free(data);
        pixDestroy(&pix);
    }
    if (success)
        lept_stderr(
            "\n  ********** Success on tiff r/w to memory *********\n\n");
    else
        lept_stderr(
            "\n  ******* Failure on at least one tiff r/w to memory ******\n\n");
    if (!success) failure = TRUE;

    /* ---------------- Part 4: Test non-tiff r/w to memory ---------------- */

    success = (n < 6) ? FALSE : TRUE;
    for (i = 0; i < n; i++) {
        if ((pix = pixaGetPix(pixa, i, L_CLONE)) == NULL) {
            success = FALSE;
            continue;
        }
        d = pixGetDepth(pix);
        snprintf(psname, sizeof(psname), "/tmp/lept/regout/junkps.%d", d);
        lept_stderr("%d bpp\n", d);
        if (test_writemem(pix, IFF_PNM, NULL)) success = FALSE;
        if (test_writemem(pix, IFF_PS, psname)) success = FALSE;
        if (d == 16) {
          pixDestroy(&pix);
          continue;
        }
        if (test_writemem(pix, IFF_PNG, NULL)) success = FALSE;
        if (test_writemem(pix, IFF_BMP, NULL)) success = FALSE;
        if (d != 32)
            if (test_writemem(pix, IFF_GIF, NULL)) success = FALSE;
        if (d == 8 || d == 32) {
            if (test_writemem(pix, IFF_JFIF_JPEG, NULL)) success = FALSE;
            if (test_writemem(pix, IFF_JP2, NULL)) success = FALSE;
            if (test_writemem(pix, IFF_WEBP, NULL)) success = FALSE;
        }
        pixDestroy(&pix);
    }
    if (success)
        lept_stderr(
            "\n  ********** Success on non-tiff r/w to memory *********\n\n");
    else
        lept_stderr(
           "\n  **** Failure on at least one non-tiff r/w to memory *****\n\n");
    if (!success) failure = TRUE;
    pixaDestroy(&pixa);

    /* ------------ Part 5: Test multipage tiff r/w to memory ------------ */

        /* Make a multipage tiff file, and read it back into memory */
    pix = pixRead("feyn.tif");
    pixa = pixaSplitPix(pix, 3, 3, 0, 0);
    for (i = 0; i < 9; i++) {
        if ((pixt = pixaGetPix(pixa, i, L_CLONE)) == NULL)
            continue;
        if (i == 0)
            pixWriteTiff("/tmp/lept/regout/junktiffmpage.tif", pixt,
                         IFF_TIFF_G4, "w");
        else
            pixWriteTiff("/tmp/lept/regout/junktiffmpage.tif", pixt,
                         IFF_TIFF_G4, "a");
        pixDestroy(&pixt);
    }
    data = l_binaryRead("/tmp/lept/regout/junktiffmpage.tif", &nbytes);
    pixaDestroy(&pixa);

        /* Read the individual pages from memory to a pix */
    pixa = pixaCreate(9);
    for (i = 0; i < 9; i++) {
        pixt = pixReadMemTiff(data, nbytes, i);
        pixaAddPix(pixa, pixt, L_INSERT);
    }
    lept_free(data);

        /* Un-tile the pix in the pixa back to the original image */
    pixt = pixaDisplayUnsplit(pixa, 3, 3, 0, 0);
    pixaDestroy(&pixa);

        /* Clip to foreground to remove any extra rows or columns */
    pixClipToForeground(pix, &pix1, NULL);
    pixClipToForeground(pixt, &pix2, NULL);
    pixEqual(pix1, pix2, &same);
    if (same)
        lept_stderr(
           "\n  ******* Success on tiff multipage read from memory ******\n\n");
    else
        lept_stderr(
           "\n  ******* Failure on tiff multipage read from memory ******\n\n");
    if (!same) failure = TRUE;

    pixDestroy(&pix);
    pixDestroy(&pixt);
    pixDestroy(&pix1);
    pixDestroy(&pix2);

    /* ------------ Part 6: Test 24 bpp writing ------------ */
#if  !HAVE_LIBTIFF
part6:
#endif  /* !HAVE_LIBTIFF */

#if  !HAVE_LIBPNG || !HAVE_LIBJPEG || !HAVE_LIBTIFF
    lept_stderr("Missing libpng, libjpeg or libtiff.  Skipping:\n"
                "  part 6 (24 bpp r/w)\n"
                "  part 7 (header read)\n\n");
    goto finish;
#endif  /* !HAVE_LIBPNG || !HAVE_LIBJPEG || !HAVE_LIBTIFF */

        /* Generate a 24 bpp (not 32 bpp !!) rgb pix and write it out */
    success = TRUE;
    if ((pix = pixRead("marge.jpg")) == NULL)
        success = FALSE;
    pixt = make_24_bpp_pix(pix);
    pixWrite("/tmp/lept/regout/junk24.png", pixt, IFF_PNG);
    pixWrite("/tmp/lept/regout/junk24.jpg", pixt, IFF_JFIF_JPEG);
    pixWrite("/tmp/lept/regout/junk24.tif", pixt, IFF_TIFF);
    pixd = pixRead("/tmp/lept/regout/junk24.png");
    pixEqual(pix, pixd, &same);
    if (same) {
        lept_stderr("    **** success writing 24 bpp png ****\n");
    } else {
        lept_stderr("    **** failure writing 24 bpp png ****\n");
        success = FALSE;
    }
    pixDestroy(&pixd);
    pixd = pixRead("/tmp/lept/regout/junk24.jpg");
    regTestCompareSimilarPix(rp, pix, pixd, 10, 0.0002, 0);  /* 0 */
    pixDestroy(&pixd);
    pixd = pixRead("/tmp/lept/regout/junk24.tif");
    pixEqual(pix, pixd, &same);
    if (same) {
        lept_stderr("    **** success writing 24 bpp tif ****\n");
    } else {
        lept_stderr("    **** failure writing 24 bpp tif ****\n");
        success = FALSE;
    }
    pixDestroy(&pixd);
    if (success)
        lept_stderr("\n  ******* Success on 24 bpp rgb writing *******\n\n");
    else
        lept_stderr("\n  ******* Failure on 24 bpp rgb writing *******\n\n");
    if (!success) failure = TRUE;
    pixDestroy(&pix);
    pixDestroy(&pixt);

    /* ------------- Part 7: Miscellaneous additional tests ------------ */
        /* Test writing and reading alpha */
    pix1 = pixRead("test32-alpha.png");
    lept_stderr("Test write/read of BMP with alpha\n");
    if (test_writemem(pix1, IFF_BMP, NULL)) success = FALSE;
    lept_stderr("Test write/read of PNG with alpha\n");
    if (test_writemem(pix1, IFF_PNG, NULL)) success = FALSE;
    pixDestroy(&pix1);

    pix1 = pixRead("test-rgba.bmp");
    pixWrite("/tmp/lept/regout/alpha1.bmp", pix1, IFF_BMP);
    pixWrite("/tmp/lept/regout/alpha1.png", pix1, IFF_PNG);
    pix2 = pixRead("/tmp/lept/regout/alpha1.bmp");
    pix3 = pixRead("/tmp/lept/regout/alpha1.png");
    pixEqual(pix2, pix1, &same);
    regTestCompareValues(rp, 1.0, same, 0.0);  /* 1 */
    pixEqual(pix3, pix1, &same);
    regTestCompareValues(rp, 1.0, same, 0.0);  /* 2 */
    pixWrite("/tmp/lept/regout/alpha2.bmp", pix2, IFF_BMP);
    pix4 = pixRead("/tmp/lept/regout/alpha2.bmp");
    pixEqual(pix4, pix1, &same);
    regTestCompareValues(rp, 1.0, same, 0.0);  /* 3 */
    if (rp->display) {
        writeImageFileInfo("/tmp/lept/regout/alpha2.bmp", stderr, 0);
        pixDisplay(pix1, 300, 100);
    }
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);

        /* Test conversion between 32 and 24 bpp */
    lept_stderr("Test conversion between 32 and 24 bpp\n");
    pix1 = pixRead("test-rgba.bmp");
    pix2 = pixConvert32To24(pix1);
    pix3 = pixConvert24To32(pix2);
    pixEqual(pix3, pix1, &same);  /* bmp */
    regTestCompareValues(rp, 1.0, same, 0.0);  /* 4 */
    pix4 = pixDisplayDiff(pix1, pix3, 1, 1, 0xff000000);
    regTestWritePixAndCheck(rp, pix4, IFF_PNG);  /* 5 */
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);

        /* Test writing and reading 24 bpp BMP and PNG */
    lept_stderr("Test write/read of 24 bpp in BMP and PNG\n");
    pix1 = pixRead("test-rgba.bmp");
    pix2 = pixConvert32To24(pix1);
    pixWrite("/tmp/lept/regout/alpha3.bmp", pix2, IFF_BMP);
    pixWrite("/tmp/lept/regout/alpha3.png", pix2, IFF_PNG);
    pix3 = pixRead("/tmp/lept/regout/alpha3.bmp");
    pix4 = pixRead("/tmp/lept/regout/alpha3.png");
    pixEqual(pix3, pix1, &same);  /* bmp */
    regTestCompareValues(rp, 1.0, same, 0.0);  /* 6 */
    pix5 = pixDisplayDiff(pix1, pix3, 1, 1, 0xff000000);
    regTestWritePixAndCheck(rp, pix5, IFF_PNG);  /* 7 */
    pixEqual(pix4, pix1, &same);  /* png */
    regTestCompareValues(rp, 1.0, same, 0.0);  /* 8 */
    pix6 = pixDisplayDiff(pix1, pix4, 1, 1, 0xff000000);
    regTestWritePixAndCheck(rp, pix6, IFF_PNG);  /* 9 */
    if (rp->display) pixDisplay(pix6, 800, 100);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);
    pixDestroy(&pix5);
    pixDestroy(&pix6);
    if (rp->success == FALSE) success = FALSE;

    if (success)
        lept_stderr("\n  ******* Success on misc tests *******\n\n");
    else
        lept_stderr("\n  ******* Failure on misc tests *******\n\n");
    if (!success) failure = TRUE;
    pixDestroy(&pix);

    /* -------------- Part 8: Read header information -------------- */
    success = TRUE;
    if (get_header_data(FILE_1BPP, IFF_TIFF_G4)) success = FALSE;
    if (get_header_data(FILE_2BPP, IFF_PNG)) success = FALSE;
    if (get_header_data(FILE_2BPP_C, IFF_PNG)) success = FALSE;
    if (get_header_data(FILE_4BPP, IFF_PNG)) success = FALSE;
    if (get_header_data(FILE_4BPP_C, IFF_PNG)) success = FALSE;
    if (get_header_data(FILE_8BPP_1, IFF_PNG)) success = FALSE;
    if (get_header_data(FILE_8BPP_2, IFF_PNG)) success = FALSE;
    if (get_header_data(FILE_8BPP_3, IFF_JFIF_JPEG)) success = FALSE;
    if (get_header_data(FILE_GRAY_ALPHA, IFF_PNG)) success = FALSE;
    if (get_header_data(FILE_16BPP, IFF_TIFF_ZIP)) success = FALSE;
    if (get_header_data(FILE_32BPP, IFF_JFIF_JPEG)) success = FALSE;
    if (get_header_data(FILE_32BPP_ALPHA, IFF_PNG)) success = FALSE;

    pix = pixRead(FILE_8BPP_1);
    tempname = l_makeTempFilename();
    pixWrite(tempname, pix, IFF_PNM);
    if (get_header_data(tempname, IFF_PNM)) success = FALSE;
    pixDestroy(&pix);

        /* These tiff formats work on 1 bpp images */
    pix = pixRead(FILE_1BPP);
    pixWrite(tempname, pix, IFF_TIFF_G3);
    if (get_header_data(tempname, IFF_TIFF_G3)) success = FALSE;
    pixWrite(tempname, pix, IFF_TIFF_G4);
    if (get_header_data(tempname, IFF_TIFF_G4)) success = FALSE;
    pixWrite(tempname, pix, IFF_TIFF_PACKBITS);
    if (get_header_data(tempname, IFF_TIFF_PACKBITS)) success = FALSE;
    pixWrite(tempname, pix, IFF_TIFF_RLE);
    if (get_header_data(tempname, IFF_TIFF_RLE)) success = FALSE;
    pixWrite(tempname, pix, IFF_TIFF_LZW);
    if (get_header_data(tempname, IFF_TIFF_LZW)) success = FALSE;
    pixWrite(tempname, pix, IFF_TIFF_ZIP);
    if (get_header_data(tempname, IFF_TIFF_ZIP)) success = FALSE;
    pixWrite(tempname, pix, IFF_TIFF);
    if (get_header_data(tempname, IFF_TIFF)) success = FALSE;
    pixDestroy(&pix);
    lept_rmfile(tempname);
    lept_free(tempname);

    if (success)
        lept_stderr( "\n  ******* Success on reading headers *******\n\n");
    else
        lept_stderr( "\n  ******* Failure on reading headers *******\n\n");
    if (!success) failure = TRUE;

#if  !HAVE_LIBPNG || !HAVE_LIBJPEG || !HAVE_LIBTIFF
finish:
#endif  /* !HAVE_LIBPNG || !HAVE_LIBJPEG || !HAVE_LIBTIFF */

    if (!failure)
        lept_stderr("  ******* Success on all tests *******\n\n");
    else
        lept_stderr("  ******* Failure on at least one test *******\n\n");

    if (failure) rp->success = FALSE;
    return regTestCleanup(rp);
}


    /* Returns 1 on error */
static l_int32
testcomp(const char  *filename,
         PIX         *pix,
         l_int32      comptype)
{
l_int32  format, sameformat, sameimage;
FILE    *fp;
PIX     *pixt;

    fp = lept_fopen(filename, "rb");
    findFileFormatStream(fp, &format);
    sameformat = TRUE;
    if (format != comptype) {
        lept_stderr("File %s has format %d, not comptype %d\n",
                    filename, format, comptype);
        sameformat = FALSE;
    }
    lept_fclose(fp);
    pixt = pixRead(filename);
    pixEqual(pix, pixt, &sameimage);
    pixDestroy(&pixt);
    if (!sameimage)
        lept_stderr("Write/read fail for file %s with format %d\n",
                    filename, format);
    return (!sameformat || !sameimage);
}


    /* Returns 1 on error */
static l_int32
testcomp_mem(PIX     *pixs,
             PIX    **ppixt,  /* input; nulled on return */
             l_int32  index,
             l_int32  format)
{
l_int32  sameimage;
PIX     *pixt;

    pixt = *ppixt;
    pixEqual(pixs, pixt, &sameimage);
    if (!sameimage)
        lept_stderr("Mem Write/read fail for file %d with format %d\n",
                    index, format);
    pixDestroy(&pixt);
    *ppixt = NULL;
    return (!sameimage);
}


    /* Returns 1 on error */
static l_int32
test_writemem(PIX      *pixs,
              l_int32   format,
              char     *psfile)
{
l_uint8   *data = NULL;
l_int32    same = TRUE;
l_int32    ds, dd;
l_float32  diff;
size_t     size = 0;
PIX       *pixd = NULL;

    if (format == IFF_PS) {
        pixWriteMemPS(&data, &size, pixs, NULL, 0, 1.0);
        l_binaryWrite(psfile, "w", data, size);
        lept_free(data);
        return 0;
    }

    /* Fail silently if library is not available */
#if !HAVE_LIBJPEG
    if (format == IFF_JFIF_JPEG)
        return 0;
#endif  /* !HAVE_LIBJPEG */
#if !HAVE_LIBPNG
    if (format == IFF_PNG)
        return 0;
#endif  /* !HAVE_LIBPNG */
#if !HAVE_LIBTIFF
    if (format == IFF_TIFF)
        return 0;
#endif  /* !HAVE_LIBTIFF */
#if !HAVE_LIBWEBP
    if (format == IFF_WEBP)
        return 0;
#endif  /* !HAVE_LIBWEBP */
#if !HAVE_LIBJP2K
    if (format == IFF_JP2)
        return 0;
#endif  /* !HAVE_LIBJP2K */
#if !HAVE_LIBGIF
    if (format == IFF_GIF)
        return 0;
#endif  /* !HAVE_LIBGIF */

    if (pixWriteMem(&data, &size, pixs, format)) {
        lept_stderr("Mem write fail for format %d\n", format);
        return 1;
    }
    if ((pixd = pixReadMem(data, size)) == NULL) {
        lept_stderr("Mem read fail for format %d\n", format);
        lept_free(data);
        return 1;
    }

    if (format == IFF_JFIF_JPEG || format == IFF_JP2 ||
        format == IFF_WEBP || format == IFF_TIFF_JPEG) {
        ds = pixGetDepth(pixs);
        dd = pixGetDepth(pixd);
        if (dd == 8) {
            pixCompareGray(pixs, pixd, L_COMPARE_ABS_DIFF, 0, NULL, &diff,
                           NULL, NULL);
        } else if (ds == 32 && dd == 32) {
            pixCompareRGB(pixs, pixd, L_COMPARE_ABS_DIFF, 0, NULL, &diff,
                          NULL, NULL);
        } else {
            lept_stderr("skipping: ds = %d, dd = %d, format = %d\n",
                        ds, dd, format);
            lept_free(data);
            pixDestroy(&pixd);
            return 0;
        }

/*        lept_stderr("  size = %lu bytes; diff = %5.2f, format = %d\n",
                      (unsigned long)size, diff, format); */
        if (diff > 8.0) {
            same = FALSE;
            lept_stderr("Mem write/read fail for format %d, diff = %5.2f\n",
                        format, diff);
        }
    } else {
        pixEqual(pixs, pixd, &same);
        if (!same)
            lept_stderr("Mem write/read fail for format %d\n", format);
    }
    pixDestroy(&pixd);
    lept_free(data);
    return (!same);
}


    /* Composes 24 bpp rgb pix */
static PIX *
make_24_bpp_pix(PIX  *pixs)
{
l_int32    i, j, w, h, wpls, wpld, rval, gval, bval;
l_uint32  *lines, *lined, *datas, *datad;
PIX       *pixd;

    pixGetDimensions(pixs, &w, &h, NULL);
    pixd = pixCreate(w, h, 24);
    datas = pixGetData(pixs);
    wpls = pixGetWpl(pixs);
    datad = pixGetData(pixd);
    wpld = pixGetWpl(pixd);
    for (i = 0; i < h; i++) {
        lines = datas + i * wpls;
        lined = datad + i * wpld;
        for (j = 0; j < w; j++) {
            extractRGBValues(lines[j], &rval, &gval, &bval);
            *((l_uint8 *)lined + 3 * j) = rval;
            *((l_uint8 *)lined + 3 * j + 1) = gval;
            *((l_uint8 *)lined + 3 * j + 2) = bval;
        }
    }

    return pixd;
}


    /* Retrieve header data from file */
static l_int32
get_header_data(const char  *filename,
                l_int32      true_format)
{
const char *tiff_compression_name = "undefined";
l_uint8    *data;
l_int32     ret1, ret2, format1, format2;
l_int32     w1, w2, h1, h2, d1, d2, bps1, bps2, spp1, spp2, iscmap1, iscmap2;
size_t      size1, size2;

    /* Fail silently if library is not available */
#if !HAVE_LIBJPEG
    if (true_format == IFF_JFIF_JPEG)
        return 0;
#endif  /* !HAVE_LIBJPEG */
#if !HAVE_LIBPNG
    if (true_format == IFF_PNG)
        return 0;
#endif  /* !HAVE_LIBPNG */
#if !HAVE_LIBTIFF
    if (L_FORMAT_IS_TIFF(true_format))
        return 0;
#endif  /* !HAVE_LIBTIFF */

        /* Read header from file */
    size1 = nbytesInFile(filename);
    ret1 = pixReadHeader(filename, &format1, &w1, &h1, &bps1, &spp1, &iscmap1);
    d1 = bps1 * spp1;
    if (d1 == 24) d1 = 32;
    if (ret1)
        lept_stderr("Error: couldn't read header data: %s\n", filename);
    else {
        if (L_FORMAT_IS_TIFF(format1)) {
            tiff_compression_name = get_tiff_compression_name(format1);
            lept_stderr("Format data for image %s with format %s:\n"
                        "  nbytes = %lu, size (w, h, d) = (%d, %d, %d)\n"
                        "  bps = %d, spp = %d, iscmap = %d\n",
                        filename, tiff_compression_name,
                        (unsigned long)size1, w1, h1, d1,
                        bps1, spp1, iscmap1);
        } else {
            lept_stderr("Format data for image %s with format %s:\n"
                        "  nbytes = %lu, size (w, h, d) = (%d, %d, %d)\n"
                        "  bps = %d, spp = %d, iscmap = %d\n",
                        filename, ImageFileFormatExtensions[format1],
                        (unsigned long)size1, w1, h1, d1, bps1, spp1, iscmap1);
        }
        if (format1 != true_format) {
            lept_stderr("Error: format is %d; should be %d\n",
                        format1, true_format);
            ret1 = 1;
        }
    }

        /* Read header from array in memory */
    data = l_binaryRead(filename, &size2);
    ret2 = pixReadHeaderMem(data, size2, &format2, &w2, &h2, &bps2,
                            &spp2, &iscmap2);
    lept_free(data);
    d2 = bps2 * spp2;
    if (d2 == 24) d2 = 32;
    if (ret2) {
        lept_stderr("Error: couldn't mem-read header data: %s\n", filename);
    } else {
        if (size1 != size2 || format1 != format2 || w1 != w2 ||
            h1 != h2 || d1 != d2 || bps1 != bps2 || spp1 != spp2 ||
            iscmap1 != iscmap2) {
            lept_stderr("Inconsistency reading image %s with format %s\n",
                        filename, tiff_compression_name);
            ret2 = 1;
        }
    }
    return ret1 || ret2;
}


static const char *
get_tiff_compression_name(l_int32  format)
{
    const char *tiff_compression_name = "unknown";
    if (format == IFF_TIFF_G4)
        tiff_compression_name = "tiff_g4";
    else if (format == IFF_TIFF_G3)
        tiff_compression_name =  "tiff_g3";
    else if (format == IFF_TIFF_ZIP)
        tiff_compression_name =  "tiff_zip";
    else if (format == IFF_TIFF_LZW)
        tiff_compression_name =  "tiff_lzw";
    else if (format == IFF_TIFF_RLE)
        tiff_compression_name =  "tiff_rle";
    else if (format == IFF_TIFF_PACKBITS)
        tiff_compression_name =  "tiff_packbits";
    else if (format == IFF_TIFF_JPEG)
        tiff_compression_name =  "tiff_jpeg";
    else if (format == IFF_TIFF)
        tiff_compression_name = "tiff_uncompressed";
    else
        lept_stderr("format %d: not tiff\n", format);
    return tiff_compression_name;
}
