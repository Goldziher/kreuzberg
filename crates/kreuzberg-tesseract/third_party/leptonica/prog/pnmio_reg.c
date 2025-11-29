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
 * pnmio_reg.c
 *
 *   Tests read and write of both ascii and packed pnm, using
 *   pix with 1, 2, 4, 8 and 32 bpp.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

l_int32 main(l_int32  argc,
             char   **argv)
{
l_uint8      *data;
size_t        size;
FILE         *fp;
PIX          *pix1, *pix2, *pix3, *pix4, *pix5;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

#if !USE_PNMIO
    lept_stderr("pnm/pam writing is not enabled\n"
                "See environ.h: #define USE_PNMIO 1\n\n");

    regTestCleanup(rp);
    return 0;
#endif  /* abort */

    lept_rmdir("lept/pnm");
    lept_mkdir("lept/pnm");

        /* Test 1 bpp (pbm) read/write */
    pix1 = pixRead("char.tif");
    fp = lept_fopen("/tmp/lept/pnm/pix1.1.pnm", "wb");
    pixWriteStreamAsciiPnm(fp, pix1);
    lept_fclose(fp);
    pix2 = pixRead("/tmp/lept/pnm/pix1.1.pnm");
    pixWrite("/tmp/lept/pnm/pix2.1.pnm", pix2, IFF_PNM);
    pix3 = pixRead("/tmp/lept/pnm/pix2.1.pnm");
    regTestComparePix(rp, pix1, pix3);  /* 0 */
        /* write PAM */
    fp = lept_fopen("/tmp/lept/pnm/pix3.1.pnm", "wb");
    pixWriteStreamPam(fp, pix1);
    lept_fclose(fp);
    pix4 = pixRead("/tmp/lept/pnm/pix3.1.pnm");
    regTestComparePix(rp, pix1, pix4);  /* 1 */
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);

        /* Test 2, 4 and 8 bpp (pgm) read/write */
    pix1 = pixRead("weasel8.png");
    pix2 = pixThresholdTo2bpp(pix1, 4, 0);
    fp = lept_fopen("/tmp/lept/pnm/pix2.2.pnm", "wb");
    pixWriteStreamAsciiPnm(fp, pix2);
    lept_fclose(fp);
    pix3 = pixRead("/tmp/lept/pnm/pix2.2.pnm");
    pixWrite("/tmp/lept/pnm/pix3.2.pnm", pix3, IFF_PNM);
    pix4 = pixRead("/tmp/lept/pnm/pix3.2.pnm");
    regTestComparePix(rp, pix2, pix4);  /* 2 */
        /* write PAM */
    fp = lept_fopen("/tmp/lept/pnm/pix4.2.pnm", "wb");
    pixWriteStreamPam(fp, pix2);
    lept_fclose(fp);
    pix5 = pixRead("/tmp/lept/pnm/pix4.2.pnm");
    regTestComparePix(rp, pix2, pix5);  /* 3 */
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);
    pixDestroy(&pix5);

    pix2 = pixThresholdTo4bpp(pix1, 16, 0);
    fp = lept_fopen("/tmp/lept/pnm/pix2.4.pnm", "wb");
    pixWriteStreamAsciiPnm(fp, pix2);
    lept_fclose(fp);
    pix3 = pixRead("/tmp/lept/pnm/pix2.4.pnm");
    pixWrite("/tmp/lept/pnm/pix3.4.pnm", pix3, IFF_PNM);
    pix4 = pixRead("/tmp/lept/pnm/pix3.4.pnm");
    regTestComparePix(rp, pix2, pix4);  /* 4 */
        /* write PAM */
    fp = lept_fopen("/tmp/lept/pnm/pix4.4.pnm", "wb");
    pixWriteStreamPam(fp, pix2);
    lept_fclose(fp);
    pix5 = pixRead("/tmp/lept/pnm/pix4.4.pnm");
    regTestComparePix(rp, pix2, pix5);  /* 5 */
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);
    pixDestroy(&pix5);

    fp = lept_fopen("/tmp/lept/pnm/pix1.8.pnm", "wb");
    pixWriteStreamAsciiPnm(fp, pix1);
    lept_fclose(fp);
    pix2 = pixRead("/tmp/lept/pnm/pix1.8.pnm");
    pixWrite("/tmp/lept/pnm/pix2.8.pnm", pix2, IFF_PNM);
    pix3 = pixRead("/tmp/lept/pnm/pix2.8.pnm");
    regTestComparePix(rp, pix1, pix3);  /* 6 */
        /* write PAM */
    fp = lept_fopen("/tmp/lept/pnm/pix3.8.pnm", "wb");
    pixWriteStreamPam(fp, pix1);
    lept_fclose(fp);
    pix4 = pixRead("/tmp/lept/pnm/pix3.8.pnm");
    regTestComparePix(rp, pix1, pix4);  /* 7 */
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);

        /* Test ppm (24 bpp rgb) read/write */
    pix1 = pixRead("marge.jpg");
    fp = lept_fopen("/tmp/lept/pnm/pix1.24.pnm", "wb");
        /* write ascii */
    pixWriteStreamAsciiPnm(fp, pix1);
    lept_fclose(fp);
    pix2 = pixRead("/tmp/lept/pnm/pix1.24.pnm");
        /* write pnm */
    pixWrite("/tmp/lept/pnm/pix2.24.pnm", pix2, IFF_PNM);
    pix3 = pixRead("/tmp/lept/pnm/pix2.24.pnm");
    regTestComparePix(rp, pix1, pix3);  /* 8 */
    pixDestroy(&pix3);
        /* write mem pnm */
    pixWriteMemPnm(&data, &size, pix1);
    pix3 = pixReadMemPnm(data, size);
    regTestComparePix(rp, pix1, pix3);  /* 9 */
    lept_free(data);
        /* write pam */
    fp = lept_fopen("/tmp/lept/pnm/pix3.24.pnm", "wb");
    pixWriteStreamPam(fp, pix1);
    lept_fclose(fp);
    pix4 = pixRead("/tmp/lept/pnm/pix3.24.pnm");
    regTestComparePix(rp, pix1, pix4);  /* 10 */
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);

        /* Test pam (32 bpp rgba) read/write */
    pix1 = pixRead("test32-alpha.png");
    fp = lept_fopen("/tmp/lept/pnm/pix1.32.pnm", "wb");
    pixWriteStreamPam(fp, pix1);
    lept_fclose(fp);
    pix2 = pixRead("/tmp/lept/pnm/pix1.32.pnm");
    pixWrite("/tmp/lept/pnm/pix2.32.pnm", pix2, IFF_PNM);
    pix3 = pixRead("/tmp/lept/pnm/pix2.32.pnm");
    regTestComparePix(rp, pix1, pix3);  /* 11 */
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);

    return regTestCleanup(rp);
}
