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
 * encoding_reg.c
 *
 *    Regression test for encoding/decoding of binary data
 *
 *    Ascii85 encoding/decoding Works properly with 0, 1, 2 or 3 extra
 *    bytes after the final full word.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
         char **argv)
{
size_t        nbytes1, nbytes2, nbytes3, nbytes4, nbytes5, nbytes6, fbytes;
char         *a85a, *a85c, *a85c2;
l_uint8      *bina, *bina2, *bin85c, *bin85c2;
PIX          *pix1;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    lept_mkdir("lept/encode");

        /* Test ascii85 encoding */
    bina = l_binaryRead("karen8.jpg", &fbytes);
    a85a = encodeAscii85(bina, fbytes, &nbytes1);
    bina2 = decodeAscii85(a85a, nbytes1, &nbytes2);
    regTestCompareValues(rp, fbytes, nbytes2, 0.0);  /* 0 */

    if (rp->display) {
        lept_stderr("file bytes = %zu, a85 bytes = %zu, bina2 bytes = %zu\n",
                    fbytes, nbytes1, nbytes2);
    }
    l_binaryWrite("/tmp/lept/encode/ascii85", "w", a85a, nbytes1);
    l_binaryWrite("/tmp/lept/encode/bina2", "w", bina2, nbytes2);

        /* Test the reconstructed image */
    pix1 = pixReadMem(bina2, nbytes2);
    regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 1 */
    pixDisplayWithTitle(pix1, 100, 100, NULL, rp->display);
    pixDestroy(&pix1);

        /* Test with compression, starting with ascii data */
    a85c = encodeAscii85WithComp((l_uint8 *)a85a, nbytes1, &nbytes3);
    bin85c = decodeAscii85WithComp(a85c, nbytes3, &nbytes4);
    regTestCompareStrings(rp, (l_uint8 *)a85a, nbytes1,
                          bin85c, nbytes4);  /* 2 */

        /* Test with compression, starting with binary data */
    a85c2 = encodeAscii85WithComp(bin85c, nbytes4, &nbytes5);
    bin85c2 = decodeAscii85WithComp(a85c2, nbytes5, &nbytes6);
    regTestCompareStrings(rp, bin85c, nbytes4,
                          bin85c2, nbytes6);  /* 3 */
    lept_free(bina);
    lept_free(bina2);
    lept_free(a85a);
    lept_free(a85c);
    lept_free(bin85c);
    lept_free(a85c2);
    lept_free(bin85c2);

        /* Test storing and retrieving compressed text from pix */
    bina = l_binaryRead("weasel32.png", &nbytes1);
    pix1 = pixRead("rabi.png");
    pixSetTextCompNew(pix1, bina, nbytes1);
    bina2 = pixGetTextCompNew(pix1, &nbytes2);
    if (rp->display)
        lept_stderr("nbytes1 = %zu, nbytes2 = %zu\n", nbytes1, nbytes2);
    regTestCompareStrings(rp, bina, nbytes1, bina2, nbytes2);  /* 4 */
    pixDestroy(&pix1);
    lept_free(bina);
    lept_free(bina2);

    return regTestCleanup(rp);
}
