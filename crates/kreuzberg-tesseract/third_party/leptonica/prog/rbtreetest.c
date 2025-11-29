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
 * Modified from the excellent code here:
 *     http://en.literateprograms.org/Red-black_tree_(C)?oldid=19567
 * which has been placed in the public domain under the Creative Commons
 * CC0 1.0 waiver (http://creativecommons.org/publicdomain/zero/1.0/).
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

#define PRINT_FULL_TREE  0
#define TRACE            0

l_int32 main(int    argc,
             char **argv)
{
l_int32    i;
RB_TYPE    x, y;
RB_TYPE   *pval;
L_RBTREE  *t;

    setLeptDebugOK(1);

    t = l_rbtreeCreate(L_INT_TYPE);
    l_rbtreePrint(stderr, t);

        /* Build the tree */
    for (i = 0; i < 5000; i++) {
        x.itype = rand() % 10000;
        y.itype = rand() % 10000;
#if TRACE
        l_rbtreePrint(stderr, t);
        printf("Inserting %d -> %d\n\n", x.itype, y.itype);
#endif  /* TRACE */
        l_rbtreeInsert(t, x, y);
        pval = l_rbtreeLookup(t, x);
        if (pval->itype != y.itype)
            L_ERROR("val %lld doesn't agree for key %lld\n", "main",
                    pval->itype, x.itype);
    }

        /* Count the nodes in the tree */
    lept_stderr("count = %d\n", l_rbtreeGetCount(t));

#if PRINT_FULL_TREE
    l_rbtreePrint(stderr, t);  /* very big */
#endif  /* PRINT_FULL_TREE */

        /* Destroy the tree and count the remaining nodes */
    l_rbtreeDestroy(&t);
    l_rbtreePrint(stderr, t);  /* should give an error message */
    lept_stderr("count = %d\n", l_rbtreeGetCount(t));

        /* Build another tree */
    t = l_rbtreeCreate(L_INT_TYPE);
    for (i = 0; i < 6000; i++) {
        x.itype = rand() % 10000;
        y.itype = rand() % 10000;
        l_rbtreeInsert(t, x, y);
    }

        /* Count the nodes in the tree */
    lept_stderr("count = %d\n", l_rbtreeGetCount(t));

        /* Delete lots of nodes randomly from the tree and recount.
         * Deleting 80,000 random points gets them all; deleting
         * 60,000 removes all but 7 points. */
    for (i = 0; i < 60000; i++) {
        x.itype = rand() % 10000;
#if TRACE
        l_rbtreePrint(stderr, t);
        printf("Deleting key %d\n\n", x.itype);
#endif  /* TRACE */
        l_rbtreeDelete(t, x);
    }
    lept_stderr("count = %d\n", l_rbtreeGetCount(t));
    l_rbtreePrint(stderr, t);
    lept_free(t);

    return 0;
}

