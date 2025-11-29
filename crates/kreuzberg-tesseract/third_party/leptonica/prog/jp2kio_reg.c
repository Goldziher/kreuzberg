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
 *   jp2kio_reg.c
 *
 *    !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
 *    This is the Leptonica regression test for lossy read/write
 *    I/O in jp2k format.
 *    !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
 *
 *    This tests reading and writing of images in jp2k format.
 *
 *    * jp2k supports 8 bpp gray, rgb, and rgba.
 *    * This makes calls into the jpeg2000 library libopenjp2.
 *    * Compared to reading and writing jpeg, reading jp2k is
 *      very slow, and writing jp2k is miserably slow.
 *    * If we try to run this starting with image half the size,
 *      the library gives opj_start_compress() encoding errors!
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include <math.h>
#include "allheaders.h"

    /* Needed for HAVE_LIBJP2K */
#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif /* HAVE_CONFIG_H */

void DoJp2kTest1(L_REGPARAMS *rp, const char *fname);
void DoJp2kTest2(L_REGPARAMS *rp, const char *fname);
void DoJp2kTest3(L_REGPARAMS *rp, const char *fname);


int main(int    argc,
         char **argv)
{
L_REGPARAMS  *rp;

#if !HAVE_LIBJP2K
    lept_stderr("jp2kio is not enabled\n"
                "libopenjp2 is required for jp2kio_reg\n"
                "See environ.h: #define HAVE_LIBJP2K\n"
                "See prog/Makefile: link in -lopenjp2\n\n");
    return 0;
#endif  /* abort */

        /* This test uses libjpeg */
#if !HAVE_LIBJPEG
    lept_stderr("libjpeg is required for jp2kio_reg\n\n");
    return 0;
#endif  /* abort */

    if (regTestSetup(argc, argv, &rp))
        return 1;

    DoJp2kTest1(rp, "karen8.jpg");
    DoJp2kTest1(rp, "test24.jpg");
/*    DoJp2kTest2(rp, "karen8.jpg");  */  /* encode fails on smallest image */
    DoJp2kTest2(rp, "test24.jpg");
    DoJp2kTest3(rp, "wyom.jpg");
    return regTestCleanup(rp);
}


void DoJp2kTest1(L_REGPARAMS  *rp,
                 const char   *fname)
{
char     buf[64];
char    *name;
l_int32  w, h;
BOX     *box;
PIX     *pix0, *pix1, *pix2, *pix3;

        /* Read, write, read back and write again */
    pix0 = pixRead(fname);
    pix1 = pixScale(pix0, 0.5, 0.5);
    pixGetDimensions(pix1, &w, &h, NULL);
    regTestWritePixAndCheck(rp, pix1, IFF_JP2);  /* 0, 5 */
    name = regTestGenLocalFilename(rp, -1, IFF_JP2);
    pix2 = pixRead(name);
    regTestWritePixAndCheck(rp, pix2, IFF_JP2);  /* 1, 6 */
    pixDisplayWithTitle(pix2, 0, 100, "1", rp->display);
    pixDestroy(&pix1);
    pixDestroy(&pix2);

        /* Test cropping and scaling in the jp2 interface */
    box = boxCreate(w / 4, h / 4, w / 2, h / 2);
    pix1 = pixReadJp2k(name, 1, box, 0, 0);  /* read cropped to the box */
    snprintf(buf, sizeof(buf), "/tmp/lept/regout/jp2kio.%02d.jp2",
             rp->index + 1);
    pixWriteJp2k(buf, pix1, 38, 0, 0, 0);  /* write cropped to the box */
    regTestCheckFile(rp, buf);  /* 2, 7 */
    pix2 = pixRead(buf);  /* read the cropped image */
    regTestWritePixAndCheck(rp, pix2, IFF_JP2);  /* 3, 8 */
    pixDisplayWithTitle(pix2, 500, 100, "2", rp->display);
    pix3 = pixReadJp2k(buf, 2, NULL, 0, 0);  /* read cropped image at 2x red */
    regTestWritePixAndCheck(rp, pix3, IFF_JP2);  /* 4, 9 */
    pixDisplayWithTitle(pix3, 1000, 100, "3", rp->display);
    pixDestroy(&pix0);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    boxDestroy(&box);
    lept_free(name);
    return;
}

void DoJp2kTest2(L_REGPARAMS  *rp,
                 const char   *fname)
{
char      buf[256];
char     *name;
l_uint8  *data;
l_int32   w, h;
size_t    nbytes;
BOX      *box;
PIX      *pix0, *pix1, *pix2, *pix3;

        /* Test the memory interface */
    pix0 = pixRead(fname);
    pix1 = pixScale(pix0, 0.5, 0.5);
    pixGetDimensions(pix1, &w, &h, NULL);
    regTestWritePixAndCheck(rp, pix1, IFF_JP2);  /* 10 */
    name = regTestGenLocalFilename(rp, -1, IFF_JP2);
    pix2 = pixRead(name);
    regTestWritePixAndCheck(rp, pix2, IFF_JP2);  /* 11 */
    data = l_binaryRead(name, &nbytes);
    pix3 = pixReadMemJp2k(data, nbytes, 1, NULL, 0, 0);
    regTestWritePixAndCheck(rp, pix3, IFF_JP2);  /* 12 */
    pixDisplayWithTitle(pix3, 0, 100, "1", rp->display);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    lept_free(data);

        /* Test scaling on read with the memory interface */
    box = boxCreate(w / 3, h / 3, w / 3, h / 3);
    pix1 = pixReadJp2k(name, 1, box, 0, 0);  /* just read the box region */
    snprintf(buf, sizeof(buf), "/tmp/lept/regout/jp2kio.%02d.jp2",
             rp->index + 1);
    pixWriteJp2k(buf, pix1, 38, 0, 0, 0);  /* write cropped to the box */
    regTestCheckFile(rp, buf);  /* 13 */
    data = l_binaryRead(buf, &nbytes);
    pix2 = pixReadMemJp2k(data, nbytes, 1, NULL, 0, 0);  /* read it again */
    regTestWritePixAndCheck(rp, pix2, IFF_JP2);  /* 14 */
    pixDisplayWithTitle(pix2, 500, 100, "2", rp->display);
    pix3 = pixReadMemJp2k(data, nbytes, 2, NULL, 0, 0);  /* read at 2x red */
    regTestWritePixAndCheck(rp, pix3, IFF_JP2);  /* 15 */
    pixDisplayWithTitle(pix3, 1000, 100, "3", rp->display);
    boxDestroy(&box);
    pixDestroy(&pix0);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    lept_free(data);
    lept_free(name);
    return;
}

void DoJp2kTest3(L_REGPARAMS  *rp,
                 const char   *fname)
{
FILE  *fp;
PIX   *pix0, *pix1;

        /* Test write and read using J2K codec */
    lept_mkdir("lept/jp2k");
    pix0 = pixRead(fname);
    if ((fp = lept_fopen("/tmp/lept/jp2k/wyom.j2k", "wb+")) != NULL) {
        pixWriteStreamJp2k(fp, pix0, 34, 6, L_J2K_CODEC, 0, 0);
        lept_fclose(fp);
    }
    pix1 = pixRead("/tmp/lept/jp2k/wyom.j2k");
    regTestCompareSimilarPix(rp, pix0, pix1, 20, 0.01, 0);  /* 16 */
    pixDisplayWithTitle(pix1, 500, 500, NULL, rp->display);
    pixDestroy(&pix0);
    pixDestroy(&pix1);
}
