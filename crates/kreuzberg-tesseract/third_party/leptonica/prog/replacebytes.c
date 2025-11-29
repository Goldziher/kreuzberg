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
 * replacebytes.c
 *
 *     Replaces the specified set of bytes in a file by the bytes in
 *     the input string.  The general invocation is:
 *         relacebytes <filein> <start> <nbytes> <string> <fileout>
 *     where <start> is the start location in the file to begin replacing,
 *     <nbytes> is the number of bytes to be removed from the input,
 *              beginning at the start location, and
 *     <string> is the replacement string.
 *
 *     To simply remove <nbytes> without replacing:
 *         relacebytes <filein> <start> <nbytes> <fileout>
 *
 *     One use of the general case is for replacing the date/time in a
 *     pdf file by a string of 12 '0's.  This removes the date without
 *     invalidating the byte counters:
 *        replacebytes <filein.pdf> 86 12 000000000000 <outfile.pdf>
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"
#include "string.h"

int main(int    argc,
         char **argv)
{
l_int32  start, nbytes;
char    *filein, *fileout, *newstr;

    if (argc != 5 && argc != 6)
        return ERROR_INT(
                  "syntax: replacebytes filein start nbytes [string] fileout",
                   __func__, 1);
    filein = argv[1];
    start = atof(argv[2]);
    nbytes = atof(argv[3]);
    if (argc == 5) {
        fileout = argv[4];
    } else {
        newstr = argv[4];
        fileout = argv[5];
    }

    if (argc == 5) {
        return fileReplaceBytes(filein, start, nbytes, NULL, 0, fileout);
    } else {  /* argc == 6 */
        return fileReplaceBytes(filein, start, nbytes, (l_uint8 *)newstr,
                                strlen(newstr), fileout);
    }
}

