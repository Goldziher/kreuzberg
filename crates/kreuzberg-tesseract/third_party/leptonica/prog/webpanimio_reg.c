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
 *    webpanimio_reg.c
 *
 *    !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
 *    This is the Leptonica regression test for animated webp
 *    !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
 *
 *    This tests writing animated webp files from a pixa of images.
 *
 *    webp supports 32 bpp rgb and rgba.
 *    Lossy writing is slow; reading is fast, comparable to reading jpeg files.
 *    Lossless writing is extremely slow.
 *
 *    Use webpinfo to inspect the contents of an animated webp file.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
         char **argv)
{
l_int32       niters, duration;
PIX          *pix1, *pix2;
PIXA         *pixa;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

#if !HAVE_LIBJPEG
    lept_stderr("libjpeg is required for webpanimio_reg\n\n");
    regTestCleanup(rp);
    return 0;
#endif  /* abort */

#if !HAVE_LIBWEBP || !HAVE_LIBWEBP_ANIM
    lept_stderr("webp and webpanim are not enabled\n"
                "See environ.h:\n"
                "    #define HAVE_LIBWEBP\n"
                "    #define HAVE_LIBWEBP_ANIM\n"
                "See prog/Makefile:\n"
                "    link in -lwebp\n"
                "    link in -lwebpmux\n\n");
    regTestCleanup(rp);
    return 0;
#endif  /* abort */

    lept_rmdir("lept/webpanim");
    lept_mkdir("lept/webpanim");

    niters = 5;
    duration = 250;   /* ms */
    pix1 = pixRead("marge.jpg");
    pix2 = pixRotate180(NULL, pix1);
    pixa = pixaCreate(6);
    pixaAddPix(pixa, pix1, L_COPY);
    pixaAddPix(pixa, pix2, L_COPY);
    pixaWriteWebPAnim("/tmp/lept/webpanim/margeanim.webp", pixa, niters,
                      duration, 80, 0);
    regTestCheckFile(rp, "/tmp/lept/webpanim/margeanim.webp");
    pixaDestroy(&pixa);
    pixDestroy(&pix1);
    pixDestroy(&pix2);

    return regTestCleanup(rp);
}
