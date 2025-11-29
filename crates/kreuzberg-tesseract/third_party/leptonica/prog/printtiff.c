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
 * printtiff.c
 *
 *   Syntax:  printtiff filein [printer]
 *
 *   Prints a multipage tiff file of 1 bpp images to a printer.
 *   If the tiff is at standard fax resolution, it expands the
 *   vertical size by a factor of two before encapsulating in
 *   ccittg4 encoded PostScript.  The PostScript file is left in /tmp,
 *   and erased (deleted, removed, unlinked) on the next invocation.
 *
 *   If the printer is not specified, this just writes the PostScript
 *   file /tmp/print_tiff.ps.
 *
 *   If your system does not have lpr, it likely has lp.  You can run
 *   printtiff to make the PostScript file, and then print with lp:
 *       lp -d <printer> /tmp/print_tiff.ps
 *       lp -d <printer> -o ColorModel=Color /tmp/print_tiff.ps
 *   etc.
 *
 *   ***************************************************************
 *   N.B.  If a printer is specified, this program invokes lpr via
 *         "system'.  It could pose a security vulnerability if used
 *         as a service in a production environment.  Consequently,
 *         this program should only be used for debug and testing.
 *   ***************************************************************
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

#define   TEMP_PS       "print_tiff.ps"   /* in the temp directory */
#define   FILL_FACTOR   0.95

int main(int    argc,
         char **argv)
{
char  *filein, *tempfile, *printer;
char   buf[512];

    if (argc != 2 && argc != 3)
        return ERROR_INT(" Syntax:  printtiff filein [printer]", __func__, 1);
    filein = argv[1];
    if (argc == 3)
        printer = argv[2];

    lept_stderr(
         "\n!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!\n"
         "   Warning: this program should only be used for testing,\n"
         "     and not in a production environment, because of a\n"
         "      potential vulnerability with the 'system' call.\n"
         "!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!\n\n");

    setLeptDebugOK(1);
    (void)lept_rm(NULL, TEMP_PS);
    tempfile = genPathname("/tmp", TEMP_PS);
    convertTiffMultipageToPS(filein, tempfile, FILL_FACTOR);

    if (argc == 3) {
        snprintf(buf, sizeof(buf), "lpr -P%s %s &", printer, tempfile);
        callSystemDebug(buf);
    }

    lept_free(tempfile);
    return 0;
}

