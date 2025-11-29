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
 *   jpegio_reg.c
 *
 *    !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
 *    This is a Leptonica regression test for jpeg I/O
 *    !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
 *
 *    This tests reading and writing of images and image
 *    metadata, between Pix and compressed data in jpeg format.
 *
 *    This only tests properly written jpeg files.  To test
 *    reading of corrupted jpeg files to insure that the
 *    reader does not crash, use prog/corrupttest.c.
 *
 *    TODO (5/5/14): Add tests for
 *    (1) different color spaces
 *    (2) no chroma subsampling
 *    (3) luminance only reading
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include <string.h>
#include "allheaders.h"

    /* Needed for HAVE_LIBJPEG */
#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif /* HAVE_CONFIG_H */

void DoJpegTest1(L_REGPARAMS *rp, const char *fname);
void DoJpegTest2(L_REGPARAMS *rp, const char *fname);
void DoJpegTest3(L_REGPARAMS *rp, const char *fname);
void DoJpegTest4(L_REGPARAMS *rp, const char *fname);


int main(int    argc,
         char **argv)
{
L_REGPARAMS  *rp;

#if !HAVE_LIBJPEG
    lept_stderr("jpegio is not enabled\n"
                "See environ.h: #define HAVE_LIBJPEG\n"
                "See prog/Makefile: link in -ljpeg\n\n");
    return 0;
#endif  /* abort */

    if (regTestSetup(argc, argv, &rp))
        return 1;

    DoJpegTest1(rp, "test8.jpg");
    DoJpegTest1(rp, "fish24.jpg");
    DoJpegTest1(rp, "test24.jpg");
    DoJpegTest2(rp, "weasel2.png");
    DoJpegTest2(rp, "weasel2.4g.png");
    DoJpegTest2(rp, "weasel4.png");
    DoJpegTest2(rp, "weasel4.5g.png");
    DoJpegTest2(rp, "weasel4.16c.png");
    DoJpegTest2(rp, "weasel8.16g.png");
    DoJpegTest2(rp, "weasel8.240c.png");
    DoJpegTest3(rp, "lucasta.150.jpg");
    DoJpegTest3(rp, "tetons.jpg");
    DoJpegTest4(rp, "karen8.jpg");

    return regTestCleanup(rp);
}


/* Use this for 8 bpp (no cmap), 24 bpp or 32 bpp pix */
void DoJpegTest1(L_REGPARAMS  *rp,
                 const char   *fname)
{
size_t    size;
l_uint8  *data;
char      buf[256];
PIX      *pixs, *pix1, *pix2, *pix3, *pix4, *pix5;

        /* Test file read/write (general functions) */
    pixs = pixRead(fname);
    snprintf(buf, sizeof(buf), "/tmp/lept/regout/jpegio.%d.jpg", rp->index + 1);
    pixWrite(buf, pixs, IFF_JFIF_JPEG);
    pix1 = pixRead(buf);
    regTestCompareSimilarPix(rp, pixs, pix1, 6, 0.01, 0);
    pixDisplayWithTitle(pix1, 500, 100, "pix1", rp->display);

        /* Test memory read/write (general functions) */
    pixWriteMemJpeg(&data, &size, pixs, 75, 0);
    pix2 = pixReadMem(data, size);
    regTestComparePix(rp, pix1, pix2);
    lept_free(data);

        /* Test file read/write (specialized jpeg functions) */
    pix3 = pixReadJpeg(fname, 0, 1, NULL, 0);
    regTestComparePix(rp, pixs, pix3);
    snprintf(buf, sizeof(buf), "/tmp/lept/regout/jpegio.%d.jpg", rp->index + 1);
    pixWriteJpeg(buf, pix3, 75, 0);
    pix4 = pixReadJpeg(buf, 0, 1, NULL, 0);
    regTestComparePix(rp, pix2, pix4);

        /* Test memory read/write (specialized jpeg functions) */
    pixWriteMemJpeg(&data, &size, pixs, 75, 0);
    pix5 = pixReadMemJpeg(data, size, 0, 1, NULL, 0);
    regTestComparePix(rp, pix4, pix5);
    lept_free(data);

    pixDestroy(&pixs);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);
    pixDestroy(&pix5);
    return;
}

/* Use this for colormapped pix and for pix with d < 8 */
void DoJpegTest2(L_REGPARAMS  *rp,
                 const char   *fname)
{
size_t    size;
l_uint8  *data;
char      buf[256];
PIX      *pixs, *pix1, *pix2, *pix3, *pix4, *pix5, *pix6;

        /* Test file read/write (general functions) */
    pixs = pixRead(fname);
    snprintf(buf, sizeof(buf), "/tmp/lept/regout/jpegio.%d.jpg", rp->index + 1);
    pixWrite(buf, pixs, IFF_JFIF_JPEG);
    pix1 = pixRead(buf);
    if (pixGetColormap(pixs) != NULL)
        pix2 = pixRemoveColormap(pixs, REMOVE_CMAP_BASED_ON_SRC);
    else
        pix2 = pixConvertTo8(pixs, 0);
    regTestCompareSimilarPix(rp, pix1, pix2, 20, 0.2, 0);
    pixDisplayWithTitle(pix1, 500, 100, "pix1", rp->display);

        /* Test memory read/write (general functions) */
    pixWriteMemJpeg(&data, &size, pixs, 75, 0);
    pix3 = pixReadMem(data, size);
    regTestComparePix(rp, pix1, pix3);
    lept_free(data);

        /* Test file write (specialized jpeg function) */
    pix4 = pixRead(fname);
    snprintf(buf, sizeof(buf), "/tmp/lept/regout/jpegio.%d.jpg", rp->index + 1);
    pixWriteJpeg(buf, pix4, 75, 0);
    pix5 = pixReadJpeg(buf, 0, 1, NULL, 0);
    regTestComparePix(rp, pix5, pix5);

        /* Test memory write (specialized jpeg function) */
    pixWriteMemJpeg(&data, &size, pixs, 75, 0);
    pix6 = pixReadMemJpeg(data, size, 0, 1, NULL, 0);
    regTestComparePix(rp, pix5, pix6);
    lept_free(data);

    pixDestroy(&pixs);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);
    pixDestroy(&pix5);
    pixDestroy(&pix6);
    return;
}

void DoJpegTest3(L_REGPARAMS  *rp,
                 const char   *fname)
{
l_int32   w1, h1, bps1, spp1, w2, h2, bps2, spp2, format1, format2;
size_t    size;
l_uint8  *data;
PIX      *pixs;

        /* Test header reading (specialized jpeg functions) */
    readHeaderJpeg(fname, &w1, &h1, &spp1, NULL, NULL);
    pixs = pixRead(fname);
    pixWriteMemJpeg(&data, &size, pixs, 75, 0);
    readHeaderMemJpeg(data, size, &w2, &h2, &spp2, NULL, NULL);
    regTestCompareValues(rp, w1, w2, 0.0);
    regTestCompareValues(rp, h1, h2, 0.0);
    regTestCompareValues(rp, spp1, spp2, 0.0);
    lept_free(data);

        /* Test header reading (general jpeg functions) */
    pixReadHeader(fname, &format1, &w1, &h1, &bps1, &spp1, NULL);
    pixWriteMem(&data, &size, pixs, IFF_JFIF_JPEG);
    pixReadHeaderMem(data, size, &format2, &w2, &h2, &bps2, &spp2, NULL);
    regTestCompareValues(rp, format1, format2, 0.0);
    regTestCompareValues(rp, w1, w2, 0.0);
    regTestCompareValues(rp, h1, h2, 0.0);
    regTestCompareValues(rp, bps1, bps2, 0.0);
    regTestCompareValues(rp, bps1, 8, 0.0);
    regTestCompareValues(rp, spp1, spp2, 0.0);
    lept_stderr("w = %d, h = %d, bps = %d, spp = %d, format = %d\n",
                w1, h1, bps1, spp1, format1);

    pixDestroy(&pixs);
    lept_free(data);
    return;
}

void DoJpegTest4(L_REGPARAMS  *rp,
                 const char   *fname)
{
char     buf[256];
char     comment1[256];
l_uint8 *comment2;
l_int32  xres, yres;
FILE    *fp;
PIX     *pixs;

        /* Test special comment and resolution readers */
    pixs = pixRead(fname);
    snprintf(comment1, sizeof(comment1), "Test %d", rp->index + 1);
    pixSetText(pixs, comment1);
    pixSetResolution(pixs, 137, 137);
    snprintf(buf, sizeof(buf), "/tmp/lept/regout/jpegio.%d.jpg", rp->index + 1);
    pixWrite(buf, pixs, IFF_JFIF_JPEG);
    regTestCheckFile(rp, buf);
    fp = lept_fopen(buf, "rb");
    fgetJpegResolution(fp, &xres, &yres);
    fgetJpegComment(fp, &comment2);
    if (!comment2)
        comment2 = (l_uint8 *)stringNew("");
    lept_fclose(fp);
    regTestCompareValues(rp, xres, 137, 0.0);
    regTestCompareValues(rp, yres, 137, 0.0);
    regTestCompareStrings(rp, (l_uint8 *)comment1, strlen(comment1),
                          comment2, strlen((char *)comment2));
    lept_stderr("xres = %d, yres = %d, comment = %s\n", xres, yres, comment1);

    lept_free(comment2);
    pixDestroy(&pixs);
    return;
}


