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
 * buffertest.c
 *
 *   Tests the bbuffer operations
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

#define   NBLOCKS     11

int main(int    argc,
         char **argv)
{
char       *filein, *fileout;
l_uint8    *array1, *array2, *dataout, *dataout2;
l_int32     i, blocksize, same;
size_t      nbytes, nout, nout2;
L_BBUFFER  *bb, *bb2;
FILE       *fp;

    if (argc != 3)
        return ERROR_INT(" Syntax:  buffertest filein fileout", __func__, 1);
    filein = argv[1];
    fileout = argv[2];
    setLeptDebugOK(1);

    if ((array1 = l_binaryRead(filein, &nbytes)) == NULL)
        return ERROR_INT("array not made", __func__, 1);
    lept_stderr("Bytes read from file: %lu\n", (unsigned long)nbytes);

        /* Application of byte buffer ops: compress/decompress in memory */
#if 1
    dataout = zlibCompress(array1, nbytes, &nout);
    l_binaryWrite(fileout, "w", dataout, nout);

    dataout2 = zlibUncompress(dataout, nout, &nout2);
    l_binaryWrite("/tmp/dataout2", "w", dataout2, nout2);

    filesAreIdentical(filein, "/tmp/dataout2", &same);
    if (same)
        lept_stderr("Correct: data is the same\n");
    else
        lept_stderr("Error: data is different\n");

    lept_stderr("nbytes in = %lu, nbytes comp = %lu, nbytes uncomp = %lu\n",
            (unsigned long)nbytes, (unsigned long)nout, (unsigned long)nout2);
    lept_free(dataout);
    lept_free(dataout2);
#endif

        /* Low-level byte buffer read/write test */
#if 1
    bb = bbufferCreate(array1, nbytes);
    bbufferRead(bb, array1, nbytes);

    array2 = (l_uint8 *)lept_calloc(2 * nbytes, sizeof(l_uint8));

    lept_stderr(" Bytes initially in buffer: %d\n", bb->n);

    blocksize = (2 * nbytes) / NBLOCKS;
    for (i = 0; i <= NBLOCKS; i++) {
        bbufferWrite(bb, array2, blocksize, &nout);
        lept_stderr(" block %d: wrote %lu bytes\n", i + 1,
                (unsigned long)nout);
    }

    lept_stderr(" Bytes left in buffer: %d\n", bb->n);

    bb2 = bbufferCreate(NULL, 0);
    bbufferRead(bb2, array1, nbytes);
    fp = lept_fopen(fileout, "wb");
    bbufferWriteStream(bb2, fp, nbytes, &nout);
    lept_stderr(" bytes written out to fileout: %lu\n", (unsigned long)nout);
    lept_fclose(fp);

    bbufferDestroy(&bb);
    bbufferDestroy(&bb2);
    lept_free(array2);
#endif

    lept_free(array1);
    return 0;
}
