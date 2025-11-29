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
 *  recog_bootnum.c
 *
 *   This does two things:
 *
 *   (1) It makes bootnum1.pa and bootnum2.pa from stored labelled data.
 *
 *   (2) Using these, as well as bootnum3.pa, it makes code for
 *       generating and compiling the the pixas, which are used by the
 *       boot digit recognizer.
 *       The output of the code generator is files such as autogen_101.*.
 *       These files have been edited to combine the .c and .h files into
 *       a single .c file:
 *          autogen_101.*  -->  src/bootnumgen1.c
 *          autogen_102.*  -->  src/bootnumgen2.c
 *          autogen_103.*  -->  src/bootnumgen3.c
 *
 *   To add another set of templates to bootnumgen1.c:
 *      (a) Add a new .pa file: prog/recog/digits/digit_setN.pa (N > 15)
 *      (b) Add code to MakeBootnum1() for this set, selecting with the
 *          string those templates you want to use.
 *      (c) Run recog_bootnum.
 *          * This makes a new /tmp/lept/recog/digits/bootnum1.pa.
 *            Replace prog/recog/digits/bootnum1.pa with this.
 *          * This makes new files: /tmp/lept/auto/autogen.101.{h,c}.
 *            The .h file is the only one we need to use.
 *            Replace the encoded string in src/bootnumgen1.c with the
 *            one in autogen.101.h, and recompile.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"
#include "bmfdata.h"

static PIXA  *MakeBootnum1(void);
static PIXA  *MakeBootnum2(void);

l_int32 main(int    argc,
             char **argv)
{
PIX          *pix1;
PIXA         *pixa1, *pixa2, *pixa3;
L_STRCODE    *strc;

    if (argc != 1) {
        lept_stderr(" Syntax: recog_bootnum\n");
        return 1;
    }

    setLeptDebugOK(1);
    lept_mkdir("lept/recog/digits");

    /* ----------------------- Bootnum 1 --------------------- */
        /* Make the bootnum pixa from the images */
    pixa1 = MakeBootnum1();
    pixaWrite("/tmp/lept/recog/digits/bootnum1.pa", pixa1);
    pix1 = pixaDisplayTiledWithText(pixa1, 1500, 1.0, 10, 2, 6, 0xff000000);
    pixDisplay(pix1, 100, 0);
    pixDestroy(&pix1);
    pixaDestroy(&pixa1);

        /* Generate the code to make the bootnum1 pixa.
         * Note: the actual code we use is in bootnumgen1.c, and
         * has already been compiled into the library. */
    strc = strcodeCreate(101);  /* arbitrary integer */
    strcodeGenerate(strc, "/tmp/lept/recog/digits/bootnum1.pa", "PIXA");
    strcodeFinalize(&strc, "/tmp/lept/auto");
    lept_free(strc);

        /* Generate the bootnum1 pixa from the generated code */
    pixa1 = l_bootnum_gen1();
    pix1 = pixaDisplayTiledWithText(pixa1, 1500, 1.0, 10, 2, 6, 0xff000000);
/*    pix1 = pixaDisplayTiled(pixa1, 1500, 0, 30); */
    pixDisplay(pix1, 100, 0);
    pixDestroy(&pix1);

        /* Extend the bootnum1 pixa by erosion */
    pixa3 = pixaExtendByMorph(pixa1, L_MORPH_ERODE, 2, NULL, 1);
    pix1 = pixaDisplayTiledWithText(pixa3, 1500, 1.0, 10, 2, 6, 0xff000000);
    pixDisplay(pix1, 100, 0);
    pixDestroy(&pix1);
    pixaDestroy(&pixa1);
    pixaDestroy(&pixa3);

    /* ----------------------- Bootnum 2 --------------------- */
        /* Read bootnum 2 */
    pixa2 = pixaRead("recog/digits/bootnum2.pa");
    pixaWrite("/tmp/lept/recog/digits/bootnum2.pa", pixa2);
    pix1 = pixaDisplayTiledWithText(pixa2, 1500, 1.0, 10, 2, 6, 0xff000000);
    pixDisplay(pix1, 100, 700);
    pixDestroy(&pix1);
    pixaDestroy(&pixa2);

        /* Generate the code to make the bootnum2 pixa.
         * Note: the actual code we use is in bootnumgen2.c. */
    strc = strcodeCreate(102);  /* another arbitrary integer */
    strcodeGenerate(strc, "/tmp/lept/recog/digits/bootnum2.pa", "PIXA");
    strcodeFinalize(&strc, "/tmp/lept/auto");
    lept_free(strc);

        /* Generate the bootnum2 pixa from the generated code */
    pixa2 = l_bootnum_gen2();
/*    pix1 = pixaDisplayTiled(pixa2, 1500, 0, 30);  */
    pix1 = pixaDisplayTiledWithText(pixa2, 1500, 1.0, 10, 2, 6, 0xff000000);
    pixDisplay(pix1, 100, 700);
    pixDestroy(&pix1);
    pixaDestroy(&pixa2);

    /* ----------------------- Bootnum 3 --------------------- */
        /* Read bootnum 3 */
    pixa1 = pixaRead("recog/digits/bootnum3.pa");
    pix1 = pixaDisplayTiledWithText(pixa1, 1500, 1.0, 10, 2, 6, 0xff000000);
    pixDisplay(pix1, 1000, 0);
    pixDestroy(&pix1);
    pixaDestroy(&pixa1);

        /* Generate the code that, when deserializes, gives you bootnum3.pa.
         * Note: the actual code we use is in bootnumgen3.c, and
         * has already been compiled into the library. */
    strc = strcodeCreate(103);  /* arbitrary integer */
    strcodeGenerate(strc, "recog/digits/bootnum3.pa", "PIXA");
    strcodeFinalize(&strc, "/tmp/lept/auto");
    lept_free(strc);

        /* Generate the bootnum3 pixa from the generated code */
    pixa1 = l_bootnum_gen3();
    pix1 = pixaDisplayTiledWithText(pixa1, 1500, 1.0, 10, 2, 6, 0xff000000);
    pixDisplay(pix1, 1000, 0);
    pixDestroy(&pix1);

        /* Extend the bootnum3 pixa twice by erosion */
    pixa3 = pixaExtendByMorph(pixa1, L_MORPH_ERODE, 2, NULL, 1);
    pix1 = pixaDisplayTiledWithText(pixa3, 1500, 1.0, 10, 2, 6, 0xff000000);
    pixDisplay(pix1, 1000, 0);
    pixDestroy(&pix1);
    pixaDestroy(&pixa1);
    pixaDestroy(&pixa3);

#if 0
    pixa1 = l_bootnum_gen1();
/*    pixa1 = pixaRead("recog/digits/bootnum1.pa"); */
    pixaWrite("/tmp/lept/junk.pa", pixa1);
    pixa2 = pixaRead("/tmp/lept/junk.pa");
    pixaWrite("/tmp/lept/junk1.pa", pixa2);
    pixa3 = pixaRead("/tmp/lept/junk1.pa");
    n = pixaGetCount(pixa3);
    for (i = 0; i < n; i++) {
        pix = pixaGetPix(pixa3, i, L_CLONE);
        lept_stderr("i = %d, text = %s\n", i, pixGetText(pix));
        pixDestroy(&pix);
    }
    pixaDestroy(&pixa1);
    pixaDestroy(&pixa2);
    pixaDestroy(&pixa3);
#endif

    return 0;
}


PIXA *MakeBootnum1(void)
{
const char  *str;
PIXA        *pixa1, *pixa2, *pixa3;

    pixa1 = pixaRead("recog/digits/digit_set02.pa");
    str = "10, 27, 35, 45, 48, 74, 79, 97, 119, 124, 148";
    pixa3 = pixaSelectWithString(pixa1, str, NULL);
    pixaDestroy(&pixa1);

    pixa1 = pixaRead("recog/digits/digit_set03.pa");
    str = "2, 15, 30, 50, 60, 75, 95, 105, 121, 135";
    pixa2 = pixaSelectWithString(pixa1, str, NULL);
    pixaJoin(pixa3, pixa2, 0, -1);
    pixaDestroy(&pixa1);
    pixaDestroy(&pixa2);

    pixa1 = pixaRead("recog/digits/digit_set05.pa");
    str = "0, 15, 30, 49, 60, 75, 90, 105, 120, 135";
    pixa2 = pixaSelectWithString(pixa1, str, NULL);
    pixaJoin(pixa3, pixa2, 0, -1);
    pixaDestroy(&pixa1);
    pixaDestroy(&pixa2);

    pixa1 = pixaRead("recog/digits/digit_set06.pa");
    str = "4, 15, 30, 48, 60, 78, 90, 105, 120, 135";
    pixa2 = pixaSelectWithString(pixa1, str, NULL);
    pixaJoin(pixa3, pixa2, 0, -1);
    pixaDestroy(&pixa1);
    pixaDestroy(&pixa2);

    pixa1 = pixaRead("recog/digits/digit_set07.pa");
    str = "3, 15, 30, 45, 60, 77, 78, 91, 105, 120, 149";
    pixa2 = pixaSelectWithString(pixa1, str, NULL);
    pixaJoin(pixa3, pixa2, 0, -1);
    pixaDestroy(&pixa1);
    pixaDestroy(&pixa2);

    pixa1 = pixaRead("recog/digits/digit_set08.pa");
    str = "0, 20, 30, 45, 60, 75, 90, 106, 121, 135";
    pixa2 = pixaSelectWithString(pixa1, str, NULL);
    pixaJoin(pixa3, pixa2, 0, -1);
    pixaDestroy(&pixa1);
    pixaDestroy(&pixa2);

    pixa1 = pixaRead("recog/digits/digit_set09.pa");
    str = "0, 20, 32, 47, 54, 63, 75, 91, 105, 125, 136";
    pixa2 = pixaSelectWithString(pixa1, str, NULL);
    pixaJoin(pixa3, pixa2, 0, -1);
    pixaDestroy(&pixa1);
    pixaDestroy(&pixa2);

    pixa1 = pixaRead("recog/digits/digit_set11.pa");
    str = "0, 15, 36, 46, 62, 63, 76, 91, 106, 123, 135";
    pixa2 = pixaSelectWithString(pixa1, str, NULL);
    pixaJoin(pixa3, pixa2, 0, -1);
    pixaDestroy(&pixa1);
    pixaDestroy(&pixa2);

    pixa1 = pixaRead("recog/digits/digit_set12.pa");
    str = "1, 20, 31, 45, 61, 75, 95, 107, 120, 135";
    pixa2 = pixaSelectWithString(pixa1, str, NULL);
    pixaJoin(pixa3, pixa2, 0, -1);
    pixaDestroy(&pixa1);
    pixaDestroy(&pixa2);

    pixa1 = pixaRead("recog/digits/digit_set13.pa");
    str = "1, 16, 31, 48, 63, 78, 98, 105, 123, 136";
    pixa2 = pixaSelectWithString(pixa1, str, NULL);
    pixaJoin(pixa3, pixa2, 0, -1);
    pixaDestroy(&pixa1);
    pixaDestroy(&pixa2);

    pixa1 = pixaRead("recog/digits/digit_set14.pa");
    str = "1, 14, 24, 37, 53, 62, 74, 83, 98, 114";
    pixa2 = pixaSelectWithString(pixa1, str, NULL);
    pixaJoin(pixa3, pixa2, 0, -1);
    pixaDestroy(&pixa1);
    pixaDestroy(&pixa2);

    pixa1 = pixaRead("recog/digits/digit_set15.pa");
    str = "0, 1, 3, 5, 7, 8, 13, 25, 35";
    pixa2 = pixaSelectWithString(pixa1, str, NULL);
    pixaJoin(pixa3, pixa2, 0, -1);
    pixaDestroy(&pixa1);
    pixaDestroy(&pixa2);

    return pixa3;
}


PIXA *MakeBootnum2(void)
{
char     *fname;
l_int32   i, n, w, h;
BOX      *box;
PIX      *pix;
PIXA     *pixa;
L_RECOG  *recog;
SARRAY   *sa;

        /* Phase 1: generate recog from the digit data */
    recog = recogCreate(0, 40, 0, 128, 1);
    sa = getSortedPathnamesInDirectory("recog/bootnums", "png", 0, 0);
    n = sarrayGetCount(sa);
    for (i = 0; i < n; i++) {
            /* Read each pix: grayscale, multi-character, labelled */
        fname = sarrayGetString(sa, i, L_NOCOPY);
        if ((pix = pixRead(fname)) == NULL) {
            lept_stderr("Can't read %s\n", fname);
            continue;
        }

            /* Convert to a set of 1 bpp, single character, labelled */
        pixGetDimensions(pix, &w, &h, NULL);
        box = boxCreate(0, 0, w, h);
        recogTrainLabeled(recog, pix, box, NULL, 0);
        pixDestroy(&pix);
        boxDestroy(&box);
    }
    recogTrainingFinished(&recog, 1, -1, -1.0);
    sarrayDestroy(&sa);

        /* Phase 2: generate pixa consisting of 1 bpp, single character pix */
    pixa = recogExtractPixa(recog);
    pixaWrite("/tmp/lept/recog/digits/bootnum2.pa", pixa);
    recogDestroy(&recog);
    return pixa;
}


