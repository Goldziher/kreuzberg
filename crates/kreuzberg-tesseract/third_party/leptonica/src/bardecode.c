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

/*!
 * \file bardecode.c
 * <pre>
 *
 *      Dispatcher
 *          char            *barcodeDispatchDecoder()
 *
 *      Format Determination
 *          static l_int32   barcodeFindFormat()
 *          l_int32          barcodeFormatIsSupported()
 *          static l_int32   barcodeVerifyFormat()
 *
 *      Decode 2 of 5
 *          static char     *barcodeDecode2of5()
 *
 *      Decode Interleaved 2 of 5
 *          static char     *barcodeDecodeI2of5()
 *
 *      Decode Code 93
 *          static char     *barcodeDecode93()
 *
 *      Decode Code 39
 *          static char     *barcodeDecode39()
 *
 *      Decode Codabar
 *          static char     *barcodeDecodeCodabar()
 *
 *      Decode UPC-A
 *          static char     *barcodeDecodeUpca()
 *
 *      Decode EAN 13
 *          static char     *barcodeDecodeEan13()
 * </pre>
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include <string.h>
#include "allheaders.h"
#include "readbarcode.h"

static l_int32 barcodeFindFormat(char *barstr);
static l_int32 barcodeVerifyFormat(char *barstr, l_int32 format,
                                   l_int32 *pvalid, l_int32 *preverse);
static char *barcodeDecode2of5(char *barstr, l_int32 debugflag);
static char *barcodeDecodeI2of5(char *barstr, l_int32 debugflag);
static char *barcodeDecode93(char *barstr, l_int32 debugflag);
static char *barcodeDecode39(char *barstr, l_int32 debugflag);
static char *barcodeDecodeCodabar(char *barstr, l_int32 debugflag);
static char *barcodeDecodeUpca(char *barstr, l_int32 debugflag);
static char *barcodeDecodeEan13(char *barstr, l_int32 first, l_int32 debugflag);

#ifndef  NO_CONSOLE_IO
#define  DEBUG_CODES       0
#endif  /* ~NO_CONSOLE_IO */

/*------------------------------------------------------------------------*
 *                           Decoding dispatcher                          *
 *------------------------------------------------------------------------*/
/*!
 * \brief   barcodeDispatchDecoder()
 *
 * \param[in]    barstr      string of integers in set {1,2,3,4} of bar widths
 * \param[in]    format      L_BF_ANY, L_BF_CODEI2OF5, L_BF_CODE93, ...
 * \param[in]    debugflag   use 1 to generate debug output
 * \return  data string of decoded barcode data, or NULL on error
 */
char *
barcodeDispatchDecoder(char    *barstr,
                       l_int32  format,
                       l_int32  debugflag)
{
char  *data = NULL;

    if (!barstr)
        return (char *)ERROR_PTR("barstr not defined", __func__, NULL);

    debugflag = FALSE;  /* not used yet */

    if (format == L_BF_ANY)
        format = barcodeFindFormat(barstr);

    if (format == L_BF_CODE2OF5)
        data = barcodeDecode2of5(barstr, debugflag);
    else if (format == L_BF_CODEI2OF5)
        data = barcodeDecodeI2of5(barstr, debugflag);
    else if (format == L_BF_CODE93)
        data = barcodeDecode93(barstr, debugflag);
    else if (format == L_BF_CODE39)
        data = barcodeDecode39(barstr, debugflag);
    else if (format == L_BF_CODABAR)
        data = barcodeDecodeCodabar(barstr, debugflag);
    else if (format == L_BF_UPCA)
        data = barcodeDecodeUpca(barstr, debugflag);
    else if (format == L_BF_EAN13)
        data = barcodeDecodeEan13(barstr, 0, debugflag);
    else
        return (char *)ERROR_PTR("format not implemented", __func__, NULL);

    return data;
}


/*------------------------------------------------------------------------*
 *                      Barcode format determination                      *
 *------------------------------------------------------------------------*/
/*!
 * \brief   barcodeFindFormat()
 *
 * \param[in]    barstr    of barcode widths, in set {1,2,3,4}
 * \return  format for barcode, or L_BF_UNKNOWN if not recognized
 */
static l_int32
barcodeFindFormat(char    *barstr)
{
l_int32  i, format, valid;

   if (!barstr)
       return ERROR_INT("barstr not defined", __func__, L_BF_UNKNOWN);

   for (i = 0; i < NumSupportedBarcodeFormats; i++) {
       format = SupportedBarcodeFormat[i];
       barcodeVerifyFormat(barstr, format, &valid, NULL);
       if (valid) {
           L_INFO("Barcode format: %s\n", __func__,
                   SupportedBarcodeFormatName[i]);
           return format;
       }
   }
   return L_BF_UNKNOWN;
}


/*!
 * \brief   barcodeFormatIsSupported()
 *
 * \param[in]    format
 * \return  1 if format is one of those supported; 0 otherwise
 *
 */
l_int32
barcodeFormatIsSupported(l_int32  format)
{
l_int32  i;

   for (i = 0; i < NumSupportedBarcodeFormats; i++) {
       if (format == SupportedBarcodeFormat[i])
           return 1;
   }
   return 0;
}


/*!
 * \brief   barcodeVerifyFormat()
 *
 * \param[in]    barstr     of barcode widths, in set {1,2,3,4}
 * \param[in]    format     L_BF_CODEI2OF5, L_BF_CODE93, ...
 * \param[out]   pvalid     0 if not valid, 1 and 2 if valid
 * \param[out]   preverse   [optional] 1 if reversed; 0 otherwise
 * \return  0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *      (1) If valid == 1, the barcode is of the given format in the
 *          forward order; if valid == 2, it is backwards.
 *      (2) If the barcode needs to be reversed to read it, and &reverse
 *          is provided, a 1 is put into %reverse.
 *      (3) Require at least 12 data bits, in addition to format identifiers.
 *          (TODO) If the barcode has a fixed length, this should be used
 *          explicitly, as is done for L_BF_UPCA and L_BF_EAN13.
 *      (4) (TODO) Add to this as more formats are supported.
 * </pre>
 */
static l_int32
barcodeVerifyFormat(char     *barstr,
                    l_int32   format,
                    l_int32  *pvalid,
                    l_int32  *preverse)
{
char    *revbarstr;
l_int32  i, start, len, stop, mid;

    if (!pvalid)
        return ERROR_INT("barstr not defined", __func__, 1);
    *pvalid = 0;
    if (preverse) *preverse = 0;
    if (!barstr)
        return ERROR_INT("barstr not defined", __func__, 1);

    switch (format)
    {
    case L_BF_CODE2OF5:
        start = !strncmp(barstr, Code2of5[C25_START], 3);
        len = strlen(barstr);
        if (len < 20)
            return ERROR_INT("barstr too short for CODE2OF5", __func__, 1);
        stop = !strncmp(&barstr[len - 5], Code2of5[C25_STOP], 5);
        if (start && stop) {
            *pvalid = 1;
        } else {
            revbarstr = stringReverse(barstr);
            start = !strncmp(revbarstr, Code2of5[C25_START], 3);
            stop = !strncmp(&revbarstr[len - 5], Code2of5[C25_STOP], 5);
            LEPT_FREE(revbarstr);
            if (start && stop) {
                *pvalid = 1;
                if (preverse) *preverse = 1;
            }
        }
        break;
    case L_BF_CODEI2OF5:
        start = !strncmp(barstr, CodeI2of5[CI25_START], 4);
        len = strlen(barstr);
        if (len < 20)
            return ERROR_INT("barstr too short for CODEI2OF5", __func__, 1);
        stop = !strncmp(&barstr[len - 3], CodeI2of5[CI25_STOP], 3);
        if (start && stop) {
            *pvalid = 1;
        } else {
            revbarstr = stringReverse(barstr);
            start = !strncmp(revbarstr, CodeI2of5[CI25_START], 4);
            stop = !strncmp(&revbarstr[len - 3], CodeI2of5[CI25_STOP], 3);
            LEPT_FREE(revbarstr);
            if (start && stop) {
                *pvalid = 1;
                if (preverse) *preverse = 1;
            }
        }
        break;
    case L_BF_CODE93:
        start = !strncmp(barstr, Code93[C93_START], 6);
        len = strlen(barstr);
        if (len < 28)
            return ERROR_INT("barstr too short for CODE93", __func__, 1);
        stop = !strncmp(&barstr[len - 7], Code93[C93_STOP], 6);
        if (start && stop) {
            *pvalid = 1;
        } else {
            revbarstr = stringReverse(barstr);
            start = !strncmp(revbarstr, Code93[C93_START], 6);
            stop = !strncmp(&revbarstr[len - 7], Code93[C93_STOP], 6);
            LEPT_FREE(revbarstr);
            if (start && stop) {
                *pvalid = 1;
                if (preverse) *preverse = 1;
            }
        }
        break;
    case L_BF_CODE39:
        start = !strncmp(barstr, Code39[C39_START], 9);
        len = strlen(barstr);
        if (len < 30)
            return ERROR_INT("barstr too short for CODE39", __func__, 1);
        stop = !strncmp(&barstr[len - 9], Code39[C39_STOP], 9);
        if (start && stop) {
            *pvalid = 1;
        } else {
            revbarstr = stringReverse(barstr);
            start = !strncmp(revbarstr, Code39[C39_START], 9);
            stop = !strncmp(&revbarstr[len - 9], Code39[C39_STOP], 9);
            LEPT_FREE(revbarstr);
            if (start && stop) {
                *pvalid = 1;
                if (preverse) *preverse = 1;
            }
        }
        break;
    case L_BF_CODABAR:
        start = stop = 0;
        len = strlen(barstr);
        if (len < 26)
            return ERROR_INT("barstr too short for CODABAR", __func__, 1);
        for (i = 16; i <= 19; i++)  /* any of these will do */
            start += !strncmp(barstr, Codabar[i], 7);
        for (i = 16; i <= 19; i++)  /* ditto */
            stop += !strncmp(&barstr[len - 7], Codabar[i], 7);
        if (start && stop) {
            *pvalid = 1;
        } else {
            start = stop = 0;
            revbarstr = stringReverse(barstr);
            for (i = 16; i <= 19; i++)
                start += !strncmp(revbarstr, Codabar[i], 7);
            for (i = 16; i <= 19; i++)
                stop += !strncmp(&revbarstr[len - 7], Codabar[i], 7);
            LEPT_FREE(revbarstr);
            if (start && stop) {
                *pvalid = 1;
                if (preverse) *preverse = 1;
            }
        }
        break;
    case L_BF_UPCA:
    case L_BF_EAN13:
        len = strlen(barstr);
        if (len != 59)
            return ERROR_INT("invalid length for UPCA or EAN13", __func__, 1);
        start = !strncmp(barstr, Upca[UPCA_START], 3);
        mid = !strncmp(&barstr[27], Upca[UPCA_MID], 5);
        stop = !strncmp(&barstr[len - 3], Upca[UPCA_STOP], 3);
        if (start && mid && stop)
            *pvalid = 1;
        break;
    default:
        return ERROR_INT("format not supported", __func__, 1);
    }

    return 0;
}


/*------------------------------------------------------------------------*
 *                             Code 2 of 5                                *
 *------------------------------------------------------------------------*/
/*!
 * \brief   barcodeDecode2of5()
 *
 * \param[in]    barstr     of widths, in set {1, 2}
 * \param[in]    debugflag
 * \return  data string of digits, or NULL if none found or on error
 *
 * <pre>
 * Notes:
 *      (1) Ref: http://en.wikipedia.org/wiki/Two-out-of-five_code (Note:
 *                 the codes given here are wrong!)
 *               http://morovia.com/education/symbology/code25.asp
 *      (2) This is a very low density encoding for the 10 digits.
 *          Each digit is encoded with 5 black bars, of which 2 are wide
 *          and 3 are narrow.  No information is carried in the spaces
 *          between the bars, which are all equal in width, represented by
 *          a "1" in our encoding.
 *      (3) The mapping from the sequence of five bar widths to the
 *          digit is identical to the mapping used by the interleaved
 *          2 of 5 code.  The start code is 21211, representing two
 *          wide bars and a narrow bar, and the interleaved "1" spaces
 *          are explicit.  The stop code is 21112.  For all codes
 *          (including start and stop), the trailing space "1" is
 *          implicit -- there is no reason to represent it in the
 *          Code2of5[] array.
 * </pre>
 */
static char *
barcodeDecode2of5(char    *barstr,
                  l_int32  debugflag)
{
char    *data, *vbarstr;
char     code[10];
l_int32  valid, reverse, i, j, len, error, ndigits, start, found;

    if (!barstr)
        return (char *)ERROR_PTR("barstr not defined", __func__, NULL);

        /* Verify format; reverse if necessary */
    barcodeVerifyFormat(barstr, L_BF_CODE2OF5, &valid, &reverse);
    if (!valid)
        return (char *)ERROR_PTR("barstr not in 2of5 format", __func__, NULL);
    if (reverse)
        vbarstr = stringReverse(barstr);
    else
        vbarstr = stringNew(barstr);

        /* Verify size */
    len = strlen(vbarstr);
    if ((len - 11) % 10 != 0) {
        LEPT_FREE(vbarstr);
        return (char *)ERROR_PTR("size not divisible by 10: invalid 2of5 code",
                                 __func__, NULL);
    }

    error = FALSE;
    ndigits = (len - 11) / 10;
    data = (char *)LEPT_CALLOC(ndigits + 1, sizeof(char));
    memset(code, 0, 10);
    for (i = 0; i < ndigits; i++) {
        start = 6 + 10 * i;
        for (j = 0; j < 9; j++)
            code[j] = vbarstr[start + j];

        if (debugflag)
            lept_stderr("code: %s\n", code);

        found = FALSE;
        for (j = 0; j < 10; j++) {
            if (!strcmp(code, Code2of5[j])) {
                data[i] = 0x30 + j;
                found = TRUE;
                break;
            }
        }
        if (!found) error = TRUE;
    }
    LEPT_FREE(vbarstr);

    if (error) {
        LEPT_FREE(data);
        return (char *)ERROR_PTR("error in decoding", __func__, NULL);
    }

    return data;
}


/*------------------------------------------------------------------------*
 *                       Interleaved Code 2 of 5                          *
 *------------------------------------------------------------------------*/
/*!
 * \brief   barcodeDecodeI2of5()
 *
 * \param[in]    barstr     of widths, in set {1, 2}
 * \param[in]    debugflag
 * \return  data string of digits, or NULL if none found or on error
 *
 * <pre>
 * Notes:
 *      (1) Ref: http://en.wikipedia.org/wiki/Interleaved_2_of_5
 *      (2) This always encodes an even number of digits.
 *          The start code is 1111; the stop code is 211.
 * </pre>
 */
static char *
barcodeDecodeI2of5(char    *barstr,
                   l_int32  debugflag)
{
char    *data, *vbarstr;
char     code1[6], code2[6];
l_int32  valid, reverse, i, j, len, error, npairs, start, found;

    if (!barstr)
        return (char *)ERROR_PTR("barstr not defined", __func__, NULL);

        /* Verify format; reverse if necessary */
    barcodeVerifyFormat(barstr, L_BF_CODEI2OF5, &valid, &reverse);
    if (!valid)
        return (char *)ERROR_PTR("barstr not in i2of5 format", __func__, NULL);
    if (reverse)
        vbarstr = stringReverse(barstr);
    else
        vbarstr = stringNew(barstr);

        /* Verify size */
    len = strlen(vbarstr);
    if ((len - 7) % 10 != 0) {
        LEPT_FREE(vbarstr);
        return (char *)ERROR_PTR("size not divisible by 10: invalid I2of5 code",
                                 __func__, NULL);
    }

    error = FALSE;
    npairs = (len - 7) / 10;
    data = (char *)LEPT_CALLOC(2 * npairs + 1, sizeof(char));
    memset(code1, 0, 6);
    memset(code2, 0, 6);
    for (i = 0; i < npairs; i++) {
        start = 4 + 10 * i;
        for (j = 0; j < 5; j++) {
            code1[j] = vbarstr[start + 2 * j];
            code2[j] = vbarstr[start + 2 * j + 1];
        }

        if (debugflag)
            lept_stderr("code1: %s, code2: %s\n", code1, code2);

        found = FALSE;
        for (j = 0; j < 10; j++) {
            if (!strcmp(code1, CodeI2of5[j])) {
                data[2 * i] = 0x30 + j;
                found = TRUE;
                break;
            }
        }
        if (!found) error = TRUE;
        found = FALSE;
        for (j = 0; j < 10; j++) {
            if (!strcmp(code2, CodeI2of5[j])) {
                data[2 * i + 1] = 0x30 + j;
                found = TRUE;
                break;
            }
        }
        if (!found) error = TRUE;
    }
    LEPT_FREE(vbarstr);

    if (error) {
        LEPT_FREE(data);
        return (char *)ERROR_PTR("error in decoding", __func__, NULL);
    }

    return data;
}


/*------------------------------------------------------------------------*
 *                                 Code 93                                *
 *------------------------------------------------------------------------*/
/*!
 * \brief   barcodeDecode93()
 *
 * \param[in]    barstr     of widths, in set {1, 2, 3, 4}
 * \param[in]    debugflag
 * \return  data string of digits, or NULL if none found or on error
 *
 * <pre>
 * Notes:
 *      (1) Ref:  http://en.wikipedia.org/wiki/Code93
 *                http://morovia.com/education/symbology/code93.asp
 *      (2) Each symbol has 3 black and 3 white bars.
 *          The start and stop codes are 111141; the stop code then is
 *          terminated with a final (1) bar.
 *      (3) The last two codes are check codes.  We are checking them
 *          for correctness, and issuing a warning on failure.  Should
 *          probably not return any data on failure.
 * </pre>
 */
static char *
barcodeDecode93(char    *barstr,
                l_int32  debugflag)
{
const char  *checkc, *checkk;
char        *data, *vbarstr;
char         code[7];
l_int32      valid, reverse, i, j, len, error, nsymb, start, found, sum;
l_int32     *index;

    if (!barstr)
        return (char *)ERROR_PTR("barstr not defined", __func__, NULL);

        /* Verify format; reverse if necessary */
    barcodeVerifyFormat(barstr, L_BF_CODE93, &valid, &reverse);
    if (!valid)
        return (char *)ERROR_PTR("barstr not in code93 format", __func__, NULL);
    if (reverse)
        vbarstr = stringReverse(barstr);
    else
        vbarstr = stringNew(barstr);

        /* Verify size; skip the first 6 and last 7 bars. */
    len = strlen(vbarstr);
    if ((len - 13) % 6 != 0) {
        LEPT_FREE(vbarstr);
        return (char *)ERROR_PTR("size not divisible by 6: invalid code 93",
                                 __func__, NULL);
    }

        /* Decode the symbols */
    nsymb = (len - 13) / 6;
    data = (char *)LEPT_CALLOC(nsymb + 1, sizeof(char));
    index = (l_int32 *)LEPT_CALLOC(nsymb, sizeof(l_int32));
    memset(code, 0, 7);
    error = FALSE;
    for (i = 0; i < nsymb; i++) {
        start = 6 + 6 * i;
        for (j = 0; j < 6; j++)
            code[j] = vbarstr[start + j];

        if (debugflag)
            lept_stderr("code: %s\n", code);

        found = FALSE;
        for (j = 0; j < C93_START; j++) {
            if (!strcmp(code, Code93[j])) {
                data[i] = Code93Val[j];
                index[i] = j;
                found = TRUE;
                break;
            }
        }
        if (!found) error = TRUE;
    }
    LEPT_FREE(vbarstr);

    if (error) {
        LEPT_FREE(index);
        LEPT_FREE(data);
        return (char *)ERROR_PTR("error in decoding", __func__, NULL);
    }

        /* Do check sums.  For character "C", use only the
         * actual data in computing the sum.  For character "K",
         * use the actual data plus the check character "C". */
    sum = 0;
    for (i = 0; i < nsymb - 2; i++)  /* skip the "C" and "K" */
        sum += ((i % 20) + 1) * index[nsymb - 3 - i];
    if (data[nsymb - 2] != Code93Val[sum % 47])
        L_WARNING("Error for check C\n", __func__);

    if (debugflag) {
        checkc = Code93[sum % 47];
        lept_stderr("checkc = %s\n", checkc);
    }

    sum = 0;
    for (i = 0; i < nsymb - 1; i++)  /* skip the "K" */
        sum += ((i % 15) + 1) * index[nsymb - 2 - i];
    if (data[nsymb - 1] != Code93Val[sum % 47])
        L_WARNING("Error for check K\n", __func__);

    if (debugflag) {
        checkk = Code93[sum % 47];
        lept_stderr("checkk = %s\n", checkk);
    }

        /* Remove the two check codes from the output */
    data[nsymb - 2] = '\0';

    LEPT_FREE(index);
    return data;
}


/*------------------------------------------------------------------------*
 *                                 Code 39                                *
 *------------------------------------------------------------------------*/
/*!
 * \brief   barcodeDecode39()
 *
 * \param[in]    barstr     of widths, in set {1, 2}
 * \param[in]    debugflag
 * \return  data string of digits, or NULL if none found or on error
 *
 * <pre>
 * Notes:
 *      (1) Ref:  http://en.wikipedia.org/wiki/Code39
 *                http://morovia.com/education/symbology/code39.asp
 *      (2) Each symbol has 5 black and 4 white bars.
 *          The start and stop codes are 121121211 (the asterisk)
 *      (3) This decoder was contributed by Roger Hyde.
 * </pre>
 */
static char *
barcodeDecode39(char    *barstr,
                l_int32  debugflag)
{
char     *data, *vbarstr;
char      code[10];
l_int32   valid, reverse, i, j, len, error, nsymb, start, found;

    if (!barstr)
        return (char *)ERROR_PTR("barstr not defined", __func__, NULL);

        /* Verify format; reverse if necessary */
    barcodeVerifyFormat(barstr, L_BF_CODE39, &valid, &reverse);
    if (!valid)
        return (char *)ERROR_PTR("barstr not in code39 format", __func__, NULL);
    if (reverse)
        vbarstr = stringReverse(barstr);
    else
        vbarstr = stringNew(barstr);

        /* Verify size */
    len = strlen(vbarstr);
    if ((len + 1) % 10 != 0) {
        LEPT_FREE(vbarstr);
        return (char *)ERROR_PTR("size+1 not divisible by 10: invalid code 39",
                                 __func__, NULL);
    }

        /* Decode the symbols */
    nsymb = (len - 19) / 10;
    data = (char *)LEPT_CALLOC(nsymb + 1, sizeof(char));
    memset(code, 0, 10);
    error = FALSE;
    for (i = 0; i < nsymb; i++) {
        start = 10 + 10 * i;
        for (j = 0; j < 9; j++)
            code[j] = vbarstr[start + j];

        if (debugflag)
            lept_stderr("code: %s\n", code);

        found = FALSE;
        for (j = 0; j < C39_START; j++) {
            if (!strcmp(code, Code39[j])) {
                data[i] = Code39Val[j];
                found = TRUE;
                break;
            }
        }
        if (!found) error = TRUE;
    }
    LEPT_FREE(vbarstr);

    if (error) {
        LEPT_FREE(data);
        return (char *)ERROR_PTR("error in decoding", __func__, NULL);
    }

    return data;
}


/*------------------------------------------------------------------------*
 *                                 Codabar                                *
 *------------------------------------------------------------------------*/
/*!
 * \brief   barcodeDecodeCodabar()
 *
 * \param[in]    barstr     of widths, in set {1, 2}
 * \param[in]    debugflag
 * \return  data string of digits, or NULL if none found or on error
 *
 * <pre>
 * Notes:
 *      (1) Ref:  http://en.wikipedia.org/wiki/Codabar
 *                http://morovia.com/education/symbology/codabar.asp
 *      (2) Each symbol has 4 black and 3 white bars.  They represent the
 *          10 digits, and optionally 6 other characters.  The start and
 *          stop codes can be any of four (typically denoted A,B,C,D).
 * </pre>
 */
static char *
barcodeDecodeCodabar(char    *barstr,
                     l_int32  debugflag)
{
char     *data, *vbarstr;
char      code[8];
l_int32   valid, reverse, i, j, len, error, nsymb, start, found;

    if (!barstr)
        return (char *)ERROR_PTR("barstr not defined", __func__, NULL);

        /* Verify format; reverse if necessary */
    barcodeVerifyFormat(barstr, L_BF_CODABAR, &valid, &reverse);
    if (!valid)
        return (char *)ERROR_PTR("barstr not in codabar format",
                                 __func__, NULL);
    if (reverse)
        vbarstr = stringReverse(barstr);
    else
        vbarstr = stringNew(barstr);

        /* Verify size */
    len = strlen(vbarstr);
    if ((len + 1) % 8 != 0) {
        LEPT_FREE(vbarstr);
        return (char *)ERROR_PTR("size+1 not divisible by 8: invalid codabar",
                                 __func__, NULL);
    }

        /* Decode the symbols */
    nsymb = (len - 15) / 8;
    data = (char *)LEPT_CALLOC(nsymb + 1, sizeof(char));
    memset(code, 0, 8);
    error = FALSE;
    for (i = 0; i < nsymb; i++) {
        start = 8 + 8 * i;
        for (j = 0; j < 7; j++)
            code[j] = vbarstr[start + j];

        if (debugflag)
            lept_stderr("code: %s\n", code);

        found = FALSE;
        for (j = 0; j < 16; j++) {
            if (!strcmp(code, Codabar[j])) {
                data[i] = CodabarVal[j];
                found = TRUE;
                break;
            }
        }
        if (!found) error = TRUE;
    }
    LEPT_FREE(vbarstr);

    if (error) {
        LEPT_FREE(data);
        return (char *)ERROR_PTR("error in decoding", __func__, NULL);
    }

    return data;
}


/*------------------------------------------------------------------------*
 *                               Code UPC-A                               *
 *------------------------------------------------------------------------*/
/*!
 * \brief   barcodeDecodeUpca()
 *
 * \param[in]    barstr     of widths, in set {1, 2, 3, 4}
 * \param[in]    debugflag
 * \return  data string of digits, or NULL if none found or on error
 *
 * <pre>
 * Notes:
 *      (1) Ref:  http://en.wikipedia.org/wiki/UniversalProductCode
 *                http://morovia.com/education/symbology/upc-a.asp
 *      (2) Each symbol has 2 black and 2 white bars, and encodes a digit.
 *          The start and stop codes are 111 and 111.  There are a total of
 *          30 black bars, encoding 12 digits in two sets of 6, with
 *          2 black bars separating the sets.
 *      (3) The last digit is a check digit.  We check for correctness, and
 *          issue a warning on failure.  Should probably not return any
 *          data on failure.
 * </pre>
 */
static char *
barcodeDecodeUpca(char    *barstr,
                  l_int32  debugflag)
{
char     *data, *vbarstr;
char      code[5];
l_int32   valid, i, j, len, error, start, found, sum, checkdigit;

    if (!barstr)
        return (char *)ERROR_PTR("barstr not defined", __func__, NULL);

        /* Verify format; reverse has no meaning here -- we must test both */
    barcodeVerifyFormat(barstr, L_BF_UPCA, &valid, NULL);
    if (!valid)
        return (char *)ERROR_PTR("barstr not in UPC-A format", __func__, NULL);

        /* Verify size */
    len = strlen(barstr);
    if (len != 59)
        return (char *)ERROR_PTR("size not 59; invalid UPC-A barcode",
                                 __func__, NULL);

        /* Check the first digit.  If invalid, reverse the string. */
    memset(code, 0, 5);
    for (i = 0; i < 4; i++)
        code[i] = barstr[i + 3];
    found = FALSE;
    for (i = 0; i < 10; i++) {
        if (!strcmp(code, Upca[i])) {
            found = TRUE;
            break;
        }
    }
    if (found == FALSE)
        vbarstr = stringReverse(barstr);
    else
        vbarstr = stringNew(barstr);

        /* Decode the 12 symbols */
    data = (char *)LEPT_CALLOC(13, sizeof(char));
    memset(code, 0, 5);
    error = FALSE;
    for (i = 0; i < 12; i++) {
        if (i < 6)
            start = 3 + 4 * i;
        else
            start = 32 + 4 * (i - 6);
        for (j = 0; j < 4; j++)
            code[j] = vbarstr[start + j];

        if (debugflag)
            lept_stderr("code: %s\n", code);

        found = FALSE;
        for (j = 0; j < 10; j++) {
            if (!strcmp(code, Upca[j])) {
                data[i] = 0x30 + j;
                found = TRUE;
                break;
            }
        }
        if (!found) error = TRUE;
    }
    LEPT_FREE(vbarstr);

    if (error) {
        LEPT_FREE(data);
        return (char *)ERROR_PTR("error in decoding", __func__, NULL);
    }

        /* Calculate the check digit (data[11]). */
    sum = 0;
    for (i = 0; i < 12; i += 2)  /* "even" digits */
        sum += 3 * (data[i] - 0x30);
    for (i = 1; i < 11; i += 2)  /* "odd" digits */
        sum += (data[i] - 0x30);
    checkdigit = sum % 10;
    if (checkdigit)  /* not 0 */
        checkdigit = 10 - checkdigit;
    if (checkdigit + 0x30 != data[11])
        L_WARNING("Error for UPC-A check character\n", __func__);

    return data;
}


/*------------------------------------------------------------------------*
 *                               Code EAN-13                              *
 *------------------------------------------------------------------------*/
/*!
 * \brief   barcodeDecodeEan13()
 *
 * \param[in]    barstr     of widths, in set {1, 2, 3, 4}
 * \param[in]    first      first digit: 0 - 9
 * \param[in]    debugflag
 * \return  data string of digits, or NULL if none found or on error
 *
 * <pre>
 * Notes:
 *      (1) Ref:  http://en.wikipedia.org/wiki/UniversalProductCode
 *                http://morovia.com/education/symbology/ean-13.asp
 *      (2) The encoding is essentially the same as UPC-A, except
 *          there are 13 digits in total, of which 12 are encoded
 *          by bars (as with UPC-A) and the 13th is a leading digit
 *          that determines the encoding of the next 6 digits,
 *          selecting each digit from one of two tables.
 *          encoded in the bars (as with UPC-A).  If the first digit
 *          is 0, the encoding is identical to UPC-A.
 *      (3) As with UPC-A, the last digit is a check digit.
 *      (4) For now, we assume the first digit is input to this function.
 *          Eventually, we will read it by pattern matching.
 *
 *    TODO: fix this for multiple tables, depending on the value of %first
 * </pre>
 */
static char *
barcodeDecodeEan13(char    *barstr,
                   l_int32  first,
                   l_int32  debugflag)
{
char     *data, *vbarstr;
char      code[5];
l_int32   valid, i, j, len, error, start, found, sum, checkdigit;

    if (!barstr)
        return (char *)ERROR_PTR("barstr not defined", __func__, NULL);

        /* Verify format.  You can't tell the orientation by the start
         * and stop codes, but you can by the location of the digits.
         * Use the UPCA verifier for EAN 13 -- it is identical. */
    barcodeVerifyFormat(barstr, L_BF_UPCA, &valid, NULL);
    if (!valid)
        return (char *)ERROR_PTR("barstr not in EAN 13 format", __func__, NULL);

        /* Verify size */
    len = strlen(barstr);
    if (len != 59)
        return (char *)ERROR_PTR("size not 59; invalid EAN 13 barcode",
                                 __func__, NULL);

        /* Check the first digit.  If invalid, reverse the string. */
    memset(code, 0, 5);
    for (i = 0; i < 4; i++)
        code[i] = barstr[i + 3];
    found = FALSE;
    for (i = 0; i < 10; i++) {
        if (!strcmp(code, Upca[i])) {
            found = TRUE;
            break;
        }
    }
    if (found == FALSE)
        vbarstr = stringReverse(barstr);
    else
        vbarstr = stringNew(barstr);

        /* Decode the 12 symbols */
    data = (char *)LEPT_CALLOC(13, sizeof(char));
    memset(code, 0, 5);
    error = FALSE;
    for (i = 0; i < 12; i++) {
        if (i < 6)
            start = 3 + 4 * i;
        else
            start = 32 + 4 * (i - 6);
        for (j = 0; j < 4; j++)
            code[j] = vbarstr[start + j];

        if (debugflag)
            lept_stderr("code: %s\n", code);

        found = FALSE;
        for (j = 0; j < 10; j++) {
            if (!strcmp(code, Upca[j])) {
                data[i] = 0x30 + j;
                found = TRUE;
                break;
            }
        }
        if (!found) error = TRUE;
    }
    LEPT_FREE(vbarstr);

    if (error) {
        LEPT_FREE(data);
        return (char *)ERROR_PTR("error in decoding", __func__, NULL);
    }

        /* Calculate the check digit (data[11]). */
    sum = 0;
    for (i = 0; i < 12; i += 2)  /* "even" digits */
        sum += 3 * (data[i] - 0x30);
    for (i = 1; i < 12; i += 2)  /* "odd" digits */
        sum += (data[i] - 0x30);
    checkdigit = sum % 10;
    if (checkdigit)  /* not 0 */
        checkdigit = 10 - checkdigit;
    if (checkdigit + 0x30 != data[11])
        L_WARNING("Error for EAN-13 check character\n", __func__);

    return data;
}
