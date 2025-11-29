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
 * sorttest.c
 *
 *   Tests sorting of connected components by various attributes,
 *   in increasing or decreasing order.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"
#include "pix_internal.h"

int main(int    argc,
         char **argv)
{
char    *filein;
l_int32  i, n, ns;
BOXA    *boxa;
PIX     *pixs, *pixt;
PIXA    *pixa, *pixas, *pixas2;

    if (argc != 2)
        return ERROR_INT(" Syntax:  sorttest filein", __func__, 1);

    filein = argv[1];
    if ((pixs = pixRead(filein)) == NULL)
        return ERROR_INT("pixs not made", __func__, 1);

#if 0
    boxa = pixConnComp(pixs, NULL, 8);
    n = boxaGetCount(boxa);

    boxas = boxaSort(boxa, L_SORT_BY_PERIMETER, L_SORT_DECREASING, NULL);
    ns = boxaGetCount(boxas);
    lept_stderr("Number of cc: n = %d, ns = %d\n", n, ns);
    boxaWrite("/tmp/junkboxa.ba", boxas);

    for (i = 0; i < n; i++) {
        box = boxaGetBox(boxas, i, L_CLONE);
        pixRenderBox(pixs, box, 2, L_FLIP_PIXELS);
        boxDestroy(&box);
    }
    pixWrite("/tmp/junkout.png", pixs, IFF_PNG);
    boxaDestroy(&boxa);
    boxaDestroy(&boxas);
#endif


#if 1
    boxa = pixConnComp(pixs, &pixa, 8);
    n = pixaGetCount(pixa);

    pixas = pixaSort(pixa, L_SORT_BY_Y, L_SORT_INCREASING, NULL, L_CLONE);
    ns = pixaGetCount(pixas);
    lept_stderr("Number of cc: n = %d, ns = %d\n", n, ns);
    pixaWrite("/tmp/pixa.pa", pixas);
    pixas2 = pixaRead("/tmp/pixa.pa");
    pixaWrite("/tmp/pixa2.pa", pixas2);

    pixt = pixaDisplayOnLattice(pixas, 100, 100, NULL, NULL);
    pixWrite("/tmp/sorted.png", pixt, IFF_PNG);
    boxaWrite("/tmp/boxa.ba", pixas->boxa);
    pixDestroy(&pixt);
    pixaDestroy(&pixa);
    pixaDestroy(&pixas);
    pixaDestroy(&pixas2);
    boxaDestroy(&boxa);
#endif

    pixDestroy(&pixs);
    return 0;
}
