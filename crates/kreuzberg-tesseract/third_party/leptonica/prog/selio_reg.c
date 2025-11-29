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
 * selio_reg.c
 *
 *    Runs a number of tests on reading and writing of Sels
 *
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include <string.h>
#include "allheaders.h"

static const char *textsel1 = "x  oo "
                              "x oOo "
                              "x  o  "
                              "x     "
                              "xxxxxx";
static const char *textsel2 = " oo  x"
                              " oOo x"
                              "  o  x"
                              "     x"
                              "xxxxxx";
static const char *textsel3 = "xxxxxx"
                              "x     "
                              "x  o  "
                              "x oOo "
                              "x  oo ";
static const char *textsel4 = "xxxxxx"
                              "     x"
                              "  o  x"
                              " oOo x"
                              " oo  x";
static const char *textsel5 = "xxxxxx"
                              "     x"
                              "  o  x"
                              " ooo x"
                              " oo  x";
static const char *textsel6 = "xxXxxx"
                              "     x"
                              "  o  x"
                              " oOo x"
                              " oo  x";


int main(int    argc,
         char **argv)
{
l_float32     val;
PIX          *pix;
SEL          *sel;
SELA         *sela1, *sela2;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

        /* selaRead() / selaWrite()  */
    sela1 = selaAddBasic(NULL);
    selaWrite("/tmp/lept/regout/sel.0.sela", sela1);
    regTestCheckFile(rp, "/tmp/lept/regout/sel.0.sela");  /* 0 */
    sela2 = selaRead("/tmp/lept/regout/sel.0.sela");
    selaWrite("/tmp/lept/regout/sel.1.sela", sela2);
    regTestCheckFile(rp, "/tmp/lept/regout/sel.1.sela");  /* 1 */
    regTestCompareFiles(rp, 0, 1);  /* 2 */
    selaDestroy(&sela1);
    selaDestroy(&sela2);

        /* Create from file and display result */
    sela1 = selaCreateFromFile("flipsels.txt");
    pix = selaDisplayInPix(sela1, 31, 3, 15, 4);
    regTestWritePixAndCheck(rp, pix, IFF_PNG);  /* 3 */
    pixDisplayWithTitle(pix, 100, 100, NULL, rp->display);
    selaWrite("/tmp/lept/regout/sel.3.sela", sela1);
    regTestCheckFile(rp, "/tmp/lept/regout/sel.3.sela");  /* 4 */
    pixDestroy(&pix);
    selaDestroy(&sela1);

        /* Create the same set of Sels from compiled strings and compare */
    sela2 = selaCreate(4);
    sel = selCreateFromString(textsel1, 5, 6, "textsel1");
    selaAddSel(sela2, sel, NULL, 0);
    sel = selCreateFromString(textsel2, 5, 6, "textsel2");
    selaAddSel(sela2, sel, NULL, 0);
    sel = selCreateFromString(textsel3, 5, 6, "textsel3");
    selaAddSel(sela2, sel, NULL, 0);
    sel = selCreateFromString(textsel4, 5, 6, "textsel4");
    selaAddSel(sela2, sel, NULL, 0);
    selaWrite("/tmp/lept/regout/sel.4.sela", sela2);
    regTestCheckFile(rp, "/tmp/lept/regout/sel.4.sela");  /* 5 */
    regTestCompareFiles(rp, 4, 5);  /* 6 */
    selaDestroy(&sela2);

        /* Attempt to create sels from invalid strings (0 or 2 origins) */
    lept_stderr("******************************************************\n");
    lept_stderr("* The next 2 error messages are intentional          *\n");
    sel = selCreateFromString(textsel5, 5, 6, "textsel5");
    val = (sel) ? 1.0 : 0.0;
    regTestCompareValues(rp, val, 0.0, 0.0);  /* 6 */
    sel = selCreateFromString(textsel6, 5, 6, "textsel6");
    val = (sel) ? 1.0 : 0.0;
    regTestCompareValues(rp, val, 0.0, 0.0);  /* 7 */
    lept_stderr("******************************************************\n");

    return regTestCleanup(rp);
}
