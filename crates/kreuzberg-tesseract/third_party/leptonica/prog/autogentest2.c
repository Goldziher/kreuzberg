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
 *  autogentest2.c
 *
 *  This is a test of the stringcode utility.
 *
 *  It uses the files compiled from autogen.137.c and autogen.137.h
 *  to regenerate each of the 2 pixa from the strings in autogen.137.h.
 *  It then writes them to file and compares with the original.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"
#include "autogen.137.h"    /* this must be included */

    /* Original serialized pixa, that were used by autogentest1.c */
static const char  *files[2] = { "fonts/chars-6.pa", "fonts/chars-10.pa" };

l_int32 main(int    argc,
             char **argv)
{
l_int32  i, same;
PIXA    *pixa;

    setLeptDebugOK(1);
    lept_mkdir("lept/auto");

    for (i = 0; i < 2; i++) {
       pixa = (PIXA *)l_autodecode_137(i);  /* this is the dispatcher */
       pixaWrite("/tmp/lept/auto/junkpa.pa", pixa);
       filesAreIdentical("/tmp/lept/auto/junkpa.pa", files[i], &same);
       if (same)
           lept_stderr("Files are the same for %s\n", files[i]);
       else
           lept_stderr("Error: files are different for %s\n", files[i]);
       pixaDestroy(&pixa);
    }

    return 0;
}

