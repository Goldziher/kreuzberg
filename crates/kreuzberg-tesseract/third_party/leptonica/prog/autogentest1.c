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
 *  autogentest1.c
 *
 *  This makes /tmp/lept/auto/autogen.137.c and /tmp/lept/auto/autogen.137.h.
 *  It shows how to use the stringcode facility.
 *
 *  In general use, you compile and run the code generator before
 *  compiling and running the generated code, in autogentest2.c.
 *
 *  But here, because we compile both autogentest1.c and autogentest2.c
 *  at the same time, it is necessary to put the generated code
 *  in this directory.  Running autogentest1 will simply regenerate
 *  this code, but in the /tmp/lept/auto/ directory.
 *
 *  As part of the test, this makes /tmp/lept/auto/autogen.138.c and
 *  /tmp/lept/auto/autogen.138.h, which contain the same data, using
 *  the function strcodeCreateFromFile().  With this method, you do not
 *  need to specify the file type (e.g., "PIXA")
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"
#include <string.h>

static const char  *files[2] = { "fonts/chars-6.pa", "fonts/chars-10.pa" };

static const char *filetext = "# testnames\n"
                              "fonts/chars-6.pa\n"
                              "fonts/chars-10.pa";

l_int32 main(int    argc,
             char **argv)
{
l_int32     i;
L_STRCODE  *strc;

    setLeptDebugOK(1);

        /* Method 1: generate autogen.137.c and autogen.137.h  */
    strc = strcodeCreate(137);
    for (i = 0; i < 2; i++)
        strcodeGenerate(strc, files[i], "PIXA");
    strcodeFinalize(&strc, NULL);

        /* Method 2: generate autogen.138.c and autogen.138.c  */
    l_binaryWrite("/tmp/lept/auto/fontnames.txt", "w", filetext,
                  strlen(filetext));
    strcodeCreateFromFile("/tmp/lept/auto/fontnames.txt", 138, NULL);
    return 0;
}

