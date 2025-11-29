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
 *   autogen.137.c
 *
 *   Automatically generated code for deserializing data from
 *   compiled strings.
 *
 *     Index    Type          Deserializer     Filename
 *     -----    ----          ------------     --------
 *     0        PIXA          pixaRead         chars-6.pa
 *     1        PIXA          pixaRead         chars-10.pa
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include <string.h>
#include "allheaders.h"
#include "autogen.137.h"

/*---------------------------------------------------------------------*/
/*                      Auto-generated deserializers                   */
/*---------------------------------------------------------------------*/
/*!
 * \brief  l_autodecode_137()
 *
 * \param[in]    index           into array of functions
 * \return       data struct     e.g., pixa, in memory
 */
void *
l_autodecode_137(l_int32 index)
{
l_uint8  *data1, *data2;
l_int32   size1;
size_t    size2;
void     *result = NULL;
l_int32   nfunc = 2;

    PROCNAME("l_autodecode_137");

    if (index < 0 || index >= nfunc) {
        L_ERROR("invalid index = %d; must be less than %d\n", procName,
                index, nfunc);
        return NULL;
    }

    lept_mkdir("lept/auto");

        /* Unencode the selected string, uncompress it, and read it */
    switch (index) {
    case 0:
        data1 = decodeBase64(l_strdata_0, strlen(l_strdata_0), &size1);
        data2 = zlibUncompress(data1, size1, &size2);
        result = (void *)pixaReadMem(data2, size2);
        lept_free(data1);
        lept_free(data2);
        break;
    case 1:
        data1 = decodeBase64(l_strdata_1, strlen(l_strdata_1), &size1);
        data2 = zlibUncompress(data1, size1, &size2);
        result = (void *)pixaReadMem(data2, size2);
        lept_free(data1);
        lept_free(data2);
        break;
    default:
        L_ERROR("invalid index", procName);
    }

    return result;
}


