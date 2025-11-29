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
 *   rasteropip_reg.c
 *
 *   Tests in-place operation using the general 2-image pixRasterop().
 *   The in-place operation works because there is no overlap
 *   between the src and dest rectangles.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
         char **argv)
{
l_int32       i, j;
PIX          *pixs, *pixt, *pixd;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    pixs = pixRead("test8.jpg");
    pixt = pixCopy(NULL, pixs);

        /* Copy, in-place and one COLUMN at a time, from the right
           side to the left side. */
    for (j = 0; j < 200; j++)
        pixRasterop(pixs, 20 + j, 20, 1, 250, PIX_SRC, pixs, 250 + j, 20);
    pixDisplayWithTitle(pixs, 50, 50, "in-place copy", rp->display);

        /* Copy, in-place and one ROW at a time, from the right
           side to the left side. */
    for (i = 0; i < 250; i++)
        pixRasterop(pixt, 20, 20 + i, 200, 1, PIX_SRC, pixt, 250, 20 + i);

        /* Test */
    regTestComparePix(rp, pixs, pixt);   /* 0 */
    pixDestroy(&pixs);
    pixDestroy(&pixt);

        /* Show the mirrored border, which uses the general
           pixRasterop() on an image in-place.  */
    pixs = pixRead("test8.jpg");
    pixt = pixRemoveBorder(pixs, 40);
    pixd = pixAddMirroredBorder(pixt, 40, 40, 40, 40);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);  /* 1 */
    pixDisplayWithTitle(pixd, 650, 50, "mirrored border", rp->display);
    pixDestroy(&pixs);
    pixDestroy(&pixt);
    pixDestroy(&pixd);
    return regTestCleanup(rp);
}
