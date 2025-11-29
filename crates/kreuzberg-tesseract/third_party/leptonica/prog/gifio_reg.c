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
 *   gifio_reg.c
 *
 *    !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
 *    This is the Leptonica regression test for lossless read/write
 *    I/O in gif format.
 *    !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
 *
 *    This tests reading and writing of images in gif format for
 *    varioius depths.
 *
 *    The relative times for writing of gif and png are interesting.
 *
 *     For 1 bpp:
 *
 *        png writing is about 2x faster than gif writing, using giflib.
 *
 *     For 32 bpp, using a 1 Mpix rgb image:
 *
 *       png:  Lossless: 1.16 sec (2.0 MB output file)
 *             Lossy: 0.43 sec, composed of:
 *                       0.22 sec (octree quant with dithering)
 *                       0.21 sec (to compress and write out)
 *
 *       gif:  Lossy: 0.34 sec, composed of:
 *                       0.22 sec (octree quant with dithering)
 *                       0.12 sec (to compress and write out)
 *             (note: no lossless mode; gif can't write out rgb)
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include <math.h>
#include "allheaders.h"

#if HAVE_LIBGIF || HAVE_LIBUNGIF
#include "gif_lib.h"
#endif  /* HAVE_LIBGIF || HAVE_LIBUNGIF */

#define   FILE_1BPP     "feyn.tif"
#define   FILE_2BPP     "weasel2.4g.png"
#define   FILE_4BPP     "weasel4.16c.png"
#define   FILE_8BPP_1   "dreyfus8.png"
#define   FILE_8BPP_2   "weasel8.240c.png"
#define   FILE_8BPP_3   "test8.jpg"
#define   FILE_16BPP    "test16.tif"
#define   FILE_32BPP    "marge.jpg"

static void test_gif(const char *fname, PIXA *pixa, L_REGPARAMS *rp);
static l_int32 test_mem_gif(const char *fname, l_int32 index);

int main(int    argc,
         char **argv)
{
char          buf[64];
l_int32       success;
PIX          *pix;
PIXA         *pixa;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

#if !HAVE_LIBGIF && !HAVE_LIBUNGIF
    lept_stderr("gifio is not enabled\n"
                "libgif or libungif are required for gifio_reg\n"
                "See environ.h: #define HAVE_LIBGIF or HAVE_LIBUNGIF 1\n"
                "See prog/Makefile: link in -lgif or -lungif\n\n");
    regTestCleanup(rp);
    return 0;
#endif  /* abort */

        /* 5.1+ and not 5.1.2 */
    snprintf(buf, sizeof(buf), "%s_reg", rp->testname);
#if (GIFLIB_MAJOR < 5 || (GIFLIB_MAJOR == 5 && GIFLIB_MINOR == 0))
    L_ERROR("Require giflib-5.1 or later.\n", buf);
    return 1;
#endif  /* < 5.1 */
#if GIFLIB_MAJOR == 5 && GIFLIB_MINOR == 1 && GIFLIB_RELEASE == 2  /* 5.1.2 */
    L_ERROR("Can't use giflib-5.1.2; suggest 5.1.3 or later.\n", buf);
    return 1;
#endif  /* 5.1.2 */

        /* Set up for display output */
    pixa = (rp->display) ? pixaCreate(0) : NULL;

    lept_rmdir("lept/gif");
    lept_mkdir("lept/gif");

    /* ------------ Part 1: Test lossless r/w to file ------------*/
    test_gif(FILE_1BPP, pixa, rp);
    test_gif(FILE_2BPP, pixa, rp);
    test_gif(FILE_4BPP, pixa, rp);
    test_gif(FILE_8BPP_1, pixa, rp);
    test_gif(FILE_8BPP_2, pixa, rp);
    test_gif(FILE_8BPP_3, pixa, rp);
    test_gif(FILE_16BPP, pixa, rp);
    test_gif(FILE_32BPP, pixa, rp);
    if (rp->success)
        lept_stderr("\n  ****** Success on lossless r/w to file *****\n\n");
    else
        lept_stderr("\n  ***** Failure on at least one r/w to file ****\n\n");

    if (rp->display) {
        pix = pixaDisplayTiledAndScaled(pixa, 32, 450, 3, 0, 20, 2);
        pixWrite("/tmp/lept/gif/giftest.jpg", pix, IFF_JFIF_JPEG);
        pixDisplay(pix, 100, 100);
        pixDestroy(&pix);
        pixaDestroy(&pixa);
    }

    /* ------------ Part 2: Test lossless r/w to memory ------------ */
    success = TRUE;
    if (test_mem_gif(FILE_1BPP, 0)) success = FALSE;
    if (test_mem_gif(FILE_2BPP, 1)) success = FALSE;
    if (test_mem_gif(FILE_4BPP, 2)) success = FALSE;
    if (test_mem_gif(FILE_8BPP_1, 3)) success = FALSE;
    if (test_mem_gif(FILE_8BPP_2, 4)) success = FALSE;
    if (test_mem_gif(FILE_8BPP_3, 5)) success = FALSE;
    if (test_mem_gif(FILE_16BPP, 6)) success = FALSE;
    if (test_mem_gif(FILE_32BPP, 7)) success = FALSE;
    if (success)
        lept_stderr("\n  ****** Success on lossless r/w to memory *****\n\n");
    else
        lept_stderr("\n  **** Failure on at least one r/w to memory ****\n\n");

        /* Success only if all tests are passed */
    if (rp->success == TRUE) rp->success = success;

    return regTestCleanup(rp);
}


static void
test_gif(const char   *fname,
         PIXA         *pixa,
         L_REGPARAMS  *rp)
{
char     buf[256];
l_int32  same;
PIX     *pixs, *pix1, *pix2;

    pixs = pixRead(fname);
    snprintf(buf, sizeof(buf), "/tmp/lept/gif/gifio-a.%d.gif", rp->index + 1);
    pixWrite(buf, pixs, IFF_GIF);
    pix1 = pixRead(buf);
    snprintf(buf, sizeof(buf), "/tmp/lept/gif/gifio-b.%d.gif", rp->index + 1);
    pixWrite(buf, pix1, IFF_GIF);
    pix2 = pixRead(buf);
    regTestWritePixAndCheck(rp, pix2, IFF_GIF);
    pixEqual(pixs, pix2, &same);

    if (!same && rp->index < 6) {
        lept_stderr("Error for %s\n", fname);
        rp->success = FALSE;
    }
    if (rp->display) {
        lept_stderr(" depth: pixs = %d, pix1 = %d\n", pixGetDepth(pixs),
                    pixGetDepth(pix1));
        pixaAddPix(pixa, pix2, L_CLONE);
    }
    pixDestroy(&pixs);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    return;
}


    /* Returns 1 on error */
static l_int32
test_mem_gif(const char  *fname,
             l_int32      index)
{
l_uint8  *data = NULL;
l_int32   same;
size_t    size = 0;
PIX      *pixs;
PIX      *pixd = NULL;

    if ((pixs = pixRead(fname)) == NULL) {
        lept_stderr("Failure to read gif file: %s\n", fname);
        return 1;
    }
    if (pixWriteMem(&data, &size, pixs, IFF_GIF)) {
        lept_stderr("Mem gif write fail on image %d\n", index);
        return 1;
    }
    if ((pixd = pixReadMem(data, size)) == NULL) {
        lept_stderr("Mem gif read fail on image %d\n", index);
        lept_free(data);
        return 1;
    }

    pixEqual(pixs, pixd, &same);
    pixDestroy(&pixs);
    pixDestroy(&pixd);
    lept_free(data);
    if (!same && index < 6) {
        lept_stderr("Mem gif write/read fail for file %s\n", fname);
        return 1;
    }
    else
        return 0;
}
