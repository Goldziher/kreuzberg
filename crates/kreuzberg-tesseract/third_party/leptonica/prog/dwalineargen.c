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
 * dwalineargen.c
 *
 *   This generates the C code for the full set of linear Sels,
 *   for dilation, erosion, opening and closing, and for both
 *   horizontal and vertical operations, from length 2 to 63.
 *
 *   These are put in files:
 *        dwalinear.3.c
 *        dwalinearlow.3.c
 *
 *   Q. Why is this C code generated here in prog, and not placed
 *      in the library where it can be linked in with all programs?
 *   A. Because the two files it generates have 17K lines of code!
 *      We also make this code available here ("out of the box") so that you
 *      can build and run dwamorph1_reg and dwamorph2_reg, without
 *      first building and running dwalineargen.c
 *
 *   Q. Why do we build code for operations up to 63 in width and height?
 *   A. Atomic DWA operations work on Sels that have hits and misses
 *      that are not larger than 31 pixel positions from the origin.
 *      Thus, they can implement a horizontal closing up to 63 pixels
 *      wide if the origin is in the center.
 *
 *      Note the word "atomic".  DWA operations can be done on arbitrarily
 *      large Sels using the *ExtendDwa() functions.  See morphdwa.c
 *      for details.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
         char **argv)
{
SELA  *sela;

    if (argc != 1)
        return ERROR_INT(" Syntax:  dwalineargen", __func__, 1);
    setLeptDebugOK(1);

        /* Generate the linear sel dwa code */
    sela = selaAddDwaLinear(NULL);
    if (fmorphautogen(sela, 3, "dwalinear"))
        return 1;
    selaDestroy(&sela);
    return 0;
}

