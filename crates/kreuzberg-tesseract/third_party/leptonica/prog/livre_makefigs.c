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
 * livre_makefigs.c
 *
 *   This makes all the figures in Chapter 18, "Document Image Applications",
 *   of the book "Mathematical morphology: from theory to applications",
 *   edited by Laurent Najman and hugues Talbot.  Published by Hermes
 *   Scientific Publishing, Ltd, 2010.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
         char **argv)
{
char  buf[256];

    if (argc != 1)
        return ERROR_INT(" Syntax:  livre_makefigs", __func__, 1);

    setLeptDebugOK(1);
    lept_mkdir("lept/livre");

        /* Generate Figure 1 (page segmentation) */
    callSystemDebug("livre_seedgen");
    snprintf(buf, sizeof(buf),
             "cp /tmp/lept/livre/seedgen.png /tmp/lept/livre/dia_fig1.png");
    callSystemDebug(buf);

        /* Generate Figures 2-5 (page segmentation) */
    snprintf(buf, sizeof(buf), "livre_pageseg pageseg2.tif");
    callSystemDebug(buf);
    snprintf(buf, sizeof(buf),
             "cp /tmp/lept/livre/segout.1.png /tmp/lept/livre/dia_fig2.png");
    callSystemDebug(buf);
    snprintf(buf, sizeof(buf),
             "cp /tmp/lept/livre/segout.2.png /tmp/lept/livre/dia_fig3.png");
    callSystemDebug(buf);
    snprintf(buf, sizeof(buf),
             "cp /tmp/lept/livre/segout.3.png /tmp/lept/livre/dia_fig4.png");
    callSystemDebug(buf);
    snprintf(buf, sizeof(buf),
             "cp /tmp/lept/livre/segout.4.png /tmp/lept/livre/dia_fig5.png");
    callSystemDebug(buf);

        /* Generate Figure 6 (hmt sels for text orientation) */
    callSystemDebug("livre_orient");
    snprintf(buf, sizeof(buf),
             "cp /tmp/lept/livre/orient.png /tmp/lept/livre/dia_fig6.png");
    callSystemDebug(buf);

        /* Generate Figure 7 (hmt sel for fancy "Tribune") */
    callSystemDebug("livre_hmt 1 8");
    snprintf(buf, sizeof(buf),
             "cp /tmp/lept/livre/hmt.png /tmp/lept/livre/dia_fig7.png");
    callSystemDebug(buf);

        /* Generate Figure 8 (hmt sel for fancy "T") */
    callSystemDebug("livre_hmt 2 4");
    snprintf(buf, sizeof(buf),
             "cp /tmp/lept/livre/hmt.png /tmp/lept/livre/dia_fig8.png");
    callSystemDebug(buf);

        /* Generate Figure 9 (tophat background cleaning) */
    callSystemDebug("livre_tophat");
    snprintf(buf, sizeof(buf),
             "cp /tmp/lept/livre/tophat.jpg /tmp/lept/livre/dia_fig9.jpg");
    callSystemDebug(buf);

        /* Run livre_adapt to generate an expanded version of Figure 9 */
    callSystemDebug("livre_adapt");


    return 0;
}
